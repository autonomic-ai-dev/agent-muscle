use anyhow::Result;
use std::path::PathBuf;

use crate::config::K8sConfig;
use crate::dataset::DatasetValidationReport;
use crate::finetune::{self, TrainBackend};
use crate::manifest::TrainManifest;

#[derive(Debug, Clone)]
pub struct TrainConfig {
    pub model: String,
    pub data: PathBuf,
    pub epochs: u32,
    pub learning_rate: f64,
    pub lora_rank: u32,
    pub output_dir: PathBuf,
    pub backend: TrainBackend,
    pub min_entries: u64,
    pub validate_only: bool,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            model: "mlx-community/Llama-3.2-3B-Instruct-4bit".into(),
            data: PathBuf::from("./training_data"),
            epochs: 3,
            learning_rate: 1e-5,
            lora_rank: 16,
            output_dir: PathBuf::from("./lora_adapters"),
            backend: TrainBackend::Auto,
            min_entries: 1,
            validate_only: false,
        }
    }
}

pub fn validate_training_data(config: &TrainConfig) -> Result<DatasetValidationReport> {
    crate::dataset::validate_dataset(&config.data, config.min_entries)
}

pub fn run_training(config: &TrainConfig, k8s: &K8sConfig) -> Result<()> {
    let validation = validate_training_data(config)?;
    println!("{}", serde_json::to_string_pretty(&validation)?);

    if !validation.valid {
        anyhow::bail!("dataset validation failed");
    }

    let manifest = TrainManifest::from_config(config, k8s, validation);
    let manifest_path = manifest.write(&config.output_dir)?;
    println!("  Manifest: {}", manifest_path.display());

    if config.validate_only {
        println!("✅ Dataset validation passed (validate-only mode)");
        return Ok(());
    }

    finetune::run_backend_training(config, k8s)
}
