use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct ClaudeRequest {
    pub model: String,
    pub messages: Vec<ClaudeMessage>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: ClaudeContent,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum ClaudeContent {
    Text(String),
    Blocks(Vec<ClaudeBlock>),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClaudeBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ClaudeResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ClaudeResponseBlock>,
    pub model: String,
    pub stop_reason: String,
    pub usage: ClaudeUsage,
}

#[derive(Debug, Serialize)]
pub struct ClaudeResponseBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct ClaudeUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}