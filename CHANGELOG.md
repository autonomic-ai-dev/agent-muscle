# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0-beta.1] - 2026-10-16

### Changed
- Unified ecosystem versioning to 1.0.0-beta.1
- Pinned `agent-body-core` dependency to `v1.0.0-beta.1`
- Preserved CI Developer ID signatures in `scripts/install.sh` (clears quarantine without re-signing)
- Dynamic GitHub pre-release flag in release workflows for beta/rc/alpha tags
- CI strictly arm64 macOS native runners with Developer ID code signing
- Aligned documentation and CI path filters with agent-brain

## [0.9.0] - 2026-07-18

### Added

- **Phase 9** `lora_hot_swap` + Phase 5.1 DPO preference pairs

## [0.8.0] - 2026-07-04

### Added
- **Phase 5 trace collector** — NATS `autonomic.spine.execution.completed` → SQLite `training_traces.db`
- **`run_lora_from_traces()`** — export successful traces to JSONL and trigger LoRA training
- **Auto-LoRA poll** — `serve` daemon runs 6h poll, trains when trace count ≥ 500

## [0.7.9] - 2026-06-27

### Changed

- **agent-body-core dependency** — updated tag to `v0.5.14` for ecosystem alignment

## [0.7.8] - 2026-06-27

### Added

- **MCP server** (`agent-muscle serve-mcp`) — starts an MCP stdio server for gateway aggregation. Used by `agent-body serve-mcp`.
- **Organ tool definitions** — `muscle_execute_bash`, `muscle_execute_python`, `muscle_finetune` with `#[tool_router]`/`#[tool_handler]` macros (rmcp 1.7)
- **Integration tests** — default parameter validation, tool handler trait compliance

## [0.7.7] - 2026-06-23

### Added

- Global `--progress` CLI flag for structured ProgressTree output (also `AGENT_PROGRESS=1`)

## [0.7.6] - 2026-06-21

### Fixed

- CI builds use git `agent-body-core` dependency instead of a local path

## [0.7.5] - 2026-06-21

### Changed

- JetStream consumer uses `agent_body_core::connect_nats()` for authenticated NATS
- `async-nats` 0.39 aligned with `agent-body-core` 0.3.3

## [0.7.4] - 2026-06-21

### Added

- `agent-muscle update [--force]` — self-update subcommand that checks GitHub releases, compares versions, and downloads the latest binary

## [0.7.3] - 2026-06-21

### Added

- `agent-muscle log <name> [--follow] [--list]` — read daemon logs from the supervisor log directory

## [0.7.2] - 2026-06-21

### Fixed

- agent-spine registration is now non-fatal — daemon starts even without spine available

## [0.7.1] - 2026-06-20

### Added

- `--version` CLI flag (`f2153ca`)
- Mermaid architecture charts in README (`d3d46fa`)

### Changed

- Professional README with standalone and integrated usage (`f525c6b`)
- Fix CUDA candle cfg on Apple Silicon (`f525c6b`)

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
