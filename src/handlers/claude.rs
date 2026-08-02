use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    models::claude::*,
    AppState,
};

pub async fn messages(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClaudeRequest>,
) -> impl IntoResponse {
    let prompt = build_claude_prompt(&req.messages);
    let model = req.model.clone();

    match state.gemini.generate(&prompt, &model).await {
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
        Ok(text) => {
            let id = format!("msg_{}", Uuid::new_v4().to_string().replace('-', ""));
            Json(ClaudeResponse {
                id,
                response_type: "message".to_string(),
                role: "assistant".to_string(),
                content: vec![ClaudeResponseBlock {
                    block_type: "text".to_string(),
                    text,
                }],
                model,
                stop_reason: "end_turn".to_string(),
                usage: ClaudeUsage {
                    input_tokens: 0,
                    output_tokens: 0,
                },
            })
            .into_response()
        }
    }
}

fn build_claude_prompt(messages: &[ClaudeMessage]) -> String {
    messages
        .iter()
        .map(|m| {
            let text = match &m.content {
                ClaudeContent::Text(t) => t.clone(),
                ClaudeContent::Blocks(blocks) => blocks
                    .iter()
                    .filter_map(|b| b.text.clone())
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            format!("{}: {}", m.role, text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}