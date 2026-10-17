# agent-muscle — Context for agents

Execution and training runtime for Autonomic. Part of [agent-body](https://github.com/autonomic-ai-dev/agent-body).

| Field | Value |
|-------|-------|
| Role | Execution runtime (kubelet / CRI analog) |
| Binary | `agent-muscle` |
| HTTP | `:3103` |
| Version | v1.0.0-beta.2 |
| Docs Hub | [https://autonomic-ai.dev/docs/execution](https://autonomic-ai.dev/docs/execution) · [Architecture](https://autonomic-ai.dev/docs/architecture) |

## When to use

Run sandboxed commands, typed JSON execute APIs, or hardware-aware train jobs. LLM emits a routing signal; muscle executes natively (no JSON tax).

## Key APIs

- `POST /execute` — command + cwd → stdout/stderr/exit/duration
- `POST /train/run` — model/data/backend (MLX / Candle)
- JetStream consumer when nerves is up

## Verification

After execute/test nodes, prefer:

```bash
agent-brain verify --source orchestrator -- cargo test
```

Allowlist lives in the **target project** `.verify/verify.toml`.

## CI

Path filter on `agent-muscle/**` + Cargo/Makefile/scripts/`.github`. Docs-only PRs skip compose.

## Related

Operator usage: [https://autonomic-ai.dev/docs/execution](https://autonomic-ai.dev/docs/execution) · Architecture: [https://autonomic-ai.dev/docs/architecture](https://autonomic-ai.dev/docs/architecture) · Spine (orchestration) · immune (sandbox/scan) · brain (verify ledger)
