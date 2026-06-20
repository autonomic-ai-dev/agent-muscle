use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub worker: WorkerConfig,
    pub spine: SpineConfig,
    pub logging: LoggingConfig,
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
            spine: SpineConfig {
                url: "http://localhost:3100".into(),
            },
            logging: LoggingConfig {
                level: "info".into(),
            },
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("agent-muscle")
            .join("config.yaml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let s = std::fs::read_to_string(&path)?;
            Ok(serde_yaml::from_str(&s)?)
        } else {
            let cfg = Config::default();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let s = serde_yaml::to_string(&cfg)?;
            std::fs::write(&path, &s)?;
            Ok(cfg)
        }
    }
}
