use axum::{Json, Router, extract::State, routing::{get, post}};
use std::sync::Arc;

use crate::config::Config;
use crate::executor;
use crate::spine::SpineClient;

pub struct AppState {
    pub config: Config,
    pub spine: SpineClient,
}

pub async fn start(config: Config) -> anyhow::Result<()> {
    tracing::info!("Starting agent-muscle daemon...");

    let spine = SpineClient::new(&config.spine.url, "agent-muscle", env!("CARGO_PKG_VERSION"));
    spine.register().await?;

    let spine_clone = spine.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let _ = spine_clone.heartbeat().await;
        }
    });

    let port = config.server.port;
    let state = Arc::new(AppState { config, spine });

    let app = Router::new()
        .route("/health", get(health))
        .route("/execute", post(execute_command))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("HTTP server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health(State(_): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

#[derive(serde::Deserialize)]
struct ExecuteRequest {
    command: String,
    cwd: Option<String>,
}

async fn execute_command(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteRequest>,
) -> Json<serde_json::Value> {
    let cwd = req.cwd.as_deref().map(std::path::Path::new);
    match executor::run_command(&req.command, cwd).await {
        Ok(result) => {
            let _ = state
                .spine
                .publish("muscle.executed", &serde_json::json!({
                    "job_id": result.job_id,
                    "command": result.command,
                    "exit_code": result.exit_code,
                    "success": result.success,
                }))
                .await;
            Json(serde_json::to_value(&result).unwrap_or_default())
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}
