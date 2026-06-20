use anyhow::{Context, Result};
use async_nats::jetstream::{self, consumer::AckPolicy};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

use agent_body_core::{STREAM_NAME, STREAM_SUBJECT_WILDCARD};

use crate::config::K8sConfig;
use crate::k8s::gpu_job;
use crate::train::TrainConfig;

pub const TRAIN_REQUEST_SUBJECT: &str = "autonomic.muscle.train.request";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainQueueSnapshot {
    pub subject: String,
    pub pending: u64,
    pub threshold: u32,
    pub active_jobs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorStatus {
    pub enabled: bool,
    pub queue: TrainQueueSnapshot,
    pub last_job: Option<String>,
    pub device: String,
}

pub async fn operator_status(url: &str, k8s: &K8sConfig) -> Result<OperatorStatus> {
    let queue = queue_snapshot(url, k8s).await?;
    Ok(OperatorStatus {
        enabled: k8s.enabled,
        queue,
        last_job: read_last_job_path(k8s),
        device: crate::finetune::candle::device_summary(),
    })
}

pub async fn sync_gpu_jobs(
    url: &str,
    k8s: &K8sConfig,
    train_defaults: &TrainConfig,
) -> Result<OperatorStatus> {
    if !k8s.enabled {
        anyhow::bail!("k8s.enabled must be true in config");
    }

    let queue = queue_snapshot(url, k8s).await?;
    info!(
        pending = queue.pending,
        threshold = queue.threshold,
        "k8s operator sync"
    );

    if queue.pending >= queue.threshold as u64 && queue.active_jobs == 0 {
        let job_id = uuid::Uuid::new_v4().simple().to_string();
        let yaml = gpu_job::render_train_job(train_defaults, k8s, &job_id)?;
        let out_dir = agent_body_core::organ_state_dir("muscle").join("k8s");
        std::fs::create_dir_all(&out_dir)?;
        let path = out_dir.join(format!("train-{job_id}.yaml"));
        std::fs::write(&path, yaml)?;

        if k8s.auto_apply {
            gpu_job::apply_yaml_file(&path)?;
            info!(job_id, "applied GPU training job");
        }

        let state_path = out_dir.join("last-job.txt");
        std::fs::write(&state_path, &job_id)?;
    } else if queue.pending == 0 && queue.active_jobs > 0 {
        info!("queue empty with active jobs — leaving jobs running");
    }

    operator_status(url, k8s).await
}

async fn queue_snapshot(url: &str, k8s: &K8sConfig) -> Result<TrainQueueSnapshot> {
    let client = async_nats::connect(url).await.context("connect to NATS")?;
    let js = jetstream::new(client);
    js.get_or_create_stream(jetstream::stream::Config {
        name: STREAM_NAME.to_string(),
        subjects: vec![STREAM_SUBJECT_WILDCARD.to_string()],
        duplicate_window: agent_body_core::default_duplicate_window(),
        ..Default::default()
    })
    .await
    .ok();

    let consumer_name = "muscle-train-operator";
    js.create_consumer_on_stream(
        jetstream::consumer::pull::Config {
            durable_name: Some(consumer_name.into()),
            filter_subject: TRAIN_REQUEST_SUBJECT.into(),
            ack_policy: AckPolicy::Explicit,
            ack_wait: agent_body_core::default_ack_wait(),
            ..Default::default()
        },
        STREAM_NAME,
    )
    .await
    .ok();

    let mut consumer: async_nats::jetstream::consumer::Consumer<
        async_nats::jetstream::consumer::pull::Config,
    > = js
        .get_consumer_from_stream(consumer_name, STREAM_NAME)
        .await
        .context("get muscle-train-operator consumer")?;

    let info = consumer.info().await.context("read consumer info")?;
    let pending = info.num_pending;
    let active_jobs = count_active_k8s_jobs(k8s).unwrap_or(0);

    Ok(TrainQueueSnapshot {
        subject: TRAIN_REQUEST_SUBJECT.into(),
        pending,
        threshold: k8s.queue_threshold,
        active_jobs,
    })
}

fn count_active_k8s_jobs(k8s: &K8sConfig) -> Result<u64> {
    let output = std::process::Command::new("kubectl")
        .args([
            "get",
            "jobs",
            "-n",
            &k8s.namespace,
            "-l",
            "component=train",
            "--field-selector",
            "status.successful!=1",
            "-o",
            "jsonpath={.items[*].metadata.name}",
        ])
        .output()
        .context("kubectl get jobs")?;

    if !output.status.success() {
        return Ok(0);
    }

    let names = String::from_utf8_lossy(&output.stdout);
    Ok(names.split_whitespace().count() as u64)
}

fn read_last_job_path(_k8s: &K8sConfig) -> Option<String> {
    let path = agent_body_core::organ_state_dir("muscle")
        .join("k8s")
        .join("last-job.txt");
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

pub async fn run_operator_loop(
    url: String,
    k8s: K8sConfig,
    train_defaults: TrainConfig,
) -> Result<()> {
    info!("agent-muscle K8s operator loop started");
    loop {
        match sync_gpu_jobs(&url, &k8s, &train_defaults).await {
            Ok(status) => info!(
                pending = status.queue.pending,
                active = status.queue.active_jobs,
                "operator tick"
            ),
            Err(e) => warn!(error = %e, "operator sync failed"),
        }
        tokio::time::sleep(Duration::from_secs(k8s.sync_interval_secs)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn train_subject_is_stable() {
        assert_eq!(TRAIN_REQUEST_SUBJECT, "autonomic.muscle.train.request");
    }
}
