//! Native train manifest written before MLX execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::dataset::DatasetValidationReport;
use crate::train::TrainConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainManifest {
    pub version: u32,
    pub backend: String,
    pub model: String,
    pub data_path: String,
    pub output_dir: String,
    pub epochs: u32,
    pub learning_rate: f64,
    pub lora_rank: u32,
    pub dataset: DatasetValidationReport,
    pub created_at: String,
}

impl TrainManifest {
    pub fn from_config(config: &TrainConfig, dataset: DatasetValidationReport) -> Self {
        Self {
            version: 1,
            backend: if config.use_mlx { "mlx" } else { "mlx" }.into(),
            model: config.model.clone(),
            data_path: config.data.display().to_string(),
            output_dir: config.output_dir.display().to_string(),
            epochs: config.epochs,
            learning_rate: config.learning_rate,
            lora_rank: config.lora_rank,
            dataset,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn write(&self, dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join("train.manifest.json");
        let body = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, body)?;
        Ok(path)
    }
}
