use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::config::Config;
use crate::executor;
use crate::finetune::TrainBackend;
use crate::spine::SpineClient;
use crate::train::TrainConfig;

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
    let k8s = config.k8s.clone();
    let train_defaults = config.default_train_config();
    let state = Arc::new(AppState { config, spine });

    if jetstream_enabled {
        let url = nats_url.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::jetstream_consumer::run_compute_consumer(&url).await {
                tracing::error!(error = %e, "JetStream compute consumer stopped");
            }
        });
    }

    if k8s.enabled {
        let url = nats_url;
        tokio::spawn(async move {
            if let Err(e) = crate::k8s::operator::run_operator_loop(url, k8s, train_defaults).await
            {
                tracing::error!(error = %e, "K8s operator loop stopped");
            }
        });
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/execute", post(execute_command))
        .route("/train/validate", post(validate_train))
        .route("/train/run", post(run_train))
        .route("/k8s/status", get(k8s_status))
        .route("/k8s/sync", post(k8s_sync))
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

#[derive(serde::Deserialize)]
struct RunTrainRequest {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    data: Option<String>,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    validate_only: bool,
}

async fn run_train(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RunTrainRequest>,
) -> Json<serde_json::Value> {
    let backend = req.backend.as_deref().map(TrainBackend::parse).transpose();
    let backend = match backend {
        Ok(b) => b.unwrap_or(TrainBackend::Auto),
        Err(e) => return Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    };

    let cfg = TrainConfig {
        model: req
            .model
            .clone()
            .unwrap_or_else(|| state.config.train.model.clone()),
        data: req
            .data
            .as_ref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(&state.config.train.data)),
        output_dir: req
            .output
            .as_ref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(&state.config.train.output)),
        backend,
        validate_only: req.validate_only,
        epochs: state.config.train.epochs,
        learning_rate: state.config.train.learning_rate,
        lora_rank: state.config.train.lora_rank,
        min_entries: 1,
    };

    let event = serde_json::json!({
        "backend": cfg.backend.as_str(),
        "model": cfg.model,
        "data": cfg.data.display().to_string(),
    });

    match tokio::task::spawn_blocking({
        let k8s = state.config.k8s.clone();
        move || crate::train::run_training(&cfg, &k8s)
    })
    .await
    {
        Ok(Ok(())) => {
            let _ = state.spine.publish("muscle.train.started", &event).await;
            Json(serde_json::json!({ "ok": true }))
        }
        Ok(Err(e)) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn k8s_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match crate::k8s::operator::operator_status(&state.config.nats.url, &state.config.k8s).await {
        Ok(status) => Json(serde_json::json!({ "ok": true, "operator": status })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn k8s_sync(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let train = state.config.default_train_config();
    match crate::k8s::operator::sync_gpu_jobs(&state.config.nats.url, &state.config.k8s, &train)
        .await
    {
        Ok(status) => Json(serde_json::json!({ "ok": true, "operator": status })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}
