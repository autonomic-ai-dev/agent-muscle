# agent-muscle verification playbooks

Project-root `.verify/` is discovered by `agent-brain verify` / `report` for scenario hashing.

- **`verify.toml`** — command allowlist. Only matching commands count as trusted for future learning.
- Record outcomes with `agent-brain verify -- <cmd>` (source=`cli`) or spine/muscle with `--source orchestrator`.

## Playbooks

| Playbook | Command | Purpose |
|----------|---------|---------|
| **AST Guard** | `agent-brain verify -- .verify/playbooks/verify_ast_guard.sh` | Tests in-memory tree-sitter AST validation, syntax error intercept, and root/home blockers |
| **Output Slicer** | `agent-brain verify -- .verify/playbooks/verify_output_slicer.sh` | Tests adaptive output compression, error slicing, and traceback preservation |
| **Full Suite** | `agent-brain verify -- .verify/playbooks/verify_full_suite.sh` | Runs all 31 unit & integration tests and cargo check with zero warnings |
| **Default Suite** | `agent-brain verify -- .verify/run.sh` | Default complete verification entrypoint |
