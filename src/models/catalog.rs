use chrono::Utc;
use crate::models::openai::ModelInfo;

/// Model tier/category for grouping in responses
#[derive(Debug, Clone)]
pub struct GeminiModel {
    pub id: &'static str,
    pub description: &'static str,
    pub owned_by: &'static str,
}

/// All currently available + latest Gemini models (as of August 2026)
/// Ordered: newest → oldest, flagship → lite
pub const ALL_MODELS: &[GeminiModel] = &[
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 🔥 Gemini 3.6 (Latest GA — July 2026)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    GeminiModel {
        id: "gemini-3.6-flash",
        description: "Latest GA model — balances speed and intelligence for agentic & multimodal tasks",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-3.6-flash-latest",
        description: "Always points to the latest Gemini 3.6 Flash release",
        owned_by: "google",
    },

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 🔥 Gemini 3.5 (GA — May 2026)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    GeminiModel {
        id: "gemini-3.5-flash",
        description: "Frontier-class performance rivaling larger models at a fraction of the cost",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-3.5-flash-latest",
        description: "Always points to the latest Gemini 3.5 Flash release",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-3.5-flash-lite",
        description: "Lightweight agentic workflows at top speeds and minimal cost",
        owned_by: "google",
    },

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 🔥 Gemini 3.1 (Preview — Feb 2026)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    GeminiModel {
        id: "gemini-3.1-pro-preview",
        description: "Strongest pure-reasoning model — premium tasks, code, and analysis",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-3.1-flash-lite",
        description: "Stable Gemini 3 efficiency option — similar pricing to 2.5 Flash-Lite with longer runway",
        owned_by: "google",
    },

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Gemini 3.0 (GA — Dec 2025)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    GeminiModel {
        id: "gemini-3-pro",
        description: "Premium Gemini 3 — frontier intelligence with Deep Research",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-3-flash",
        description: "Best for complex multimodal understanding and challenging agentic problems",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-3-pro-preview",
        description: "Preview alias for Gemini 3.1 Pro Preview",
        owned_by: "google",
    },

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Gemini 2.5 (Stable — 2025)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    GeminiModel {
        id: "gemini-2.5-pro",
        description: "Premium 2.5 variant — complex reasoning, 1M token context window",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-2.5-pro-latest",
        description: "Always points to the latest Gemini 2.5 Pro release",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-2.5-flash",
        description: "Fast, cost-effective 2.5 model — recommended stable baseline",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-2.5-flash-latest",
        description: "Always points to the latest Gemini 2.5 Flash release",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-2.5-flash-lite",
        description: "Built for massive scale — balances cost and performance for high-throughput tasks",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-2.5-flash-08-2025",
        description: "Pinned stable Gemini 2.5 Flash (August 2025 snapshot)",
        owned_by: "google",
    },

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Gemini 2.0 (Stable)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    GeminiModel {
        id: "gemini-2.0-flash",
        description: "Fast multimodal Gemini 2.0 — strong performance on everyday tasks",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-2.0-flash-latest",
        description: "Always points to the latest Gemini 2.0 Flash release",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-2.0-flash-thinking",
        description: "Gemini 2.0 Flash with extended thinking / reasoning mode",
        owned_by: "google",
    },

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Gemini 1.5 (Legacy — still available)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    GeminiModel {
        id: "gemini-1.5-pro",
        description: "Legacy Gemini 1.5 Pro — 2M token context window",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-1.5-flash",
        description: "Legacy Gemini 1.5 Flash — fast and efficient",
        owned_by: "google",
    },
    GeminiModel {
        id: "gemini-1.5-flash-8b",
        description: "Legacy Gemini 1.5 Flash 8B — smallest and fastest 1.5 variant",
        owned_by: "google",
    },
];

/// Convert catalog entry to OpenAI-compatible ModelInfo
pub fn to_model_info(m: &GeminiModel) -> ModelInfo {
    ModelInfo {
        id: m.id.to_string(),
        object: "model".to_string(),
        created: Utc::now().timestamp(),
        owned_by: m.owned_by.to_string(),
    }
}