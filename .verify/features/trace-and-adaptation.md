# trace-and-adaptation

Summary: agent-muscle records every command execution as a trace, turns those traces into preference pairs for training, keeps LoRA adapters swappable at runtime, reports outcomes back to agent-heart, and compresses long command output before it returns to the caller. Commands pass an AST guard before they run.

## Sub-features

- `trace-store`: Persists execution trace events to SQLite.
  - Intent: Keep a durable record of what was run and how it ended, for training and audit.
  - Behaviour: `ExecutionTraceEvent` (`agent-muscle/src/trace_collector.rs` line 17) is written to `TraceStore` (line 34). `default_db_path` (line 27) picks the file.
  - Outcome: Each execution adds one event row. Covered by 1 inline test.
- `trace-consumer`: Reads trace events from the bus and stores them.
  - Behaviour: `run_trace_consumer` (line 143) runs as a long-lived task.
  - Outcome: Events published on the bus appear in the store. Not verified against a live broker in this pass.
- `auto-lora-poll`: Checks the trace store periodically and decides when to start training.
  - Behaviour: `run_auto_lora_poll` (line 200) receives the training defaults and the k8s configuration, and starts the training pipeline when enough data exists.
  - Outcome: A training run starts. The threshold that triggers it is not verified in this pass.
- `preference-pairs`: Exports chosen and rejected pairs from the trace store for training.
  - Behaviour: `export_preference_pairs(conn, limit)` (`src/preference_pairs.rs` line 15) returns `PreferencePair` values (line 8).
  - Outcome: Up to `limit` pairs. Covered by 1 inline test.
- `lora-adapter-registry`: Tracks the LoRA adapters that can be swapped in at runtime.
  - Behaviour: `LoraAdapterRegistry` (`src/lora_hot_swap.rs` line 7).
  - Outcome: The registry holds the adapter set. Covered by 1 inline test. Whether a swap changes a running model is not verified.
- `outcome-report`: Sends one execution outcome to agent-heart.
  - Intent: Feed the self-learning critic in agent-heart.
  - Behaviour: `report_to(base_url, report)` (`src/outcome_report.rs` line 21) posts an `OutcomeReport` (line 12). `report` (line 33) wraps `report_to`. The base URL it uses was not read in this pass.
  - Outcome: The POST is sent. Covered by 2 inline tests. Whether the receiving route in agent-heart (`POST /outcomes/record`) accepts the payload is not verified in this pass.
- `output-compression`: Shortens long command output before it is returned.
  - Intent: Keep large logs from overwhelming a caller's context.
  - Behaviour: `compress_output(raw, exit_code, max_lines)` (`src/output_slicer.rs` line 7).
  - Outcome: At most `max_lines` lines are kept, with the exit code preserved. Covered by 3 inline tests.
- `ast-guard`: Rejects destructive shell commands before execution.
  - Intent: Stop `rm -rf /`, disk writes, and pipe-to-shell installs before they reach the host.
  - Behaviour: `validate_bash(command)` (`src/ast_guard.rs` line 63) is called by `executor.rs` line 26 before a command runs. It also validates script files (line 243).
  - Outcome: A rejected command returns an error and does not run. Covered by 14 inline tests.
- `jetstream-compute-consumer`: Receives compute requests from the bus.
  - Behaviour: `run_compute_consumer(url)` (`src/jetstream_consumer.rs` line 52) consumes a `ComputeJob` from `autonomic.compute.job` and publishes a `ComputeResult` to `autonomic.compute.result`.
  - Outcome: The job runs through the guarded executor and its result is published. Covered by `tests/jetstream_consumer_tests.rs`, which starts a real `nats-server` with JetStream, checks the echoed output, and stops the server. Recorded through `require_tests.sh --test jetstream_consumer_tests`.

## How to get to it (user POV)

- CLI: `agent-muscle run <cmd>` runs the executor, which applies the AST guard. `agent-muscle train` starts training from the trace store.
- HTTP: `POST /execute` (guarded), `POST /train/run`, and `GET /k8s/status` on the daemon port.
- Bus: `run_compute_consumer` and `run_trace_consumer` run inside `agent-muscle serve`.

## Verification Suites (Targeted Test Commands)

- **Unit Tests:** `cargo test` (whole crate, 43 tests in the baseline). The guarded filters: `sh .verify/checks/require_tests.sh ast_guard` and `sh .verify/checks/require_tests.sh output_slicer`. Each fails on zero matched tests.
- **Functional API Tests (Hurl):** not written.
- **Allowlisted Verification:** entries in `.verify/verify.toml`, including `bash .verify/checks/route_drift.sh`, which checks the README route table against source.
- **Preconditions:** Rust toolchain. `auto-lora-poll` and training need the k8s and GPU setup described in `command-and-training.md`.

### Verification Steps

1. **ast-guard-unit-tests** (sub-feature: ast-guard)
   - Request: `sh .verify/checks/require_tests.sh ast_guard`
   - Why: Proves the 14 destructive-command cases are rejected and the safe cases pass.
   - Expected: The matching tests pass, 0 failed. Check the printed passed count. The `ast_guard` filter should match the 14 inline tests. A count of 0 is not evidence.
   - Impact: A failure means a destructive command could run. Fix the guard. Never loosen the test.
2. **output-compression-unit-tests** (sub-feature: output-compression)
   - Request: `sh .verify/checks/require_tests.sh output_slicer`
   - Why: Proves long output is cut to the limit and the exit code survives.
   - Expected: 3 passed, 0 failed.
   - Impact: A failure means callers receive truncated or misleading output.

## Gotchas

- Duplicate guard: agent-mouth has its own AST gate (its crate has 36 passing tests in total), and this crate has a separate `ast_guard.rs` with 14 inline tests. The two policies are maintained separately and can drift. Any rule added to one must be added to the other, with a test in each. Consider a shared policy crate.
- Fixed (2026-10-24): `connect_js(url)` in `jetstream_consumer.rs` connects to the URL it is given, through `connect_nats_at`. That helper applies the `AUTONOMIC_NATS_*` auth and TLS settings. The test points `AUTONOMIC_NATS_URL` at a dead port, so only the URL argument can reach the broker. Before, a caller that passed a different URL was silently sent to the environment's broker.
- Gap: `auto-lora-poll`'s trigger threshold is not checked by any test in this pass.
- `outcome_report` posts to agent-heart. If heart is down, the report fails. Confirm the caller treats that as non-fatal before relying on it.
- The graph lists a `gpu_job` module, but no `gpu_job.rs` exists under `src/`. The GPU operator is referenced as `crate::k8s::operator` in `serve.rs`. It is not mapped by any contract yet.
