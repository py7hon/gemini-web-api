use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use std::sync::Arc;

use crate::{
    models::gemini::*,
    AppState,
};

pub async fn generate_content(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,  // e.g. "gemini-2.0-flash:generateContent"
    Json(req): Json<GeminiRequest>,
) -> impl IntoResponse {
    // ✅ Split on ':' to extract just the model name
    // "gemini-2.0-flash:generateContent" -> "gemini-2.0-flash"
    let model = path
        .split(':')
        .next()
        .unwrap_or(&path)
        .to_string();

    // ✅ Only allow the :generateContent action (reject unknown actions)
    let action = path.split(':').nth(1).unwrap_or("");
    if action != "generateContent" {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": format!("Unknown action: '{}'", action)})),
        )
            .into_response();
    }

    let prompt = build_gemini_prompt(&req.contents);

    match state.gemini.generate(&prompt, &model).await {
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),

        Ok(text) => {
            Json(GeminiResponse {
                candidates: vec![GeminiCandidate {
                    content: GeminiContent {
                        role: "model".to_string(),
                        parts: vec![GeminiPart { text: Some(text.clone()) }],
                    },
                    finish_reason: "STOP".to_string(),
                    index: 0,
                }],
                usage_metadata: GeminiUsageMetadata {
                    prompt_token_count: 0,
                    candidates_token_count: (text.split_whitespace().count() as f32 * 1.3) as u32,
                    total_token_count: (text.split_whitespace().count() as f32 * 1.3) as u32,
                },
            })
            .into_response()
        }
    }
}

fn build_gemini_prompt(contents: &[GeminiContent]) -> String {
    contents
        .iter()
        .flat_map(|c| c.parts.iter().filter_map(|p| p.text.clone()))
        .collect::<Vec<_>>()
        .join("\n")
}