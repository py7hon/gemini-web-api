mod config;
mod gemini;
mod handlers;
mod models;

use std::sync::Arc;
use axum::{routing::{get, post}, Router, Json};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use config::Config;
use gemini::{client::GeminiClient, cookie_pool::CookiePool};

pub struct AppState {
    pub gemini: GeminiClient,
    pub config: Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load variables from .env into the process environment (ignore error if file is absent)
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("gemini_api=debug".parse()?)
                .add_directive("tower_http=info".parse()?),
        )
        .init();

    let config = Config::from_env()?;
    let port = config.port;

    // Build cookie pool (supports 1 or many accounts, from cookies.txt or .env)
    let pool = CookiePool::load()?;
    let gemini_client = GeminiClient::new(pool, config.max_retries);

    // Eagerly refresh tokens on startup
    gemini_client
        .refresh_tokens()
        .await
        .expect("Failed to initialize — check your cookies in .env");

    let state = Arc::new(AppState {
        gemini: gemini_client,
        config,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let state_for_health = state.clone();
    let app = Router::new()
        // Health check with pool status
        .route("/health", get(move || {
            let s = state_for_health.clone();
            async move {
                let status = s.gemini.pool.status().await;
                Json(status)
            }
        }))
        // OpenAI
        .route("/openai/v1/models",             get(handlers::openai::list_models))
        .route("/openai/v1/chat/completions",   post(handlers::openai::chat_completions))
        // Claude
        .route("/anthropic/v1/messages",        post(handlers::claude::messages))
        // Gemini native
        .route("/gemini/v1beta/models/*path",   post(handlers::gemini::generate_content))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    info!("🚀 Gemini Web To API running at http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}