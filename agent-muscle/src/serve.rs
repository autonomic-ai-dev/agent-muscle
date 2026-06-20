use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
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
    let nats_url = config.nats.url.clone();
    let jetstream_enabled = config.nats.jetstream_consumer;
    let state = Arc::new(AppState { config, spine });

    if jetstream_enabled {
        let url = nats_url.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::jetstream_consumer::run_compute_consumer(&url).await {
                tracing::error!(error = %e, "JetStream compute consumer stopped");
            }
        });
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/execute", post(execute_command))
        .route("/train/validate", post(validate_train))
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
                .publish(
                    "muscle.executed",
                    &serde_json::json!({
                        "job_id": result.job_id,
                        "command": result.command,
                        "exit_code": result.exit_code,
                        "success": result.success,
                    }),
                )
                .await;
            Json(serde_json::to_value(&result).unwrap_or_default())
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(serde::Deserialize)]
struct ValidateTrainRequest {
    data: String,
    #[serde(default = "default_min_entries")]
    min_entries: u64,
}

fn default_min_entries() -> u64 {
    1
}

async fn validate_train(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ValidateTrainRequest>,
) -> Json<serde_json::Value> {
    let path = std::path::PathBuf::from(&req.data);
    match crate::dataset::validate_dataset(&path, req.min_entries) {
        Ok(report) => {
            let subject = if report.valid {
                "muscle.train.validated"
            } else {
                "muscle.train.rejected"
            };
            let _ = state
                .spine
                .publish(
                    subject,
                    &serde_json::json!({
                        "path": report.path,
                        "entries": report.entries,
                        "valid": report.valid,
                    }),
                )
                .await;
            Json(serde_json::json!({ "ok": report.valid, "report": report }))
        }
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}
