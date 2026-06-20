use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub struct TrainConfig {
    pub model: String,
    pub data: PathBuf,
    pub epochs: u32,
    pub learning_rate: f64,
    pub lora_rank: u32,
    pub output_dir: PathBuf,
    pub use_mlx: bool,
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
            use_mlx: true,
        }
    }
}

pub fn run_training(config: &TrainConfig) -> Result<()> {
    let python_check = Command::new("python3")
        .arg("-c")
        .arg("import mlx; print(mlx.__version__)")
        .output();

    match python_check {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("✅ MLX available (version: {})", version);
        }
        _ => {
            println!("⚠️  MLX not found. Attempting to install...");
            let install = Command::new("pip3")
                .args(["install", "mlx-lm"])
                .output()?;
            if !install.status.success() {
                let stderr = String::from_utf8_lossy(&install.stderr);
                anyhow::bail!("Failed to install mlx-lm: {}", stderr);
            }
            println!("✅ MLX installed successfully");
        }
    }

    std::fs::create_dir_all(&config.output_dir)?;

    if !config.data.exists() {
        anyhow::bail!("Training data directory not found: {}", config.data.display());
    }

    println!();
    println!("🚀 Starting LoRA fine-tuning...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Model:    {}", config.model);
    println!("  Data:     {}", config.data.display());
    println!("  Epochs:   {}", config.epochs);
    println!("  LR:       {:.0e}", config.learning_rate);
    println!("  LoRA rank: {}", config.lora_rank);
    println!("  Output:   {}", config.output_dir.display());
    println!();

    let status = Command::new("mlx_lm.train")
        .args([
            "--model", &config.model,
            "--data", &config.data.to_string_lossy(),
            "--num-layers", &config.lora_rank.to_string(),
            "--iters", &(config.epochs * 100).to_string(),
            "--learning-rate", &config.learning_rate.to_string(),
            "--fine-tune-type", "lora",
            "--save-path", &config.output_dir.to_string_lossy(),
        ])
        .status()?;

    if status.success() {
        println!();
        println!("✅ Training complete!");
        println!("  Adapters saved to: {}", config.output_dir.display());
        println!();
        println!("To use the fine-tuned model:");
        println!("  mlx_lm.generate --model {} --adapter {}", config.model, config.output_dir.display());
    } else {
        anyhow::bail!("Training failed with exit code: {:?}", status.code());
    }

    Ok(())
}
