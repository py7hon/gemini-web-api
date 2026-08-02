use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    Json,
};
use axum::response::sse::Event;
use chrono::Utc;
use futures::stream;
use serde_json::json;
use std::sync::Arc;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::{
    models::{
        catalog::{ALL_MODELS, to_model_info},
        openai::*,
    },
    AppState,
};

/// GET /openai/v1/models
pub async fn list_models() -> impl IntoResponse {
    let data = ALL_MODELS.iter().map(to_model_info).collect();
    Json(ModelsResponse {
        object: "list".to_string(),
        data,
    })
}

/// POST /openai/v1/chat/completions
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatCompletionRequest>,
) -> Response {
    // Validate model is known (optional — warn but still forward)
    let model_known = ALL_MODELS.iter().any(|m| m.id == req.model);
    if !model_known {
        tracing::warn!(
            "Unknown model '{}' requested — forwarding anyway",
            req.model
        );
    }

    let prompt = build_prompt(&req.messages);
    let model = req.model.clone();

    let result = state.gemini.generate(&prompt, &model).await;

    match result {
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": {"message": e.to_string(), "type": "api_error"}})),
        )
            .into_response(),

        Ok(content) if !req.stream => {
            let now = Utc::now().timestamp();
            let id = format!("chatcmpl-{}", Uuid::new_v4());
            let prompt_tokens = estimate_tokens(&prompt);
            let completion_tokens = estimate_tokens(&content);

            Json(ChatCompletionResponse {
                id,
                object: "chat.completion".to_string(),
                created: now,
                model,
                choices: vec![Choice {
                    index: 0,
                    message: ResponseMessage {
                        role: "assistant".to_string(),
                        content,
                    },
                    finish_reason: "stop".to_string(),
                }],
                usage: Usage {
                    prompt_tokens,
                    completion_tokens,
                    total_tokens: prompt_tokens + completion_tokens,
                },
            })
            .into_response()
        }

        Ok(content) => {
            let id = format!("chatcmpl-{}", Uuid::new_v4());
            let now = Utc::now().timestamp();
            let model_clone = model.clone();
            let id_clone = id.clone();

            let words: Vec<String> = content
                .split_whitespace()
                .map(|w| format!("{} ", w))
                .collect();

            // First chunk: role delta
            let first_chunk = ChatCompletionChunk {
                id: id_clone.clone(),
                object: "chat.completion.chunk".to_string(),
                created: now,
                model: model_clone.clone(),
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: DeltaMessage {
                        role: Some("assistant".to_string()),
                        content: None,
                    },
                    finish_reason: None,
                }],
            };

            let content_chunks = stream::iter(words).map({
                let id = id_clone.clone();
                let m = model_clone.clone();
                move |word| {
                    let chunk = ChatCompletionChunk {
                        id: id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: now,
                        model: m.clone(),
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: DeltaMessage {
                                role: None,
                                content: Some(word),
                            },
                            finish_reason: None,
                        }],
                    };
                    Ok::<Event, std::convert::Infallible>(
                        Event::default().data(serde_json::to_string(&chunk).unwrap()),
                    )
                }
            });

            let done_chunk = ChatCompletionChunk {
                id: id.clone(),
                object: "chat.completion.chunk".to_string(),
                created: now,
                model,
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: DeltaMessage { role: None, content: None },
                    finish_reason: Some("stop".to_string()),
                }],
            };

            let s = stream::once(async move {
                Ok::<Event, std::convert::Infallible>(
                    Event::default()
                        .data(serde_json::to_string(&first_chunk).unwrap()),
                )
            })
            .chain(content_chunks)
            .chain(stream::once(async move {
                let data = serde_json::to_string(&done_chunk).unwrap();
                Ok::<Event, std::convert::Infallible>(
                    Event::default().data(format!("{}\n\ndata: [DONE]", data)),
                )
            }));

            Sse::new(s).into_response()
        }
    }
}

fn build_prompt(messages: &[Message]) -> String {
    messages
        .iter()
        .map(|m| {
            let content = match &m.content {
                MessageContent::Text(t) => t.clone(),
                MessageContent::Parts(parts) => parts
                    .iter()
                    .filter_map(|p| p.text.clone())
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            format!("{}: {}", m.role, content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn estimate_tokens(text: &str) -> u32 {
    (text.split_whitespace().count() as f32 * 1.3) as u32
}