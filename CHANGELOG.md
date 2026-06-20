# Changelog

## [Unreleased]

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
