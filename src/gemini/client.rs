use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};

use super::cookie_pool::CookiePool;

pub const GEMINI_STREAM_URL: &str =
    "https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate";

pub struct GeminiClient {
    pub http: Client,
    pub pool: CookiePool,
    pub max_retries: u32,
}

impl GeminiClient {
    pub fn new(pool: CookiePool, max_retries: u32) -> Self {
        let http = Client::builder()
            .cookie_store(true)
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/126.0.0.0 Safari/537.36",
            )
            .build()
            .expect("Failed to build reqwest client");
        Self { http, pool, max_retries }
    }

    pub async fn refresh_tokens(&self) -> Result<(String, String)> {
        let entry = self.pool.current().await
            .ok_or_else(|| anyhow::anyhow!("No cookie available in pool"))?;

        let res = self.http
            .get("https://gemini.google.com/app")
            .header("Cookie", entry.cookie_header())
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/126.0.0.0 Safari/537.36",
            )
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await?;

        let status = res.status();
        let body = res.text().await?;

        if !status.is_success() {
            self.pool.mark_failure().await;
            return Err(anyhow::anyhow!(
                "Gemini app page returned HTTP {} — cookie may be expired",
                status
            ));
        }

        // ✅ Fix E0728: resolve the async index BEFORE the closure
        let slot = self.pool.current_index().await + 1;
        let snlm0e = extract_snlm0e(&body)
            .ok_or_else(|| anyhow::anyhow!(
                "SNlM0e not found — cookie slot {} may be expired", slot
            ))?;

        let bl = extract_bl(&body);
        let bl_val = bl.clone().unwrap_or_else(|| {
            "boq_assistant-bard-web-server_20240101.08_p0".to_string()
        });

        self.pool.update_tokens(snlm0e.clone(), bl).await;
        info!("Tokens refreshed for cookie slot {} — bl={}", slot, bl_val);

        Ok((snlm0e, bl_val))
    }

    async fn get_tokens(&self) -> Result<(String, String)> {
        let entry = self.pool.current().await
            .ok_or_else(|| anyhow::anyhow!("No cookie available"))?;

        match (entry.snlm0e, entry.bl) {
            (Some(s), Some(b)) => Ok((s, b)),
            _ => self.refresh_tokens().await,
        }
    }

    pub async fn generate(&self, prompt: &str, _model: &str) -> Result<String> {
        // ✅ Fix unused variable warning: no need for `let null = Value::Null`
        //    since we can just use Value::Null directly as a binding in json!()
        let inner = json!([[prompt], Value::Null, Value::Null]);
        let inner_str = serde_json::to_string(&inner)?;
        let f_req = json!([Value::Null, inner_str]);
        let f_req_str = serde_json::to_string(&f_req)?;

        debug!("f.req: {}", &f_req_str[..f_req_str.len().min(200)]);

        let mut attempt = 0u32;
        loop {
            attempt += 1;

            let (snlm0e, bl) = match self.get_tokens().await {
                Ok(t) => t,
                Err(e) => {
                    error!("Token fetch failed: {}", e);
                    let has_healthy = self.pool.rotate().await;
                    if !has_healthy || attempt >= self.max_retries {
                        return Err(anyhow::anyhow!("All cookies exhausted: {}", e));
                    }
                    continue;
                }
            };

            let entry = match self.pool.current().await {
                Some(e) => e,
                None => return Err(anyhow::anyhow!("No cookie available in pool")),
            };

            let slot = self.pool.current_index().await + 1;
            debug!("Attempt {}/{} using cookie slot {}", attempt, self.max_retries, slot);

            let params = [("bl", bl.as_str()), ("_reqID", "0"), ("rt", "c")];

            let res = self.http
                .post(GEMINI_STREAM_URL)
                .header("Cookie",        entry.cookie_header())
                .header("Content-Type",  "application/x-www-form-urlencoded;charset=UTF-8")
                .header("Origin",        "https://gemini.google.com")
                .header("Referer",       "https://gemini.google.com/app")
                .header("X-Same-Domain", "1")
                .query(&params)
                .form(&[("at", snlm0e.as_str()), ("f.req", f_req_str.as_str())])
                .send()
                .await;

            match res {
                Err(e) => {
                    error!("HTTP error on attempt {}: {}", attempt, e);
                    let has_healthy = self.pool.mark_failure().await;
                    if !has_healthy || attempt >= self.max_retries {
                        return Err(anyhow::anyhow!(
                            "HTTP error after {} attempts: {}", attempt, e
                        ));
                    }
                    let _ = self.refresh_tokens().await;
                }

                Ok(r) => {
                    let status = r.status();
                    let raw = r.text().await?;

                    debug!("Status: {}", status);
                    debug!("Raw (2000): {}", &raw[..raw.len().min(2000)]);

                    // 401/403 = cookie definitely expired → mark unhealthy immediately
                    if status == 401 || status == 403 {
                        warn!("Cookie slot {} returned {} — marking unhealthy", slot, status);
                        // 3 failures = unhealthy + auto-rotate
                        self.pool.mark_failure().await;
                        self.pool.mark_failure().await;
                        self.pool.mark_failure().await;
                        let _ = self.refresh_tokens().await;
                        if attempt >= self.max_retries {
                            return Err(anyhow::anyhow!(
                                "Cookie expired, all retries exhausted"
                            ));
                        }
                        continue;
                    }

                    if !status.is_success() {
                        error!("Non-2xx {} on attempt {}", status, attempt);
                        let has_healthy = self.pool.mark_failure().await;
                        if !has_healthy || attempt >= self.max_retries {
                            return Err(anyhow::anyhow!(
                                "Gemini HTTP {} after {} attempts", status, attempt
                            ));
                        }
                        let _ = self.refresh_tokens().await;
                        continue;
                    }

                    match parse_gemini_response(&raw) {
                        Some(text) => {
                            self.pool.mark_success().await;
                            debug!("Parsed {} chars", text.len());
                            return Ok(text);
                        }
                        None => {
                            warn!("Parse failed on attempt {}", attempt);
                            if attempt >= self.max_retries {
                                return Err(anyhow::anyhow!(
                                    "Failed to parse after {} attempts. Raw: {}",
                                    attempt, &raw[..raw.len().min(300)]
                                ));
                            }
                            let _ = self.refresh_tokens().await;
                        }
                    }
                }
            }
        }
    }
}

