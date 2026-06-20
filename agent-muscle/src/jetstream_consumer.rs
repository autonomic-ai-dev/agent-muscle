use anyhow::{Context, Result};
use async_nats::jetstream::{self, consumer::AckPolicy, stream::StorageType};
use futures::StreamExt;
use std::path::Path;
use std::time::Duration;
use tracing::{error, info, warn};

use agent_body_core::nats::subjects;
use agent_body_core::{ComputeJob, ComputeResult, STREAM_NAME, STREAM_SUBJECT_WILDCARD};

async fn connect_js(url: &str) -> Result<jetstream::Context> {
    let client = async_nats::connect(url)
        .await
        .context("connect to NATS")?;
    let js = jetstream::new(client);
    js.get_or_create_stream(jetstream::stream::Config {
        name: STREAM_NAME.to_string(),
        subjects: vec![STREAM_SUBJECT_WILDCARD.to_string()],
        storage: StorageType::File,
        duplicate_window: agent_body_core::default_duplicate_window(),
        max_age: Duration::from_secs(7 * 24 * 3600),
        ..Default::default()
    })
    .await
    .context("ensure AUTONOMIC stream")?;
    js.create_consumer_on_stream(
        jetstream::consumer::pull::Config {
            durable_name: Some("muscle-compute".into()),
            filter_subject: subjects::COMPUTE_JOB.into(),
            ack_policy: AckPolicy::Explicit,
            ack_wait: agent_body_core::default_ack_wait(),
            ..Default::default()
        },
        STREAM_NAME,
    )
    .await
    .ok();
    Ok(js)
}

async fn publish_result(js: &jetstream::Context, result: &ComputeResult) -> Result<()> {
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Nats-Msg-Id", result.msg_id.as_str());
    let bytes = serde_json::to_vec(result)?;
    js.publish_with_headers(
        subjects::COMPUTE_RESULT.to_string(),
        headers,
        bytes.into(),
    )
        .await?
        .await
        .context("publish compute result")?;
    Ok(())
}

pub async fn run_compute_consumer(url: &str) -> Result<()> {
    let js = connect_js(url).await?;
    let consumer = js
        .get_consumer_from_stream("muscle-compute", STREAM_NAME)
        .await
        .context("get muscle-compute consumer")?;
    let mut messages = consumer
        .fetch()
        .max_messages(1)
        .messages()
        .await
        .context("fetch compute jobs")?;

    info!("agent-muscle JetStream consumer active on {}", subjects::COMPUTE_JOB);

    while let Some(msg) = messages.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "compute consumer fetch error");
                continue;
            }
        };

        let job: ComputeJob = match serde_json::from_slice(&msg.payload) {
            Ok(j) => j,
            Err(e) => {
                error!(error = %e, "invalid compute job payload");
                let _ = msg.ack().await;
                continue;
            }
        };

        let cwd = job.cwd.as_deref().map(Path::new);
        let result = match crate::executor::run_command(&job.command, cwd).await {
            Ok(exec) => ComputeResult {
                msg_id: format!("{}-result", job.msg_id),
                job_id: job.job_id.clone(),
                exit_code: exec.exit_code,
                stdout: exec.stdout,
                stderr: exec.stderr,
                success: exec.success,
                duration_ms: exec.duration_ms,
            },
            Err(e) => ComputeResult {
                msg_id: format!("{}-result", job.msg_id),
                job_id: job.job_id,
                exit_code: -1,
                stdout: String::new(),
                stderr: e.to_string(),
                success: false,
                duration_ms: 0,
            },
        };

        if let Err(e) = publish_result(&js, &result).await {
            warn!(error = %e, "failed to publish compute result");
        }
        if let Err(e) = msg.ack().await {
            warn!(error = %e, "failed to ack compute job");
        }
    }

    Ok(())
}
