use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// A single cookie credential pair with its own cached SNlM0e + bl tokens.
#[derive(Debug, Clone)]
pub struct CookieEntry {
    pub label: String,
    pub psid: String,
    pub psidts: String,
    pub snlm0e: Option<String>,
    pub bl: Option<String>,
    pub healthy: bool,
    pub fail_count: u32,
}

impl CookieEntry {
    pub fn new(psid: String, psidts: String) -> Self {
        Self::new_labeled(None, psid, psidts)
    }

    pub fn new_labeled(label: Option<String>, psid: String, psidts: String) -> Self {
        Self {
            label: label.unwrap_or_else(|| format!("{}...", &psid[..psid.len().min(8)])),
            psid,
            psidts,
            snlm0e: None,
            bl: None,
            healthy: true,
            fail_count: 0,
        }
    }

    pub fn cookie_header(&self) -> String {
        format!(
            "__Secure-1PSID={}; __Secure-1PSIDTS={}",
            self.psid, self.psidts
        )
    }
}

/// Thread-safe pool of cookie entries with round-robin + fallback rotation.
#[derive(Debug, Clone)]
pub struct CookiePool {
    entries: Arc<RwLock<Vec<CookieEntry>>>,
    current_index: Arc<RwLock<usize>>,
}

impl CookiePool {
    /// Build from a list of (psid, psidts) pairs.
    pub fn new(pairs: Vec<(String, String)>) -> Self {
        let labeled = pairs.into_iter().map(|(psid, psidts)| (None, psid, psidts)).collect();
        Self::new_labeled(labeled)
    }

    /// Build from a list of (label, psid, psidts) triples.
    pub fn new_labeled(pairs: Vec<(Option<String>, String, String)>) -> Self {
        assert!(!pairs.is_empty(), "CookiePool must have at least one cookie pair");
        let entries = pairs
            .into_iter()
            .map(|(label, psid, psidts)| CookieEntry::new_labeled(label, psid, psidts))
            .collect();
        Self {
            entries: Arc::new(RwLock::new(entries)),
            current_index: Arc::new(RwLock::new(0)),
        }
    }

    /// Parse one "account line" used by both cookies.txt and GEMINI_COOKIES.
    /// Accepted forms:
    ///   PSID:PSIDTS
    ///   label|PSID:PSIDTS
    /// Blank lines and lines starting with '#' are ignored by the caller.
    fn parse_line(line: &str) -> Option<(Option<String>, String, String)> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (label, rest) = match line.split_once('|') {
            Some((label, rest)) => (Some(label.trim().to_string()), rest),
            None => (None, line),
        };
        let mut parts = rest.trim().splitn(2, ':');
        let psid = parts.next()?.trim().to_string();
        let psidts = parts.next()?.trim().to_string();
        if psid.is_empty() || psidts.is_empty() {
            return None;
        }
        Some((label, psid, psidts))
    }

    /// Load cookie accounts from a plain text file, one account per line:
    ///   PSID:PSIDTS
    ///   my-account|PSID:PSIDTS
    /// Blank lines and lines starting with '#' are ignored (use them as comments/labels).
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read cookies file {}: {}", path, e))?;

        let pairs: Vec<(Option<String>, String, String)> =
            content.lines().filter_map(Self::parse_line).collect();

        if pairs.is_empty() {
            return Err(anyhow::anyhow!(
                "Cookies file {} exists but contains no valid PSID:PSIDTS lines",
                path
            ));
        }

        info!("Cookie pool loaded with {} account(s) from {}", pairs.len(), path);
        Ok(Self::new_labeled(pairs))
    }

    /// Parse from env: "psid1:psidts1,psid2:psidts2" (or "label|psid:psidts,...")
    pub fn from_env() -> Result<Self> {
        // Try multi-cookie pool first
        if let Ok(raw) = std::env::var("GEMINI_COOKIES") {
            let pairs: Vec<(Option<String>, String, String)> =
                raw.split(',').filter_map(Self::parse_line).collect();

            if !pairs.is_empty() {
                info!("Cookie pool loaded with {} cookie pair(s)", pairs.len());
                return Ok(Self::new_labeled(pairs));
            }
        }

        // Fallback: single pair from GEMINI_1PSID / GEMINI_1PSIDTS
        let psid = std::env::var("GEMINI_1PSID")
            .map_err(|_| anyhow::anyhow!("Neither GEMINI_COOKIES nor GEMINI_1PSID is set"))?;
        let psidts = std::env::var("GEMINI_1PSIDTS")
            .map_err(|_| anyhow::anyhow!("GEMINI_1PSIDTS is not set"))?;

        info!("Cookie pool loaded with 1 cookie pair (fallback single-pair mode)");
        Ok(Self::new(vec![(psid, psidts)]))
    }

    /// Preferred entrypoint: load accounts from GEMINI_COOKIES_FILE (default "cookies.txt")
    /// if that file exists, otherwise fall back to .env-based config (from_env).
    pub fn load() -> Result<Self> {
        let path = std::env::var("GEMINI_COOKIES_FILE").unwrap_or_else(|_| "cookies.txt".to_string());
        if std::path::Path::new(&path).exists() {
            Self::from_file(&path)
        } else {
            Self::from_env()
        }
    }

    /// Get the current active index (round-robin, skips unhealthy entries).
    pub async fn current_index(&self) -> usize {
        *self.current_index.read().await
    }

    /// Get a clone of the current active entry.
    pub async fn current(&self) -> Option<CookieEntry> {
        let entries = self.entries.read().await;
        let idx = *self.current_index.read().await;
        entries.get(idx).cloned()
    }

    /// Rotate to the next healthy cookie in the pool.
    /// Returns true if a new healthy cookie was found, false if all are unhealthy.
    pub async fn rotate(&self) -> bool {
        let entries = self.entries.read().await;
        let len = entries.len();
        let current = *self.current_index.read().await;

        // Try each entry starting from the next one
        for offset in 1..=len {
            let candidate = (current + offset) % len;
            if entries[candidate].healthy {
                drop(entries);
                let mut idx = self.current_index.write().await;
                *idx = candidate;
                info!("Cookie pool rotated to slot {}/{}", candidate + 1, len);
                return true;
            }
        }

        warn!("All {} cookie(s) in pool are marked unhealthy!", len);
        false
    }

    /// Mark the current cookie as failed. If fail_count exceeds threshold,
    /// mark it unhealthy and rotate to the next one.
