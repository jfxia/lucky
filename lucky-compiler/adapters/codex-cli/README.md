# Codex CLI Adapter for Lucky

Integrates Lucky goal-oriented agent programs as tools within OpenAI's Codex CLI.

## Prerequisites

- Python 3.9+
- Lucky compiler installed and available as `lucky` on PATH
- A running Lucky LTP server (or use stdio mode which auto-spawns one)

## Setup

### 1. Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `LUCKY_SERVER_URL` | No | `http://localhost:9700` | HTTP endpoint of the LTP server. Use `ltp+stdio://lucky serve --transport stdio` for stdio mode. |
| `LUCKY_SERVER_TOKEN` | No | — | Bearer token for authenticated LTP servers. |

### 2. Install the adapter in Codex CLI

Copy or symlink the adapter configuration into your Codex CLI agent directory:

```
codex agents install D:\test\lucky\lucky-compiler\adapters\codex-cli
```

Or register the tool executor in your Codex CLI session:

```
codex tool add lucky_execute --executor "python D:\test\lucky\lucky-compiler\adapters\codex-cli\tool-executor.py lucky_execute"
codex tool add lucky_status  --executor "python D:\test\lucky\lucky-compiler\adapters\codex-cli\tool-executor.py lucky_status"
codex tool add lucky_approve --executor "python D:\test\lucky\lucky-compiler\adapters\codex-cli\tool-executor.py lucky_approve"
```

### 3. Start an LTP server (if using HTTP mode)

```bash
lucky serve --transport http --port 9700
```

### 4. Verify

```bash
python tool-executor.py lucky_execute --program ..\..\examples\tiny.lk
```

## Tools

### lucky_execute

Run a Lucky program.

```
lucky_execute --program path/to/prog.lk [--goal GoalName] [--context '{"key":"val"}'] [--mode sync|async]
```

### lucky_status

Poll an async execution.

```
lucky_status --execution_id <id>
```

### lucky_approve

Approve or reject a human-approval request.

```
lucky_approve --approval_id <id> --decision approved [--reason "looks good"]
```

## stdin mode

Tool executor also accepts JSON on stdin:

```bash
echo '{"tool":"lucky_execute","args":{"program":"examples/tiny.lk"}}' | python tool-executor.py
```
