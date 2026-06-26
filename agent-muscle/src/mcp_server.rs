use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ErrorData as McpError, ServerInfo, Implementation, ServerCapabilities};
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;

use crate::config::Config;
use crate::executor::{run_command, run_python};
use crate::finetune::TrainBackend;
use crate::train::{run_training, TrainConfig};

#[derive(Clone)]
pub struct MuscleMcp {
    config: Config,
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

impl MuscleMcp {
    pub fn new(config: Config) -> Self {
        Self { 
            config,
            tool_router: Self::tool_router(),
        }
    }

    pub async fn run(config: Config) -> anyhow::Result<()> {
        let server = Self::new(config);
        let service = server.serve(rmcp::transport::io::stdio()).await?;
        service.waiting().await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExecuteParams {
    command: String,
    cwd: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PythonParams {
    code: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FinetuneParams {
    #[serde(default = "default_model")]
    model: String,
    #[serde(default = "default_data")]
    data: String,
    #[serde(default = "default_epochs")]
    epochs: u32,
    #[serde(default = "default_learning_rate")]
    learning_rate: f64,
    #[serde(default = "default_lora_rank")]
    lora_rank: u32,
    #[serde(default = "default_output_dir")]
    output_dir: String,
    #[serde(default = "default_backend")]
    backend: String,
    #[serde(default = "default_min_entries")]
    min_entries: u64,
    #[serde(default = "default_validate_only")]
    validate_only: bool,
}

fn default_model() -> String {
    "mlx-community/Llama-3.2-3B-Instruct-4bit".to_string()
}
fn default_data() -> String {
    "./training_data".to_string()
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
fn default_output_dir() -> String {
    "./lora_adapters".to_string()
}
fn default_backend() -> String {
    "auto".to_string()
}
fn default_min_entries() -> u64 {
    1
}
fn default_validate_only() -> bool {
    false
}

#[tool_router]
impl MuscleMcp {
    #[tool(description = "Execute a shell command natively on the host via agent-muscle")]
    async fn muscle_execute_bash(
        &self,
        params: Parameters<ExecuteParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let cwd_path = p.cwd.map(PathBuf::from);
        match run_command(&p.command, cwd_path.as_deref()).await {
            Ok(result) => {
                let text =
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string());
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Err(McpError::invalid_params(format!("{e}"), None)),
        }
    }

    #[tool(description = "Run a Python snippet in an isolated interpreter")]
    async fn muscle_execute_python(
        &self,
        params: Parameters<PythonParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        match run_python(&p.code).await {
            Ok(result) => {
                let text =
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string());
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Err(McpError::invalid_params(format!("{e}"), None)),
        }
    }

    #[tool(description = "Trigger a local LoRA fine-tuning job on Apple MLX or Candle")]
    async fn muscle_finetune(
        &self,
        params: Parameters<FinetuneParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let backend = TrainBackend::parse(&p.backend)
            .map_err(|e| McpError::invalid_params(format!("{e}"), None))?;

        let cfg = TrainConfig {
            model: p.model,
            data: PathBuf::from(p.data),
            epochs: p.epochs,
            learning_rate: p.learning_rate,
            lora_rank: p.lora_rank,
            output_dir: PathBuf::from(p.output_dir),
            backend,
            min_entries: p.min_entries,
            validate_only: p.validate_only,
        };

        let k8s_cfg = self.config.k8s.clone();

        let result = tokio::task::spawn_blocking(move || run_training(&cfg, &k8s_cfg)).await;

        match result {
            Ok(Ok(())) => Ok(CallToolResult::success(vec![Content::text(
                "Fine-tuning job completed successfully.".to_string(),
            )])),
            Ok(Err(e)) => Err(McpError::internal_error(
                format!("Training failed: {e}"),
                None,
            )),
            Err(e) => Err(McpError::internal_error(
                format!("Thread panic/error: {e}"),
                None,
            )),
        }
    }
}

#[tool_handler]
impl ServerHandler for MuscleMcp {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.instructions = Some(
            "Agent-Muscle MCP Server. Tools: muscle_execute_bash (shell commands), muscle_execute_python (Python snippets), muscle_finetune (MLX/Candle LoRA)."
                .into(),
        );
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        let mut impl_info = Implementation::default();
        impl_info.name = "agent-muscle".into();
        impl_info.version = env!("CARGO_PKG_VERSION").into();
        info.server_info = impl_info;
        info
    }
}
