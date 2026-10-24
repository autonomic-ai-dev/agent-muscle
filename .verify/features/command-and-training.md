# command-and-training

Summary: agent-muscle runs sandboxed shell and Python commands and returns JSON results, validates training datasets, runs LoRA fine-tuning, and drives GPU training jobs on Kubernetes.

## Sub-features

- `command-run`: Executes one command and returns a JSON result.
  - Intent: Give agents a controlled place to run commands.
  - Behaviour: `agent-muscle run <cmd>` and `POST /execute` run the command and capture output and exit code.
  - Outcome: A JSON object with stdout, stderr, and exit code.
- `dataset-validate`: Checks a JSONL training dataset.
  - Intent: Reject bad training data before a costly run.
  - Behaviour: `agent-muscle validate --data PATH` and `POST /train/validate` check the file.
  - Outcome: Exit 0 for a valid dataset, non-zero with the first violation otherwise.
- `train-run`: Starts a LoRA fine-tuning pipeline.
  - Intent: Retrain the local model from accumulated trajectories.
  - Behaviour: `agent-muscle train` with `--backend mlx`, `candle`, or `auto`. `POST /train/run` starts it over HTTP.
  - Outcome: A training pipeline starts and reports progress.
- `k8s-gpu-operator`: Scales GPU Jobs from the training queue.
  - Intent: Run training on cluster GPUs when work is queued.
  - Behaviour: `agent-muscle operator run|sync|status`, `k8s render-job`, `GET /k8s/status`, `POST /k8s/sync`.
  - Outcome: A rendered Job manifest, or a status showing queued and running jobs.
- `jetstream-compute`: Consumes compute requests from the bus.
  - Intent: Let other organs submit work without a direct call.
  - Behaviour: `agent-muscle serve` runs the HTTP API and a JetStream compute consumer.
  - Outcome: A queued request is executed and its result published.

## Code

- `agent-muscle/src/config.rs`
- `agent-muscle/src/dataset.rs`
- `agent-muscle/src/finetune`
- `agent-muscle/src/k8s`
- `agent-muscle/src/lib.rs`
- `agent-muscle/src/log.rs`
- `agent-muscle/src/main.rs`
- `agent-muscle/src/manifest.rs`
- `agent-muscle/src/mcp_server.rs`
- `agent-muscle/src/serve.rs`
- `agent-muscle/src/spine.rs`
- `agent-muscle/src/train.rs`
- `agent-muscle/src/update.rs`
- `agent-muscle/Cargo.toml`

## How to get to it (user POV)
- Run guard: `ast_guard::validate_bash` delegates to `agent_body_core::guard::inspect`. The rules are in the `command-policy` feature map in agent-body.

- CLI: `agent-muscle run`, `validate`, `train`, `operator`, `k8s render-job`, `status`, `serve`, `serve-mcp`
- HTTP: `GET /health`, `POST /execute`, `POST /train/validate`, `POST /train/run`, `GET /k8s/status`, `POST /k8s/sync`
- MCP: `muscle_execute_bash`, `muscle_execute_python` through agent-body

## Verification Suites (Targeted Test Commands)

- **Unit Tests:** `cargo test` (whole crate, 43 tests in the baseline)
- **Functional API Tests (Hurl):** not yet written
- **Allowlisted Verification:** entries in `.verify/verify.toml`
- **Preconditions:** Rust toolchain. `train` needs MLX or candle and a GPU host. `operator` needs a Kubernetes context.

### Verification Steps

1. **muscle-unit-tests** (sub-features: all)
   - Request: `cargo test`
   - Why: Proves command execution, dataset validation, and operator logic.
   - Expected: 43 passed, 0 failed (baseline).
   - Impact: A failure means execution or training gating changed. Fix the cause.
## Gotchas

- Missing proof: `dataset-validate` has no committed valid and malformed JSONL fixture under `.verify/`, so the validation gate is not shown to accept and reject correctly. Add the fixtures, then a step that runs `agent-muscle validate --data` on each.
- `command-run` executes arbitrary commands. Every verification run must go through the allowlist and the AST gate in agent-mouth, not a bare shell.
- `train-run` needs GPU hardware. It is not part of the default functional run and is recorded as not available on a laptop without MLX.
- `k8s-gpu-operator` talks to a live cluster when it runs. Tests must mock the cluster boundary or skip the step with a recorded reason.
