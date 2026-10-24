# Trace and adaptation

## What it does

agent-muscle records each command execution as a trace, turns traces into preference pairs for training, keeps LoRA adapters that can be swapped at runtime, reports outcomes to agent-heart, and shortens long output before it returns. Every command passes an AST guard before it runs.

## How it works

1. **Trace store.** Each execution writes an `ExecutionTraceEvent` to SQLite through `TraceStore` (`src/trace_collector.rs`).
2. **Trace consumer.** `run_trace_consumer` runs as a long-lived task. It reads trace events from the bus and stores them.
3. **Auto-LoRA poll.** `run_auto_lora_poll` checks the trace store periodically, and starts training when enough data exists.
4. **Preference pairs.** `export_preference_pairs` (`src/preference_pairs.rs`) returns chosen and rejected pairs, up to a limit.
5. **Adapter registry.** `LoraAdapterRegistry` (`src/lora_hot_swap.rs`) tracks the adapters that can be swapped.
6. **Outcome report.** `report_to` posts an `OutcomeReport` to agent-heart's `POST /outcomes/record`. A failed post, including heart answering HTTP 500, is returned as an error and logged at warn level. It does not stop the caller.
7. **Output compression.** `compress_output` keeps at most `max_lines` lines and preserves the exit code.
8. **AST guard.** `validate_bash` (`src/ast_guard.rs`) rejects destructive commands before the executor runs them. It also checks script files.
9. **Compute consumer.** `run_compute_consumer` takes `ComputeJob` messages from `autonomic.compute.job`, runs them through the guarded executor, and publishes `ComputeResult` to `autonomic.compute.result`.

## Where the code is

- `src/trace_collector.rs`, `src/preference_pairs.rs`, `src/lora_hot_swap.rs`, `src/outcome_report.rs`, `src/output_slicer.rs`, `src/ast_guard.rs`, `src/jetstream_consumer.rs`
- Test: `tests/jetstream_consumer_tests.rs`, which starts a real `nats-server`

## How to use it

- `agent-muscle run <cmd>` runs a command through the guard and returns JSON
- `agent-muscle serve` runs the HTTP API and the bus consumers
- Publish a `ComputeJob` on `autonomic.compute.job` to run work without a direct call

## How it is verified

- `cargo test` for the whole crate (43 tests in the recorded baseline)
- `sh .verify/checks/require_tests.sh ast_guard`: the 14 destructive-command cases. A zero-match count is not evidence.
- `sh .verify/checks/require_tests.sh output_slicer`: 3 tests
- `sh .verify/checks/require_tests.sh --test jetstream_consumer_tests`: the compute job runs and its result is published
- `bash .verify/checks/route_drift.sh`

## Known limits

- The AST guard in agent-mouth is a separate implementation. The two policies can drift, so any rule change must go into both, with a test in each. A shared policy crate would remove that risk.
- The auto-LoRA trigger threshold is not checked by any test.
- If agent-heart is down, the outcome report fails. Confirm the caller treats that as non-fatal.
- The graph lists a `gpu_job` module that does not exist under `src/`. The GPU operator is `crate::k8s::operator`, and no contract maps it yet.
- The consumer's URL handling was fixed on 2026-10-24. The helper is duplicated in agent-immune until body-core ships it under a new tag.

## Related

- Command and training: [command-and-training](command-and-training.md)
- Sandbox in immune: [scan-and-sandbox](../../../agent-immune/.verify/knowledge-base/scan-and-sandbox.md)
