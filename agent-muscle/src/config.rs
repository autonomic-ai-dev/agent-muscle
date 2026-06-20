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
}