// ── Response parser ───────────────────────────────────────────────────────────

fn parse_gemini_response(raw: &str) -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with(")]}'")
            || trimmed.starts_with("//")
            || trimmed.chars().all(|c| c.is_ascii_digit())
            || !trimmed.starts_with('[')
        {
            continue;
        }

        let outer: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let inner_str = match outer
            .get(0).and_then(|v| v.get(2))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            Some(s) => s,
            None => continue,
        };

        let inner: Value = match serde_json::from_str(inner_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(t) = get_str(&inner, &[4, 0, 1, 0]) { candidates.push(t); continue; }
        if let Some(t) = inner.get(2).and_then(|v| get_str(v, &[4, 0, 1, 0])) { candidates.push(t); continue; }
        if let Some(t) = get_str(&inner, &[4, 0, 0, 0]) { candidates.push(t); continue; }
        if let Some(t) = get_str(&inner, &[0, 0])        { candidates.push(t); continue; }
        if let Some(t) = find_first_text(&inner, 0)      { candidates.push(t); }
    }

    candidates.into_iter().max_by_key(|s| s.len())
}

fn get_str(v: &Value, path: &[usize]) -> Option<String> {
    let mut cur = v;
    for &idx in path { cur = cur.get(idx)?; }
    let s = cur.as_str()?.to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn find_first_text(v: &Value, depth: u32) -> Option<String> {
    if depth > 8 { return None; }
    match v {
        Value::String(s) if s.len() > 10 => Some(s.clone()),
        Value::Array(arr) => arr.iter().find_map(|i| find_first_text(i, depth + 1)),
        _ => None,
    }
}

fn extract_snlm0e(html: &str) -> Option<String> {
    for marker in &[r#"SNlM0e":""#, r#""SNlM0e": ""#] {
        if let Some(start) = html.find(marker) {
            let rest = &html[start + marker.len()..];
            if let Some(end) = rest.find('"') {
                let t = &rest[..end];
                if !t.is_empty() { return Some(t.to_string()); }
            }
        }
    }
    if let Some(start) = html.find("SNlM0e\\x22:\\x22") {
        let rest = &html[start + "SNlM0e\\x22:\\x22".len()..];
        if let Some(end) = rest.find("\\x22") {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn extract_bl(html: &str) -> Option<String> {
    for marker in &[r#"cfb2h":""#, r#""bl":""#] {
        if let Some(start) = html.find(marker) {
            let rest = &html[start + marker.len()..];
            if let Some(end) = rest.find('"') {
                let val = &rest[..end];
                if val.starts_with("boq_") { return Some(val.to_string()); }
            }
        }
    }
    None
}