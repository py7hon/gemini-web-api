use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn}; 

#[derive(Debug, Clone)]
pub struct GeminiSession {
    pub psid: String,
    pub psidts: String,
    pub snlm0e: Arc<RwLock<Option<String>>>,
    pub bl: Arc<RwLock<Option<String>>>,
}

impl GeminiSession {
    pub fn new(psid: String, psidts: String) -> Self {
        Self {
            psid,
            psidts,
            snlm0e: Arc::new(RwLock::new(None)),
            bl: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn fetch_snlm0e(&self, client: &reqwest::Client) -> Result<String> {
        let cookie = format!(
            "__Secure-1PSID={}; __Secure-1PSIDTS={}",
            self.psid, self.psidts
        );

        let res = client
            .get("https://gemini.google.com/app")
            .header("Cookie", &cookie)
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
            return Err(anyhow::anyhow!(
                "Gemini app page returned HTTP {}. Your cookies may be expired.",
                status
            ));
        }

        // ── Extract SNlM0e ──
        let token = extract_snlm0e(&body).ok_or_else(|| {
            anyhow::anyhow!(
                "SNlM0e not found in Gemini page. \
                 Your __Secure-1PSID / __Secure-1PSIDTS cookies may be \
                 expired or invalid. Please refresh them from your browser."
            )
        })?;

        // ── Extract bl (build label) ──
        if let Some(bl_val) = extract_bl(&body) {
            info!("bl extracted: {}", bl_val);
            let mut lock = self.bl.write().await;
            *lock = Some(bl_val);
        } else {
            warn!("bl not found in page — will use fallback");
        }

        info!("SNlM0e token refreshed successfully");
        let mut lock = self.snlm0e.write().await;
        *lock = Some(token.clone());
        Ok(token)
    }

    pub async fn get_or_refresh_snlm0e(&self, client: &reqwest::Client) -> Result<String> {
        {
            let lock = self.snlm0e.read().await;
            if let Some(ref token) = *lock {
                return Ok(token.clone());
            }
        }
        warn!("SNlM0e not cached — fetching from Gemini app page");
        self.fetch_snlm0e(client).await
    }

    /// Returns the current bl value, or a safe fallback if not yet fetched.
    pub async fn get_bl(&self) -> String {
        let lock = self.bl.read().await;
        lock.clone()
            .unwrap_or_else(|| "boq_assistant-bard-web-server_20240101.08_p0".to_string())
    }
}

// ── Extractors ────────────────────────────────────────────────────────────────

fn extract_snlm0e(html: &str) -> Option<String> {
    // Pattern 1: SNlM0e":"<token>"
    for marker in &[r#"SNlM0e":""#, r#""SNlM0e": ""#] {
        if let Some(start) = html.find(marker) {
            let rest = &html[start + marker.len()..];
            if let Some(end) = rest.find('"') {
                let token = &rest[..end];
                if !token.is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }
    // Pattern 2: hex-escaped quotes
    if let Some(start) = html.find("SNlM0e\\x22:\\x22") {
        let rest = &html[start + "SNlM0e\\x22:\\x22".len()..];
        if let Some(end) = rest.find("\\x22") {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn extract_bl(html: &str) -> Option<String> {
    // The build label appears in several forms in the page HTML:
    // "cfb2h":"boq_assistant-bard-web-server_20240601.08_p0"
    // "bl":"boq_assistant-bard-web-server_..."
    for marker in &[r#"cfb2h":""#, r#""bl":""#] {
        if let Some(start) = html.find(marker) {
            let rest = &html[start + marker.len()..];
            if let Some(end) = rest.find('"') {
                let val = &rest[..end];
                // Must look like a build label
                if val.starts_with("boq_") {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}