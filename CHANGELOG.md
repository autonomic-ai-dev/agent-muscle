# Changelog

## [Unreleased]

## [0.7.0] - 2026-06-20

### Added

- **Multi-backend training** — `--backend mlx|candle|auto` with MLX on Apple Silicon and candle/K8s on CUDA
- **Candle orchestration** — GPU device probe (`--features candle`), local CUDA helper, Metal→MLX delegation
- **Kubernetes GPU operator** — watches `autonomic.muscle.train.request` JetStream depth and renders/applies GPU Jobs
- **CLI** — `operator run|sync|status`, `k8s render-job`
- **HTTP** — `/train/run`, `/k8s/status`, `/k8s/sync`; operator loop when `[k8s] enabled = true`
- **Config** — `[train]` defaults and `[k8s]` namespace, GPU count, queue threshold, auto_apply

## [0.6.0] - 2026-06-20

### Added

- **Dataset validation gate** — `agent-muscle validate` and `train --validate-only` check JSONL before MLX
- **Train manifest** — writes `train.manifest.json` with validation report before training
- **HTTP `/train/validate`** — remote validation for agent-heart finetune pipeline

## [0.5.0] - 2026-06-20

### Added

- **JetStream compute consumer** — `serve` consumes `autonomic.compute.job` with explicit ACK, publishes `autonomic.compute.result`

### Changed

- Version bumped from `0.4.0` to `0.5.0`

## [0.4.0] - 2026-06-20

### Added

- **Unified config** — loads from `~/.autonomic/config.toml` via `agent-body-core::organ_config::load("muscle")`

### Changed

- Version bumped from `0.3.0` to `0.4.0`

## [0.3.0] - 2026-06-20

### Added

- **MLX fine-tuning CLI** — `agent-muscle train` provides a 1-line interface for local LoRA training via MLX (Apple Silicon)
- **Auto-install** — Automatically installs `mlx-lm` if not detected
- **Configurable** — Supports `--model`, `--data`, `--epochs`, `--lr`, `--lora-rank`, `--output` flags

### Changed

- Version bumped from `0.2.0` to `0.3.0`

## [0.2.0] - 2026-06-20

### Added

- **HTTP daemon** — `agent-muscle serve` now starts an axum HTTP server with `/health` and `/execute` endpoints
- **Agent-spine integration** — registers with agent-spine event bus on startup, heartbeats every 30s, publishes `muscle.executed` events
- **Config extended** — `server.port` (default 3103) and `spine.url` (default `http://localhost:3100`) settings

### Changed

- Version bumped from `0.1.0` to `0.2.0`

## [0.1.0] - 2026-06-20

### Added

- **Initial project scaffold** — workspace, crate, config
- **Command executor** — runs shell commands via tokio::process, captures stdout/stderr
- **CLI** — `agent-muscle serve` (daemon placeholder), `run <cmd>` (execute), `status`
- **CI pipeline** — test + build + release workflows
