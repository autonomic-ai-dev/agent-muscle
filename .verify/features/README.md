# agent-muscle features

## Baseline preconditions

- Rust toolchain matching `Cargo.lock`.
- Training needs MLX or candle and a GPU host. Operator checks need a Kubernetes context. Neither is available on a default laptop run.

## Driving conventions

- Unit tests run through `cargo test`. Command execution tests must not run host commands outside the allowlist.

## Proof standards

- A feature is proven by its unit test. HTTP routes and the JetStream consumer have no functional test yet.

## Feature index

| Feature file | Sub-features | Primary surface |
|---|---|---|
| `command-and-training.md` | 5 | `run`, `validate`, `train`, `operator`, `serve` (HTTP `/execute`, `/train/*`, `/k8s/*`) |
| `trace-and-adaptation.md` | 8 | Trace store, preference pairs, LoRA registry, outcome report, output compression, AST guard (14 tests), compute consumer (no test) |
