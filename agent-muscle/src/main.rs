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
            println!("  config: {}", agent_muscle::config::Config::config_path().display());
            println!("  port: {}", config.server.port);
            println!("  spine: {}", config.spine.url);
        }
    }
    Ok(())
}
