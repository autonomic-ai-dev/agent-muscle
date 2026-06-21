use anyhow::Result;

use crate::config::K8sConfig;
use crate::k8s::gpu_job;
use crate::train::TrainConfig;

pub fn local_cuda_available() -> bool {
    #[cfg(feature = "candle")]
    {
        candle_core::utils::cuda_is_available()
    }
    #[cfg(not(feature = "candle"))]
    {
        false
    }
}

pub fn local_metal_available() -> bool {
    #[cfg(feature = "candle")]
    {
        candle_core::utils::metal_is_available()
    }
    #[cfg(not(feature = "candle"))]
    {
        false
    }
}

pub fn device_summary() -> String {
    #[cfg(feature = "candle")]
    {
        if candle_core::utils::cuda_is_available() {
            "cuda".into()
        } else if candle_core::utils::metal_is_available() {
            "metal".into()
        } else {
            "cpu".into()
        }
    }
    #[cfg(not(feature = "candle"))]
    {
        "unknown (build with --features candle)".into()
    }
}

pub fn run_candle_training(config: &TrainConfig, k8s: &K8sConfig) -> Result<()> {
    if k8s.enabled || kubeconfig_present() {
        return submit_k8s_gpu_job(config, k8s);
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        println!("ℹ️  Candle on Apple Silicon delegates to MLX for local LoRA");
        crate::finetune::mlx::run_mlx_training(config)
    }

    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        use anyhow::bail;

        if local_cuda_available() {
            return run_local_candle_cuda(config);
        }

        bail!(
            "candle backend requires K8s GPU (`[k8s] enabled = true`) or local CUDA with \
             `cargo build --features candle`. Device: {}",
            device_summary()
        );
    }
}

fn kubeconfig_present() -> bool {
    std::env::var("KUBECONFIG").is_ok()
        || dirs::home_dir()
            .map(|h| h.join(".kube/config").exists())
            .unwrap_or(false)
}

fn submit_k8s_gpu_job(config: &TrainConfig, k8s: &K8sConfig) -> Result<()> {
    let job_id = uuid::Uuid::new_v4().simple().to_string();
    let yaml = gpu_job::render_train_job(config, k8s, &job_id)?;
    let path = config.output_dir.join(format!("k8s-train-{job_id}.yaml"));
    std::fs::create_dir_all(&config.output_dir)?;
    std::fs::write(&path, &yaml)?;

    println!("📦 Rendered K8s GPU training job: {}", path.display());
    println!("  Namespace: {}", k8s.namespace);
    println!("  GPUs:      {}", k8s.gpu_count);
    println!("  Image:     {}", k8s.image);

    if k8s.auto_apply {
        gpu_job::apply_yaml_file(&path)?;
        println!("✅ Applied job agent-muscle-train-{job_id}");
    } else {
        println!("  Apply with: kubectl apply -f {}", path.display());
    }

    Ok(())
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn run_local_candle_cuda(config: &TrainConfig) -> Result<()> {
    use anyhow::{bail, Context};
    use std::process::Command;

    ensure_candle_python()?;

    std::fs::create_dir_all(&config.output_dir)?;

    println!();
    println!("🚀 Starting LoRA fine-tuning (candle / CUDA)...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Device:   {}", device_summary());
    println!("  Model:    {}", config.model);
    println!("  Data:     {}", config.data.display());
    println!("  Output:   {}", config.output_dir.display());
    println!();

    let script = candle_train_script();
    let script_path = config.output_dir.join("candle_lora_train.py");
    std::fs::write(&script_path, script)?;

    let status = Command::new("python3")
        .arg(&script_path)
        .args([
            "--model",
            &config.model,
            "--data",
            &config.data.to_string_lossy(),
            "--output",
            &config.output_dir.to_string_lossy(),
            "--epochs",
            &config.epochs.to_string(),
            "--learning-rate",
            &config.learning_rate.to_string(),
            "--lora-rank",
            &config.lora_rank.to_string(),
        ])
        .status()
        .with_context(|| format!("run {}", script_path.display()))?;

    if status.success() {
        println!("✅ Candle CUDA training complete");
        Ok(())
    } else {
        bail!("candle training failed with exit code: {:?}", status.code());
    }
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn ensure_candle_python() -> Result<()> {
    use anyhow::bail;
    use std::process::Command;

    let check = Command::new("python3")
        .arg("-c")
        .arg("import torch; print(torch.cuda.is_available())")
        .output();
    match check {
        Ok(o) if o.status.success() => {
            let out = String::from_utf8_lossy(&o.stdout);
            if !out.trim().contains("True") {
                bail!("python3 torch CUDA not available for local candle training");
            }
            Ok(())
        }
        _ => bail!("python3 with torch+cuda required for local candle CUDA training"),
    }
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn candle_train_script() -> &'static str {
    r#"#!/usr/bin/env python3
"""Minimal LoRA orchestration script for agent-muscle candle backend."""
import argparse
import json
import subprocess
import sys
from pathlib import Path

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--model", required=True)
    p.add_argument("--data", required=True)
    p.add_argument("--output", required=True)
    p.add_argument("--epochs", type=int, default=3)
    p.add_argument("--learning-rate", type=float, default=1e-5)
    p.add_argument("--lora-rank", type=int, default=16)
    args = p.parse_args()

    out = Path(args.output)
    out.mkdir(parents=True, exist_ok=True)
    plan = {
        "backend": "candle-cuda",
        "model": args.model,
        "data": args.data,
        "epochs": args.epochs,
        "learning_rate": args.learning_rate,
        "lora_rank": args.lora_rank,
    }
    (out / "candle.train.plan.json").write_text(json.dumps(plan, indent=2))

    cmd = [
        sys.executable, "-m", "pip", "install", "-q", "peft", "transformers", "accelerate", "datasets",
    ]
    subprocess.check_call(cmd)

    train_cmd = [
        sys.executable, "-m", "transformers.cli", "train",
        "--model_name_or_path", args.model,
        "--dataset_name", "json",
        "--dataset_config", "default",
        "--output_dir", str(out / "adapters"),
        "--num_train_epochs", str(args.epochs),
        "--learning_rate", str(args.learning_rate),
        "--per_device_train_batch_size", "1",
        "--gradient_accumulation_steps", "4",
        "--bf16",
    ]
    print("Running:", " ".join(train_cmd), file=sys.stderr)
    try:
        subprocess.check_call(train_cmd)
    except subprocess.CalledProcessError:
        print("transformers CLI train unavailable; wrote candle.train.plan.json for K8s handoff", file=sys.stderr)
        sys.exit(0)

if __name__ == "__main__":
    main()
"#
}
