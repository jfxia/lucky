# Lucky MCP Adapter for Claude Code

Execute Lucky programs directly from Claude Code via the Model Context Protocol (MCP). This adapter bridges Claude Code's MCP tool interface to the Lucky Tool Protocol (LTP) runtime.

## Prerequisites

- **Lucky compiler** installed and on PATH (provides the `lucky` CLI)
- **Python 3.8+** with no additional dependencies (stdlib only)
- **Claude Code** with MCP support

## Quick Start

1. Start the Lucky LTP server (in a separate terminal):

   ```bash
   lucky serve --transport http --port 9700
   ```

2. Copy `settings.json` into your Claude Code configuration, or merge its `mcpServers` section:

   ```json
   {
     "mcpServers": {
       "lucky-ltp": {
         "command": "python",
         "args": [
           "D:\\test\\lucky\\lucky-compiler\\adapters\\claude-code\\mcp-server.py"
         ],
         "env": {
           "LUCKY_LTP_SERVER": "http://localhost:9700",
           "LUCKY_LTP_TOKEN": ""
         },
         "description": "Lucky language MCP server",
         "autoStart": true
       }
     }
   }
   ```

3. Restart Claude Code. The `lucky_run`, `lucky_status`, and `lucky_approve` tools will be available.

## Tools

### lucky_run

Execute a Lucky program.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `source` | string | yes | Lucky source code |
| `goal` | string | no | Goal name (entry point) |
| `context` | object | no | Execution context (e.g. `{"repo": "...", "branch": "main"}`) |
| `mode` | string | no | `sync` (wait for result) or `async` (return execution_id) |

### lucky_status

Check execution status. Use with `execution_id` from an async `lucky_run`.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `execution_id` | string | yes | Execution ID from lucky_run |

### lucky_approve

Respond to a human approval request from a running Lucky program.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `approval_id` | string | yes | Approval request ID |
| `decision` | string | yes | `approve`, `reject`, or `modify` |
| `reason` | string | no | Explanation for the decision |

## Transport Modes

### HTTP (default)

```bash
lucky serve --transport http --port 9700
```

Set `LUCKY_LTP_SERVER=http://localhost:9700` in settings. Use `LUCKY_LTP_TOKEN` for bearer auth.

### stdio (subprocess)

Set `LUCKY_LTP_SERVER` to `ltp+stdio://lucky serve --transport stdio`. The MCP server spawns the Lucky runtime as a subprocess — no separate server needed.

## Troubleshooting

- **"lucky CLI not found"**: Install the Lucky compiler and ensure it's on PATH.
- **"Connection refused"**: Make sure `lucky serve` is running on the configured port.
- **Debug output**: The MCP server logs to stderr. Check Claude Code's MCP server logs.
- **Tool not appearing**: Verify `settings.json` is correctly placed and Claude Code restarted.
