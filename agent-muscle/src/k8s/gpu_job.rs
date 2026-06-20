use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::config::K8sConfig;
use crate::train::TrainConfig;

pub fn render_train_job(config: &TrainConfig, k8s: &K8sConfig, job_id: &str) -> Result<String> {
    let name = format!("agent-muscle-train-{job_id}");
    Ok(format!(
        r#"apiVersion: batch/v1
kind: Job
metadata:
  name: {name}
  namespace: {namespace}
  labels:
    app: agent-muscle
    component: train
spec:
  backoffLimit: 1
  ttlSecondsAfterFinished: {ttl}
  template:
    metadata:
      labels:
        app: agent-muscle
        component: train
    spec:
      restartPolicy: Never
      nodeSelector:
        {node_selector_key}: "{node_selector_value}"
      containers:
        - name: train
          image: {image}
          imagePullPolicy: IfNotPresent
          command: ["agent-muscle", "train"]
          args:
            - "--backend"
            - "candle"
            - "--model"
            - "{model}"
            - "--data"
            - "{data}"
            - "--output"
            - "{output}"
            - "--epochs"
            - "{epochs}"
            - "--learning-rate"
            - "{learning_rate}"
            - "--lora-rank"
            - "{lora_rank}"
          env:
            - name: AUTONOMIC_TRAIN_BACKEND
              value: candle
            - name: NVIDIA_VISIBLE_DEVICES
              value: all
          resources:
            limits:
              nvidia.com/gpu: "{gpu_count}"
            requests:
              nvidia.com/gpu: "{gpu_count}"
              memory: "{memory_request}"
"#,
        namespace = k8s.namespace,
        ttl = k8s.job_ttl_seconds,
        node_selector_key = k8s.gpu_node_selector_key,
        node_selector_value = k8s.gpu_node_selector_value,
        image = k8s.image,
        model = config.model,
        data = config.data.display(),
        output = config.output_dir.display(),
        epochs = config.epochs,
        learning_rate = config.learning_rate,
        lora_rank = config.lora_rank,
        gpu_count = k8s.gpu_count,
        memory_request = k8s.memory_request,
    ))
}

pub fn apply_yaml_file(path: &Path) -> Result<()> {
    let output = Command::new("kubectl")
        .args(["apply", "-f"])
        .arg(path)
        .output()
        .context("spawn kubectl")?;

    if !output.status.success() {
        anyhow::bail!(
            "kubectl apply failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::K8sConfig;
    use crate::finetune::TrainBackend;
    use std::path::PathBuf;

    #[test]
    fn renders_gpu_job_yaml() {
        let config = TrainConfig {
            backend: TrainBackend::Candle,
            model: "meta-llama/Llama-3.2-3B".into(),
            data: PathBuf::from("/data/train"),
            output_dir: PathBuf::from("/out/adapters"),
            ..TrainConfig::default()
        };
        let k8s = K8sConfig::default();
        let yaml = render_train_job(&config, &k8s, "abc123").unwrap();
        assert!(yaml.contains("nvidia.com/gpu"));
        assert!(yaml.contains("agent-muscle-train-abc123"));
        assert!(yaml.contains("candle"));
    }
}
