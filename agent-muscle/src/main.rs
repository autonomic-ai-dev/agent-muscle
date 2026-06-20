use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "agent-muscle", about = "Remote actuator and command execution")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the actuator daemon
    Serve,
    /// Run a command and stream output
    Run {
        /// Command to execute
        command: String,
        /// Working directory
        #[arg(short, long)]
        cwd: Option<std::path::PathBuf>,
    },
    /// Show status
    Status,
    /// Run local LoRA fine-tuning via MLX
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
        #[arg(long, default_value_t = 1)]
        min_entries: u64,
        #[arg(long)]
        validate_only: bool,
    },
    /// Validate JSONL training data without running MLX
    Validate {
        #[arg(long, default_value = "./training_data")]
        data: std::path::PathBuf,
        #[arg(long, default_value_t = 1)]
        min_entries: u64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Serve => {
            let config = agent_muscle::config::Config::load()?;
            agent_muscle::serve::start(config).await?;
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
        }
        Commands::Train {
            model,
            data,
            epochs,
            learning_rate,
            lora_rank,
            output,
            min_entries,
            validate_only,
        } => {
            let cfg = agent_muscle::train::TrainConfig {
                model,
                data,
                epochs,
                learning_rate,
                lora_rank,
                output_dir: output,
                use_mlx: true,
                min_entries,
                validate_only,
            };
            agent_muscle::train::run_training(&cfg)?;
        }
        Commands::Validate { data, min_entries } => {
            let report = agent_muscle::dataset::validate_dataset(&data, min_entries)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if !report.valid {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
