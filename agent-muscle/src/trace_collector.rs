//! Collect workflow execution traces from NATS into SQLite for LoRA training.

use anyhow::{Context, Result};
use async_nats::jetstream::{self, consumer::AckPolicy};
use futures::StreamExt;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{error, info, warn};

/// Keep in sync with agent-spine `execution_trace::EXECUTION_COMPLETED_SUBJECT`.
pub const EXECUTION_COMPLETED_SUBJECT: &str = "autonomic.spine.execution.completed";

/// Training trace from agent-spine workflow completion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionTraceEvent {
    pub msg_id: String,
    pub execution_id: String,
    pub workflow_name: String,
    pub prompt: String,
    pub completion: String,
    pub reward: f64,
    pub success: bool,
}

pub fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("agent_muscle")
        .join("training_traces.db")
}

pub struct TraceStore {
    conn: Connection,
}

impl TraceStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path.as_ref()).context("open training_traces.db")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS traces (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                msg_id TEXT NOT NULL UNIQUE,
                execution_id TEXT NOT NULL,
                workflow_name TEXT NOT NULL,
                prompt TEXT NOT NULL,
                completion TEXT NOT NULL,
                reward REAL NOT NULL,
                success INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_traces_success ON traces(success);",
        )?;
        Ok(Self { conn })
    }

    pub fn insert(&self, trace: &ExecutionTraceEvent) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO traces
                (msg_id, execution_id, workflow_name, prompt, completion, reward, success)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                trace.msg_id,
                trace.execution_id,
                trace.workflow_name,
                trace.prompt,
                trace.completion,
                trace.reward,
                i64::from(trace.success),
            ],
        )?;
        Ok(())
    }

    pub fn count(&self) -> Result<u64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM traces", [], |row| row.get(0))?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    pub fn export_preference_pairs(&self, limit: usize) -> Result<Vec<crate::preference_pairs::PreferencePair>> {
        crate::preference_pairs::export_preference_pairs(&self.conn, limit)
    }

    pub fn export_jsonl(&self, out_dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(out_dir)?;
        let out_path = out_dir.join("traces_export.jsonl");
        let mut stmt = self.conn.prepare(
            "SELECT prompt, completion FROM traces WHERE success = 1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut file = std::fs::File::create(&out_path)?;
        use std::io::Write;
        for row in rows {
            let (instruction, response) = row?;
            let line = serde_json::json!({
                "instruction": instruction,
                "response": response,
            });
            writeln!(file, "{}", serde_json::to_string(&line)?)?;
        }
        Ok(out_path)
    }
}

async fn connect_js() -> Result<jetstream::Context> {
    let client = agent_body_core::connect_nats()
        .await
        .context("connect to NATS")?;
    let js = jetstream::new(client);
    js.get_or_create_stream(jetstream::stream::Config {
        name: agent_body_core::STREAM_NAME.to_string(),
        subjects: vec![agent_body_core::STREAM_SUBJECT_WILDCARD.to_string()],
        ..Default::default()
    })
    .await
    .context("ensure AUTONOMIC stream")?;
    js.create_consumer_on_stream(
        jetstream::consumer::pull::Config {
            durable_name: Some("muscle-trace-collector".into()),
            filter_subject: EXECUTION_COMPLETED_SUBJECT.into(),
            ack_policy: AckPolicy::Explicit,
            ack_wait: agent_body_core::default_ack_wait(),
            ..Default::default()
        },
        agent_body_core::STREAM_NAME,
    )
    .await
    .ok();
    Ok(js)
}

pub async fn run_trace_consumer(db_path: PathBuf) -> Result<()> {
    let js = connect_js().await?;
    let store = TraceStore::open(&db_path)?;
    let consumer = js
        .get_consumer_from_stream("muscle-trace-collector", agent_body_core::STREAM_NAME)
        .await
        .context("get muscle-trace-collector consumer")?;

    info!(subject = EXECUTION_COMPLETED_SUBJECT, "trace collector active");

    let mut messages = consumer
        .fetch()
        .max_messages(32)
        .expires(Duration::from_secs(30))
        .messages()
        .await
        .context("fetch trace messages")?;

    loop {
        match messages.next().await {
            Some(Ok(msg)) => {
                match serde_json::from_slice::<ExecutionTraceEvent>(&msg.payload) {
                    Ok(trace) => {
                        if let Err(e) = store.insert(&trace) {
                            warn!(error = %e, "failed to persist trace");
                        }
                    }
                    Err(e) => warn!(error = %e, "invalid trace payload"),
                }
                if let Err(e) = msg.ack().await {
                    warn!(error = %e, "trace ack failed");
                }
            }
            Some(Err(e)) => {
                error!(error = %e, "trace consumer error");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            None => {
                messages = consumer
                    .fetch()
                    .max_messages(32)
                    .expires(Duration::from_secs(30))
                    .messages()
                    .await
                    .context("refetch trace messages")?;
            }
        }
    }
}


/// Poll interval for auto-LoRA (Kernel V2 Phase 5).
pub const AUTO_LORA_POLL_INTERVAL: Duration = Duration::from_secs(6 * 3600);

/// Background loop: every 6h, export traces and trigger LoRA when count > 500.
pub async fn run_auto_lora_poll(
    db_path: PathBuf,
    mut train_config: crate::train::TrainConfig,
    k8s: crate::config::K8sConfig,
) {
    use crate::finetune::{self, AUTO_LORA_MIN_TRACES};

    loop {
        tokio::time::sleep(AUTO_LORA_POLL_INTERVAL).await;
        match TraceStore::open(&db_path) {
            Ok(store) => {
                let count = store.count().unwrap_or(0);
                if count >= AUTO_LORA_MIN_TRACES {
                    train_config.data = db_path
                        .parent()
                        .map(|p| p.join("lora_export"))
                        .unwrap_or_else(|| PathBuf::from("./lora_export"));
                    let k8s_ref = k8s.clone();
                    let result = tokio::task::spawn_blocking({
                        let cfg = train_config.clone();
                        move || finetune::run_lora_from_traces(&store, &cfg, &k8s_ref, AUTO_LORA_MIN_TRACES)
                    })
                    .await;
                    if let Err(e) = result {
                        warn!(error = %e, "auto-LoRA task join failed");
                    }
                } else {
                    info!(count, min = AUTO_LORA_MIN_TRACES, "auto-LoRA poll: waiting for traces");
                }
            }
            Err(e) => warn!(error = %e, "auto-LoRA poll: cannot open trace db"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_trace(suffix: &str) -> ExecutionTraceEvent {
        ExecutionTraceEvent {
            msg_id: format!("trace:test-{suffix}"),
            execution_id: format!("exec-{suffix}"),
            workflow_name: "demo".into(),
            prompt: format!("prompt {suffix}"),
            completion: format!("completion {suffix}"),
            reward: 1.0,
            success: true,
        }
    }

    #[test]
    fn traces_accumulate_on_success() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("training_traces.db");
        let store = TraceStore::open(&db).unwrap();
        assert_eq!(store.count().unwrap(), 0);
        store.insert(&sample_trace("1")).unwrap();
        assert_eq!(store.count().unwrap(), 1);
        store.insert(&sample_trace("1")).unwrap();
        assert_eq!(store.count().unwrap(), 1);
        store.insert(&sample_trace("2")).unwrap();
        assert_eq!(store.count().unwrap(), 2);
    }
}
