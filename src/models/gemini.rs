use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeminiPart {
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GenerationConfig {
    pub temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: GeminiUsageMetadata,
}

#[derive(Debug, Serialize)]
pub struct GeminiCandidate {
    pub content: GeminiContent,
    #[serde(rename = "finishReason")]
    pub finish_reason: String,
    pub index: u32,
}

#[derive(Debug, Serialize)]
pub struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: u32,
}
