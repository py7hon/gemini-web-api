use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub refresh_interval: u64,
    pub max_retries: u32,
    pub rate_limit_enabled: bool,
    pub rate_limit_max_requests: u32,
    pub rate_limit_window_ms: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        // Load D:\projj\gemini-api\.env
        dotenvy::dotenv().ok();

        Ok(Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "4981".to_string())
                .parse()?,

            refresh_interval: env::var("GEMINI_REFRESH_INTERVAL")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,

            max_retries: env::var("GEMINI_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()?,

            rate_limit_enabled: env::var("RATE_LIMIT_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,

            rate_limit_max_requests: env::var("RATE_LIMIT_MAX_REQUESTS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,

            rate_limit_window_ms: env::var("RATE_LIMIT_WINDOW_MS")
                .unwrap_or_else(|_| "60000".to_string())
                .parse()?,
        })
    }
}