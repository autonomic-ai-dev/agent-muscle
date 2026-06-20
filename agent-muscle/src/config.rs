use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub worker: WorkerConfig,
    pub nats: NatsConfig,
    pub spine: SpineConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub train: TrainDefaults,
    #[serde(default)]
    pub k8s: K8sConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainDefaults {
    #[serde(default = "default_train_model")]
    pub model: String,
    #[serde(default = "default_train_data")]
    pub data: String,
    #[serde(default = "default_train_output")]
    pub output: String,
    #[serde(default = "default_epochs")]
    pub epochs: u32,
    #[serde(default = "default_learning_rate")]
    pub learning_rate: f64,
    #[serde(default = "default_lora_rank")]
    pub lora_rank: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_k8s_namespace")]
    pub namespace: String,
    #[serde(default = "default_gpu_count")]
    pub gpu_count: u32,
    #[serde(default = "default_k8s_image")]
    pub image: String,
    #[serde(default = "default_queue_threshold")]
    pub queue_threshold: u32,
    #[serde(default = "default_sync_interval")]
    pub sync_interval_secs: u64,
    #[serde(default)]
    pub auto_apply: bool,
    #[serde(default = "default_job_ttl")]
    pub job_ttl_seconds: u32,
    #[serde(default = "default_gpu_node_key")]
    pub gpu_node_selector_key: String,
    #[serde(default = "default_gpu_node_value")]
    pub gpu_node_selector_value: String,
    #[serde(default = "default_memory_request")]
    pub memory_request: String,
}

fn default_train_model() -> String {
    "mlx-community/Llama-3.2-3B-Instruct-4bit".into()
}

fn default_train_data() -> String {
    "./training_data".into()
}

fn default_train_output() -> String {
    "./lora_adapters".into()
}

fn default_epochs() -> u32 {
    3
}

fn default_learning_rate() -> f64 {
    1e-5
}

fn default_lora_rank() -> u32 {
    16
}

fn default_k8s_namespace() -> String {
    "autonomic".into()
}

fn default_gpu_count() -> u32 {
    1
}

fn default_k8s_image() -> String {
    "ghcr.io/autonomic-ai-dev/agent-muscle:latest".into()
}

fn default_queue_threshold() -> u32 {
    1
}

fn default_sync_interval() -> u64 {
    60
}

fn default_job_ttl() -> u32 {
    86_400
}

fn default_gpu_node_key() -> String {
    "accelerator".into()
}

fn default_gpu_node_value() -> String {
    "nvidia-gpu".into()
}

fn default_memory_request() -> String {
    "16Gi".into()
}

impl Default for TrainDefaults {
    fn default() -> Self {
        Self {
            model: default_train_model(),
            data: default_train_data(),
            output: default_train_output(),
            epochs: default_epochs(),
            learning_rate: default_learning_rate(),
            lora_rank: default_lora_rank(),
        }
    }
}

impl Default for K8sConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            namespace: default_k8s_namespace(),
            gpu_count: default_gpu_count(),
            image: default_k8s_image(),
            queue_threshold: default_queue_threshold(),
            sync_interval_secs: default_sync_interval(),
            auto_apply: false,
            job_ttl_seconds: default_job_ttl(),
            gpu_node_selector_key: default_gpu_node_key(),
            gpu_node_selector_value: default_gpu_node_value(),
            memory_request: default_memory_request(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub max_concurrent_jobs: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NatsConfig {
    pub url: String,
    pub jetstream_consumer: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpineConfig {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig { port: 3103 },
            worker: WorkerConfig {
                max_concurrent_jobs: 4,
            },
            nats: NatsConfig {
                url: "nats://localhost:4222".into(),
                jetstream_consumer: true,
            },
            spine: SpineConfig {
                url: "http://localhost:3100".into(),
            },
            logging: LoggingConfig {
                level: "info".into(),
            },
            train: TrainDefaults::default(),
            k8s: K8sConfig::default(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        agent_body_core::config_path()
    }

    pub fn load() -> Result<Self> {
        agent_body_core::organ_config::load("muscle")
    }

    pub fn default_train_config(&self) -> crate::train::TrainConfig {
        crate::train::TrainConfig {
            model: self.train.model.clone(),
            data: PathBuf::from(&self.train.data),
            epochs: self.train.epochs,
            learning_rate: self.train.learning_rate,
            lora_rank: self.train.lora_rank,
            output_dir: PathBuf::from(&self.train.output),
            backend: crate::finetune::TrainBackend::Auto,
            min_entries: 1,
            validate_only: false,
        }
    }
}
