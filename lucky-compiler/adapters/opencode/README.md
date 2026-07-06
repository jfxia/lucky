# OpenCode Adapter for Lucky

Integrates Lucky as an OpenCode skill (`lucky-executor`), enabling OpenCode to compile and execute Lucky programs through the Lucky Tool Protocol (LTP).

## Directory Structure

```
adapters/
├── ltp_client.py          # LTP client (shared by all adapters)
└── opencode/
	├── SKILL.md            # OpenCode skill definition (YAML frontmatter)
	├── run.py              # Tool implementations (lucky_run, lucky_status, lucky_approve)
	└── README.md           # This file
```

## Prerequisites

- **Python 3.9+**
- **Lucky compiler/runtime** — the `lucky` CLI must be on PATH, or set `LUCKY_SERVER_URL`
- **OpenCode** CLI tool installed

## Installation

1. Copy or symlink the skill directory into OpenCode's skills folder:

```bash
# Unix (macOS / Linux)
mkdir -p ~/.config/opencode/skills
cp -r D:\test\lucky\lucky-compiler\adapters\opencode ~/.config/opencode/skills/lucky-executor

# Windows (PowerShell)
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.config\opencode\skills"
Copy-Item -Recurse "D:\test\lucky\lucky-compiler\adapters\opencode" "$env:USERPROFILE\.config\opencode\skills\lucky-executor"
```

2. Ensure the Lucky compiler is available:

```bash
lucky --version
# Lucky compiler v0.1.0
```

## Configuration

### Stdio mode (default)

The executor spawns `lucky serve --transport stdio` as a subprocess.
No extra setup required — just ensure `lucky` is on your PATH.

Override the server command:

```bash
export LUCKY_SERVER_COMMAND="lucky serve --transport stdio --log-level debug"
```

### HTTP mode

Start the Lucky runtime as a long-running HTTP server, then point the executor at it:

```bash
# Terminal 1 — start the server
lucky serve --transport http --port 9700

# Terminal 2 — configure the executor
export LUCKY_SERVER_URL="http://localhost:9700"
export LUCKY_SERVER_TOKEN="ltp-token-xyz"   # if auth is enabled
```

## Usage

### Via OpenCode (skill activation)

Once installed, OpenCode automatically activates the `lucky-executor` skill when you mention Lucky, `.lk` files, workflows, or agents:

```
> Run the Lucky program in deploy.lir with the goal DeployStaging
> Check the status of execution exec-abc-123
> Approve the Lucky deployment gate appr-001
```

### Standalone CLI

The `run.py` script can also be used directly for testing:

```bash
# Run a Lucky program
python run.py run path/to/program.lir --goal BuildWebsite

# Run with context
python run.py run path/to/program.lir --goal DeployStaging -c '{"branch":"main"}'

# Check execution status
python run.py status exec-abc-123

# Respond to an approval
python run.py approve appr-001 approve -r "Changes look good"
```

## How It Works

```
┌─────────────┐     Skill activation      ┌───────────────────┐
│   OpenCode   │ ──────────────────────>  │  lucky-executor   │
│   CLI tool   │                           │  (SKILL.md)       │
└──────┬───────┘                           └────────┬──────────┘
		│                                            │
		│  Calls lucky_run / lucky_status /          │
		│  lucky_approve                             │
		│                                            │
		▼                                            ▼
┌──────────────────────────────────────────────────────────────┐
│  run.py                                                      │
│  ┌──────────────────────────────────────────────────────────┐│
│  │  LtpClient (ltp_client.py)                              ││
│  │  - session/initialize                                   ││
│  │  - ir/load                                              ││
│  │  - execution/start                                      ││
│  │  - execution/get_status                                 ││
│  │  - approval/respond                                     ││
│  └───────────────────────┬──────────────────────────────────┘│
└──────────────────────────┼───────────────────────────────────┘
							│  LTP (JSON-RPC 2.0 over stdio/HTTP)
							▼
┌──────────────────────────────────────────────────────────────┐
│  Lucky Runtime (lucky serve)                                 │
│  - Executes the Lucky IR DAG                                │
│  - Streams events (node_started, node_completed, etc.)      │
│  - Manages checkpoints and approvals                        │
└──────────────────────────────────────────────────────────────┘
```

## Troubleshooting

| Problem | Solution |
|---|---|
| `LtpError -32010: Connection error` | Check that `lucky serve` is running and the URL/port is correct. |
| `FileNotFoundError: IR file not found` | Verify the `.lir` file path is absolute or relative to the working directory. |
| `LtpError -32004: Compilation failed` | The Lucky source has errors. Check `lucky ir` output for diagnostics. |
| `LtpError -30001: AUTH_REQUIRED` | Set `LUCKY_SERVER_TOKEN` to a valid bearer token. |
| OpenCode doesn't recognize the skill | Verify the skill directory is at `~/.config/opencode/skills/lucky-executor/` with `SKILL.md` present. |
