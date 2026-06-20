# agent-muscle

**Remote actuator — command execution, JetStream compute jobs, and LoRA fine-tuning.**

`agent-muscle` runs shell commands locally or via NATS, validates training datasets, and orchestrates MLX / candle / K8s GPU training pipelines.

Standalone: `agent-muscle run "cargo test"` · Integrated: JetStream consumer on `autonomic.compute.job`, spine events on execute/train.

---

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/autonomic-ai-dev/agent-muscle/master/scripts/install.sh | bash
```

---

## Quick start

```bash
agent-muscle status
agent-muscle run "echo hello"
agent-muscle validate --data ./training_data
agent-muscle train --backend auto --validate-only
agent-muscle serve                 # HTTP :3103 + JetStream worker
```

---

## Commands

| Command | Description |
|---------|-------------|
| `run <cmd>` | Execute command, JSON result |
| `serve` | HTTP API + JetStream compute consumer |
| `train` | LoRA fine-tune (`--backend mlx\|candle\|auto`) |
| `validate` | JSONL dataset gate |
| `operator run\|sync\|status` | K8s GPU scaling from train queue |
| `k8s render-job` | Emit GPU Job manifest |

---

## HTTP API

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Daemon health |
| `POST /execute` | Run command |
| `POST /train/validate` | Dataset validation |
| `POST /train/run` | Start training pipeline |
| `GET /k8s/status` · `POST /k8s/sync` | GPU operator |

---

## Configuration

Sections `[muscle]`, `[train]`, `[k8s]` in `~/.autonomic/config.toml` (default port **3103**).

Train queue subject: `autonomic.muscle.train.request`

---

## Development

```bash
cargo test --release -p agent-muscle
cargo build --release -p agent-muscle --features candle   # CUDA device probe
```

---

## License

MIT
