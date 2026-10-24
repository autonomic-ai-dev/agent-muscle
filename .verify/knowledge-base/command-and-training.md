# Command and training

## What it does

agent-muscle runs sandboxed shell and Python commands and returns JSON results. It validates training datasets, runs LoRA fine-tuning, and drives GPU training jobs on Kubernetes.

## How it works

1. **Command run.** `agent-muscle run <cmd>` and `POST /execute` run one command and return stdout, stderr, and the exit code as JSON. Every command passes the AST guard first (see [trace-and-adaptation](trace-and-adaptation.md)).
2. **Dataset validation.** `agent-muscle validate --data PATH` and `POST /train/validate` check a JSONL training file. A valid file exits 0. An invalid one exits non-zero with the first violation.
3. **Training.** `agent-muscle train` runs LoRA fine-tuning with `--backend mlx`, `candle`, or `auto`. `POST /train/run` starts the same pipeline over HTTP.
4. **GPU operator.** `agent-muscle operator run|sync|status` and `k8s render-job` scale GPU Jobs from the training queue. `GET /k8s/status` and `POST /k8s/sync` expose the same over HTTP.
5. **Compute queue.** `agent-muscle serve` runs the HTTP API and a JetStream compute consumer. A queued request runs and its result is published.

## Where the code is

- Command execution: `agent-muscle/src/executor.rs`
- Training and validation: `agent-muscle/src/` training modules, dispatched from `serve.rs`
- Operator: `crate::k8s::operator`, called from `serve.rs` lines 80 to 81
- Compute consumer: `agent-muscle/src/jetstream_consumer.rs`

## How to use it

- `agent-muscle run "echo hello"` for one guarded command
- `agent-muscle validate --data train.jsonl` before any training run
- `agent-muscle train --backend auto` on a host with MLX or candle and a GPU
- `agent-muscle operator status` with a Kubernetes context set

## How it is verified

- `cargo test` for the whole crate (43 tests in the recorded baseline), which covers command execution, dataset validation, and operator logic
- `route_drift.sh`: the README route table, including `/k8s/status` and `/k8s/sync`, matches source

## Known limits

- No valid or malformed JSONL fixture is committed under `.verify/`. The validation gate is not shown to accept and reject correctly. Add fixtures, then a step that runs `validate --data` on each.
- Training needs GPU hardware. It is not part of the default functional run, and a laptop without MLX cannot prove it.
- The GPU operator talks to a live cluster when it runs. Tests must mock that boundary, or skip with a recorded reason.
- `command-run` executes arbitrary commands. Verification runs must go through the allowlist and the AST guard, not a bare shell.

## Related

- Trace and adaptation: [trace-and-adaptation](trace-and-adaptation.md)
- Feature map: `agent-muscle/.verify/features/command-and-training.md`
