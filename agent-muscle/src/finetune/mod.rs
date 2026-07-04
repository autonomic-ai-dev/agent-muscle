//! Fine-tuning backends (MLX local, candle + K8s GPU).

pub mod candle;
pub mod mlx;

use anyhow::{bail, Result};

use crate::train::TrainConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrainBackend {
    Mlx,
    Candle,
    Auto,
}

impl TrainBackend {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "mlx" => Ok(Self::Mlx),
            "candle" => Ok(Self::Candle),
            "auto" => Ok(Self::Auto),
            _ => bail!("invalid backend '{s}' (use mlx, candle, or auto)"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mlx => "mlx",
            Self::Candle => "candle",
            Self::Auto => "auto",
        }
    }
}

pub fn resolve_backend(config: &TrainConfig, k8s: &crate::config::K8sConfig) -> TrainBackend {
    match config.backend {
        TrainBackend::Auto => {
            if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
                TrainBackend::Mlx
            } else if k8s.enabled {
                TrainBackend::Candle
            } else {
                // On non-Apple hardware, always fallback to Candle.
                // The candle backend runner will gracefully bail with a clean error
                // if neither Kubernetes nor local CUDA is available.
                TrainBackend::Candle
            }
        }
        other => other,
    }
}

pub fn run_backend_training(config: &TrainConfig, k8s: &crate::config::K8sConfig) -> Result<()> {
    let backend = resolve_backend(config, k8s);
    println!(
        "  Backend: {} (resolved from {})",
        backend.as_str(),
        config.backend.as_str()
    );

    match backend {
        TrainBackend::Mlx => mlx::run_mlx_training(config),
        TrainBackend::Candle => candle::run_candle_training(config, k8s),
        TrainBackend::Auto => unreachable!("resolve_backend never returns Auto"),
    }
}

pub fn backend_label(config: &TrainConfig, k8s: &crate::config::K8sConfig) -> String {
    resolve_backend(config, k8s).as_str().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::K8sConfig;

    #[test]
    fn parses_backend_names() {
        assert_eq!(TrainBackend::parse("MLX").unwrap(), TrainBackend::Mlx);
        assert_eq!(TrainBackend::parse("candle").unwrap(), TrainBackend::Candle);
        assert!(TrainBackend::parse("invalid").is_err());
    }

    #[test]
    fn auto_prefers_mlx_on_mac() {
        let config = TrainConfig {
            backend: TrainBackend::Auto,
            ..TrainConfig::default()
        };
        let k8s = K8sConfig::default();
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(resolve_backend(&config, &k8s), TrainBackend::Mlx);
        }
    }

    #[test]
    fn candle_forces_candle_path() {
        let config = TrainConfig {
            backend: TrainBackend::Candle,
            ..TrainConfig::default()
        };
        assert_eq!(
            resolve_backend(&config, &K8sConfig::default()),
            TrainBackend::Candle
        );
    }
}


use crate::config::K8sConfig;
use crate::trace_collector::TraceStore;

/// Minimum successful traces before auto-LoRA (Kernel V2 Phase 5).
pub const AUTO_LORA_MIN_TRACES: u64 = 500;

/// Export traces to JSONL and run LoRA when `trace_count >= min_traces`.
pub fn run_lora_from_traces(
    store: &TraceStore,
    config: &TrainConfig,
    k8s: &K8sConfig,
    min_traces: u64,
) -> anyhow::Result<()> {
    let count = store.count()?;
    if count < min_traces {
        tracing::info!(count, min_traces, "skipping auto-LoRA: insufficient traces");
        return Ok(());
    }

    let export_dir = config.data.clone();
    let jsonl = store.export_jsonl(&export_dir)?;
    tracing::info!(path = %jsonl.display(), count, "exported traces for LoRA");

    if config.validate_only {
        let validation = crate::train::validate_training_data(config)?;
        if !validation.valid {
            anyhow::bail!("dataset validation failed");
        }
        return Ok(());
    }

    run_backend_training(config, k8s)
}

#[cfg(test)]
mod lora_trace_tests {
    use super::*;
    use crate::trace_collector::{ExecutionTraceEvent, TraceStore};
    use tempfile::tempdir;

    fn seed_traces(store: &TraceStore, n: u64) {
        for i in 0..n {
            store
                .insert(&ExecutionTraceEvent {
                    msg_id: format!("trace:seed-{i}"),
                    execution_id: format!("exec-{i}"),
                    workflow_name: "wf".into(),
                    prompt: format!("prompt {i}"),
                    completion: format!("completion {i}"),
                    reward: 1.0,
                    success: true,
                })
                .unwrap();
        }
    }

    #[test]
    fn lora_round_from_500_traces() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("training_traces.db");
        let store = TraceStore::open(&db).unwrap();
        seed_traces(&store, 500);
        assert_eq!(store.count().unwrap(), 500);

        let data_dir = dir.path().join("data");
        let jsonl = store.export_jsonl(&data_dir).unwrap();
        assert!(jsonl.exists());

        let report = crate::dataset::validate_dataset(&data_dir, 500).unwrap();
        assert!(report.valid, "{:?}", report.errors);

        let cfg = TrainConfig {
            data: data_dir,
            validate_only: true,
            min_entries: 500,
            output_dir: dir.path().join("out"),
            ..TrainConfig::default()
        };
        let k8s = K8sConfig::default();
        run_lora_from_traces(&store, &cfg, &k8s, 500).unwrap();
    }
}
