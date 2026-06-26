use clap::{Parser, Subcommand, ValueEnum};

use agent_body_core::cli::apply_progress_env;
use agent_body_core::ui::ProgressMode;
use agent_muscle::finetune::TrainBackend;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ProgressArg {
    Auto,
    Plain,
    Quiet,
}

impl From<ProgressArg> for ProgressMode {
    fn from(value: ProgressArg) -> Self {
        match value {
            ProgressArg::Auto => ProgressMode::Auto,
            ProgressArg::Plain => ProgressMode::Plain,
            ProgressArg::Quiet => ProgressMode::Quiet,
        }
    }
}

#[derive(Parser)]
#[command(version)]
#[command(name = "agent-muscle", about = "Remote actuator and command execution")]
struct Cli {
    /// Progress output style: auto, plain, or quiet
    #[arg(long, value_enum, global = true, default_value = "auto")]
    progress: ProgressArg,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the actuator daemon
    Serve,
    /// Start the Standalone MCP Server over stdio
    ServeMcp,
    /// Run a command and stream output
    Run {
        command: String,
        #[arg(short, long)]
        cwd: Option<std::path::PathBuf>,
    },
    /// Show status
    Status,
    /// Run LoRA fine-tuning (MLX, candle, or auto)
    Train {
        #[arg(long, default_value = "mlx-community/Llama-3.2-3B-Instruct-4bit")]
        model: String,
        #[arg(long, default_value = "./training_data")]
        data: std::path::PathBuf,
        #[arg(long, default_value_t = 3)]
        epochs: u32,
        #[arg(long, default_value_t = 1e-5)]
        learning_rate: f64,
        #[arg(long, default_value_t = 16)]
        lora_rank: u32,
        #[arg(long, default_value = "./lora_adapters")]
        output: std::path::PathBuf,
        #[arg(long, default_value = "auto")]
        backend: String,
        #[arg(long, default_value_t = 1)]
        min_entries: u64,
        #[arg(long)]
        validate_only: bool,
    },
    /// Validate JSONL training data without running training
    Validate {
        #[arg(long, default_value = "./training_data")]
        data: std::path::PathBuf,
        #[arg(long, default_value_t = 1)]
        min_entries: u64,
    },
    /// Kubernetes GPU training operator
    Operator {
        #[command(subcommand)]
        command: OperatorCommands,
    },
    /// Kubernetes GPU job helpers
    K8s {
        #[command(subcommand)]
        command: K8sCommands,
    },
    /// Update agent-muscle to the latest release
    Update {
        #[arg(short, long)]
        force: bool,
    },
    /// Display daemon logs
    Log {
        /// Daemon name (e.g. spine, nerves, heart) or "all"
        name: Option<String>,
        /// Follow log output (tail -f)
        #[arg(short, long)]
        follow: bool,
        /// List available log files
        #[arg(short, long)]
        list: bool,
    },
}

#[derive(Subcommand)]
enum OperatorCommands {
    /// Run operator loop (scale GPU jobs from JetStream queue)
    Run,
    /// One-shot sync of GPU jobs from JetStream queue depth
    Sync,
    /// Show operator / queue status
    Status,
}

#[derive(Subcommand)]
enum K8sCommands {
    /// Render a GPU training Job manifest to stdout
    RenderJob {
        #[arg(long)]
        job_id: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    apply_progress_env(cli.progress.into());
    match cli.command {
        Commands::Serve => {
            let config = agent_muscle::config::Config::load()?;
            agent_muscle::serve::start(config).await?;
        }
        Commands::ServeMcp => {
            let config = agent_muscle::config::Config::load()?;
            agent_muscle::mcp_server::MuscleMcp::run(config).await?;
        }
        Commands::Run { command, cwd } => {
            let result = agent_muscle::executor::run_command(&command, cwd.as_deref()).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Status => {
            let config = agent_muscle::config::Config::load()?;
            println!("agent-muscle status");
            println!(
                "  config: {}",
                agent_muscle::config::Config::config_path().display()
            );
            println!("  port: {}", config.server.port);
            println!("  spine: {}", config.spine.url);
            println!(
                "  default dataset: {}",
                agent_muscle::dataset::default_merged_dataset().display()
            );
            println!(
                "  train backend default: auto (device: {})",
                agent_muscle::finetune::candle::device_summary()
            );
            println!(
                "  k8s: enabled={} namespace={} gpus={}",
                config.k8s.enabled, config.k8s.namespace, config.k8s.gpu_count
            );
        }
        Commands::Train {
            model,
            data,
            epochs,
            learning_rate,
            lora_rank,
            output,
            backend,
            min_entries,
            validate_only,
        } => {
            let config = agent_muscle::config::Config::load()?;
            let cfg = agent_muscle::train::TrainConfig {
                model,
                data,
                epochs,
                learning_rate,
                lora_rank,
                output_dir: output,
                backend: TrainBackend::parse(&backend)?,
                min_entries,
                validate_only,
            };
            agent_muscle::train::run_training(&cfg, &config.k8s)?;
        }
        Commands::Validate { data, min_entries } => {
            let report = agent_muscle::dataset::validate_dataset(&data, min_entries)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if !report.valid {
                std::process::exit(1);
            }
        }
        Commands::Operator { command } => {
            let config = agent_muscle::config::Config::load()?;
            match command {
                OperatorCommands::Run => {
                    let url = config.nats.url.clone();
                    let k8s = config.k8s.clone();
                    let train = config.default_train_config();
                    agent_muscle::k8s::operator::run_operator_loop(url, k8s, train).await?;
                }
                OperatorCommands::Sync => {
                    let status = agent_muscle::k8s::operator::sync_gpu_jobs(
                        &config.nats.url,
                        &config.k8s,
                        &config.default_train_config(),
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
                OperatorCommands::Status => {
                    let status =
                        agent_muscle::k8s::operator::operator_status(&config.nats.url, &config.k8s)
                            .await?;
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
            }
        }
        Commands::K8s { command } => match command {
            K8sCommands::RenderJob { job_id } => {
                let config = agent_muscle::config::Config::load()?;
                let job_id = job_id.unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string());
                let train = config.default_train_config();
                let yaml = agent_muscle::k8s::render_train_job(&train, &config.k8s, &job_id)?;
                print!("{yaml}");
            }
        },
        Commands::Update { force } => {
            agent_muscle::update::run_update(force)?;
        }
        Commands::Log { name, follow, list } => {
            if list {
                let logs = agent_muscle::log::list_logs()?;
                if logs.is_empty() {
                    println!("No log files found.");
                } else {
                    println!("Available logs:");
                    for log in &logs {
                        println!("  {log}");
                    }
                }
                return Ok(());
            }
            let name = match name {
                Some(n) => n,
                None => anyhow::bail!(
                    "usage: agent-muscle log <name> [--follow]  (or --list to see available logs)"
                ),
            };
            if follow {
                agent_muscle::log::follow_log(&name)?;
            } else {
                agent_muscle::log::print_log(&name)?;
            }
        }
    }
    Ok(())
}
