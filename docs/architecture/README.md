# agent-muscle architecture documentation

## Design goals

agent-muscle handles two distinct but related concerns: **safe command execution** and **model fine-tuning**. Both share a validation-first pipeline: validate inputs before touching expensive resources (subprocess, GPU hours).

### Command execution pipeline

```
agent-muscle run "cargo test"
  1. Parse command string
  2. Spawn subprocess (no TTY, no interactive input)
  3. Capture stdout/stderr with line-buffered readers
  4. Wait for exit with configurable timeout
  5. Return JSON: { exit_code, stdout, stderr, duration_ms, success }
```

### Training pipeline (gate stages)

```
agent-muscle train --backend auto
  Stage 1: validate --data
    → Check JSONL format, instruction/response pairs, min entries
  Stage 2: train --validate-only (dry run)
    → Check model availability, backend detection, config validity
  Stage 3: train (actual training)
    → Backend: MLX (Apple Silicon), candle (CUDA), or K8s operator
```

### Key design decisions

| Decision | Rationale |
|----------|-----------|
| **JSON output, not raw stdout** | Structured output is machine-parseable. Agents don't need to grep for exit codes. |
| **Validation gates before GPU** | A single malformed JSONL file can waste hours of GPU time. Validate first, train second. |
| **Backend auto-detection** | `--backend auto` probes for MLX, then candle, then falls back to K8s. Users don't need to know which backend is available. |
| **JetStream consumer for async** | `serve` mode subscribes to `autonomic.compute.job`. Organs can submit jobs without waiting for a response. |

### Alternatives considered

| Option | Why rejected |
|--------|-------------|
| **PTY-based execution** | PTYs add complexity and unpredictable behavior. Subprocess with captured stdio is simpler and more reliable. |
| **Docker container per command** | Too heavyweight for "run `cargo test`". The subprocess backend is fast; Docker and Firecracker are available via immune if stronger isolation is needed. |
| **Cloud-only training (Modal/RunPod)** | Ties training to a cloud provider. MLX and candle run locally, free, and offline. |