pub async fn mark_failure(&self) -> bool {
    let mut entries = self.entries.write().await;
    let idx = *self.current_index.read().await;

    // ✅ Fix E0502: snapshot len before the mutable borrow via get_mut()
    let len = entries.len();

    if let Some(entry) = entries.get_mut(idx) {
        entry.fail_count += 1;
        entry.snlm0e = None;
        entry.bl = None;

        if entry.fail_count >= 3 {
            entry.healthy = false;
            warn!(
                "Cookie slot {}/{} ({}) marked UNHEALTHY after {} failures",
                idx + 1,
                len,                          // ← snapshot used here ✅
                entry.label,
                entry.fail_count,
            );
            drop(entries);
            return self.rotate().await;
        }
    }
    true
}

    /// Mark the current cookie as healthy and reset its fail counter.
    pub async fn mark_success(&self) {
        let mut entries = self.entries.write().await;
        let idx = *self.current_index.read().await;
        if let Some(entry) = entries.get_mut(idx) {
            entry.fail_count = 0;
            entry.healthy = true;
        }
    }

    /// Update cached SNlM0e + bl for the current slot.
    pub async fn update_tokens(&self, snlm0e: String, bl: Option<String>) {
        let mut entries = self.entries.write().await;
        let idx = *self.current_index.read().await;
        if let Some(entry) = entries.get_mut(idx) {
            entry.snlm0e = Some(snlm0e);
            if let Some(b) = bl {
                entry.bl = Some(b);
            }
        }
    }

    /// Returns a status summary for health checks.
    pub async fn status(&self) -> PoolStatus {
        let entries = self.entries.read().await;
        let current = *self.current_index.read().await;
        let total = entries.len();
        let healthy = entries.iter().filter(|e| e.healthy).count();

        let slots = entries
            .iter()
            .enumerate()
            .map(|(i, e)| SlotStatus {
                slot: i + 1,
                label: e.label.clone(),
                healthy: e.healthy,
                fail_count: e.fail_count,
                active: i == current,
            })
            .collect();

        PoolStatus {
            total,
            healthy,
            unhealthy: total - healthy,
            current_slot: current + 1,
            slots,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SlotStatus {
    pub slot: usize,
    pub label: String,
    pub healthy: bool,
    pub fail_count: u32,
    pub active: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct PoolStatus {
    pub total: usize,
    pub healthy: usize,
    pub unhealthy: usize,
    pub current_slot: usize,
    pub slots: Vec<SlotStatus>,
}