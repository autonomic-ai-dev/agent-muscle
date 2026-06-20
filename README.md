# agent-muscle

**Remote actuator and command execution — run commands, stream output, manage workers.**

agent-muscle executes shell commands on remote or local workers, streaming stdout/stderr back in structured JSON output.

---

## Why agent-muscle?

| Problem | agent-muscle answer |
|---------|-------------------|
| "I need to run a build on a remote machine" | **Command execution** — runs shell commands, captures all output |
| "My agent needs to compile code" | **Structured results** — exit code, stdout, stderr, duration |

## Commands

| Command | Description |
|---------|-------------|
| `agent-muscle serve` | Start daemon (future: NATS worker) |
| `agent-muscle run <cmd>` | Execute a command and return results |
| `agent-muscle status` | Show config |

---

## Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/autonomic-ai-dev/agent-muscle/master/scripts/install.sh | bash
```

## Development

```bash
cargo build --release -p agent-muscle
cargo test --release -p agent-muscle
```

## License

MIT
