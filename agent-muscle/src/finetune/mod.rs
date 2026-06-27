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
