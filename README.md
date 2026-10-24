# agent-muscle

`agent-muscle` runs commands for agents in a subprocess and returns a structured JSON result. It also validates training datasets and runs fine-tuning jobs.

Docs: [autonomic-ai-dev.vercel.app](https://autonomic-ai-dev.vercel.app)

## Quick start

```bash
brew tap autonomic-ai-dev/tap && brew install agent-muscle
agent-muscle run "echo hello"
agent-muscle status
```

## Commands

| Command | What it does |
|---------|--------------|
| `agent-muscle run <cmd>` | Runs a command and returns `stdout`, `stderr`, `exit_code`, and `duration_ms` as JSON |
| `agent-muscle serve` | Runs the HTTP API on port 3103 and the JetStream compute consumer |
| `agent-muscle serve-mcp` | Runs the MCP stdio server only |
| `agent-muscle train` | Runs LoRA fine-tuning (`--backend mlx`, `candle`, or `auto`) |
| `agent-muscle validate --data PATH` | Checks a JSONL dataset before any training run |
| `agent-muscle operator run\|sync\|status` | Scales GPU training jobs on Kubernetes |
| `agent-muscle status` | Shows backends and dataset paths |

## HTTP API

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Daemon health and uptime |
| `POST` | `/execute` | Runs a command |
| `POST` | `/train/validate` | Validates a training dataset |
| `POST` | `/train/run` | Starts a training pipeline |
| `GET` | `/k8s/status` | Kubernetes operator status |
| `POST` | `/k8s/sync` | Syncs the GPU job queue to the operator |

## Command safety

Commands pass a bash AST guard before they run. The rules that block destructive or dynamic commands are listed in the `command-policy` feature map in agent-body.

## Configuration

Settings live in the `[muscle]`, `[train]`, and `[k8s]` sections of `~/.autonomic/config.toml`. The default port is 3103. Training jobs arrive on the `autonomic.muscle.train.request` subject.

## Develop

```bash
cargo test -p agent-muscle
cargo clippy -p agent-muscle -- -D warnings
```

Feature map and knowledge base: `.verify/features/` and `.verify/knowledge-base/`.
