# Lucky Dify Adapter

Integrate Lucky as a custom Tool in [Dify](https://dify.ai) workflows. This adapter wraps the [Lucky Tool Protocol (LTP)](../../Lucky%20Tool%20Protocol%20Specification%20V0.1.md), enabling Dify pipelines to compile, load, and execute Lucky IR programs as workflow steps.

## Architecture

```
┌──────────────────────────────────────────────────┐
│                   Dify Workflow                   │
│                                                  │
│  ┌──────────┐   ┌──────────────────┐   ┌──────┐ │
│  │ HTTP Req │──▶│ Lucky Executor    │──▶│ LLM  │ │
│  │ (fetch)  │   │ (analyze via LTP) │   │(sum) │ │
│  └──────────┘   └────────┬─────────┘   └──────┘ │
│                          │                       │
└──────────────────────────┼───────────────────────┘
                           │ LTP (HTTP/stdio)
┌──────────────────────────┼───────────────────────┐
│              Lucky Runtime Server                 │
│  ┌───────────────────────▼─────────────────────┐ │
│  │  LTP Server — compiles IR, runs DAG,        │ │
│  │  manages agents, tracks cost                │ │
│  └─────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

## Prerequisites

- Python 3.9+
- A running Lucky LTP server (see below)
- Dify platform (self-hosted or cloud, v0.6.0+)

## Installation

### 1. Start the Lucky LTP Server

```bash
# Install Lucky CLI (if not already installed)
cargo install lucky-cli

# Start the LTP server on port 9700
lucky serve --transport http --port 9700

# Or with authentication
lucky serve --transport http --port 9700 --auth-token "your-secret-token"
```

### 2. Install the Dify Tool Provider

Copy the adapter files into your Dify installation's tool directory:

```bash
# Self-hosted Dify
cp -r lucky-compiler/adapters/dify/ /path/to/dify/api/core/tools/provider/lucky/
cp lucky-compiler/adapters/ltp_client.py /path/to/dify/api/core/tools/provider/lucky/

# Or symlink for development
ln -s $(pwd)/lucky-compiler/adapters/dify/ /path/to/dify/api/core/tools/provider/lucky/
ln -s $(pwd)/lucky-compiler/adapters/ltp_client.py /path/to/dify/api/core/tools/provider/lucky/
```

Install Python dependencies on the Dify server:

```bash
pip install requests
```

### 3. Restart Dify

```bash
# Docker Compose
docker compose restart api worker

# Or standalone
flask db upgrade  # if schema changed
flask run
```

The **Lucky Program Executor** tool should now appear in Dify's tool list (Settings → Tools).

## Configuration

### Credentials

In Dify, go to **Settings → Tools → Lucky Program Executor → Add Credential**:

| Field | Description | Example |
|---|---|---|
| `ltp_endpoint` | LTP server URL | `http://localhost:9700` |
| `ltp_token` | Bearer token (optional) | `ltp-token-v1-...` |
| `ltp_transport` | Transport type | `http` (default) or `stdio` |
| `ltp_command` | Server command (stdio only) | `lucky serve --transport stdio` |

### Parameter Reference

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ir` | string | yes | — | Lucky IR program as a JSON string |
| `goal` | string | yes | — | Goal name to execute |
| `context` | object | no | `{}` | Execution context key-value pairs |
| `mode` | string | no | `sync` | `sync` (blocking) or `async` (returns execution ID) |

### Output Schema

| Field | Type | Description |
|---|---|---|
| `result` | string | Execution result status (`success` / `failure`) |
| `cost_usd` | number | Total cost in USD |
| `outputs` | object | All node outputs from the execution |
| `duration_ms` | number | Total execution duration in milliseconds |
| `execution_id` | string | Execution ID (async mode) |

## Usage

### In a Dify Workflow

1. Open Dify Studio, create a **Workflow** app
2. Drag the **Lucky Program Executor** tool onto the canvas
3. Connect upstream nodes to feed the `ir`, `goal`, and `context` parameters
4. Connect downstream nodes to consume the outputs

Example variable references in Dify's template syntax:

```
{{#lucky_analysis.result#}}       → "success"
{{#lucky_analysis.cost_usd#}}     → 3.45
{{#lucky_analysis.outputs#}}      → {"report": "...", "artifacts": [...]}
{{#lucky_analysis.duration_ms#}}  → 124000
```

### Import the Example Workflow

```bash
# From the Dify UI: Studio → Import DSL → upload workflow-example.yml
# Or via CLI
curl -X POST http://localhost:5001/console/api/apps/import \
  -H "Authorization: Bearer $DIFY_API_TOKEN" \
  -F "file=@workflow-example.yml"
```

### Programmatic Testing

```bash
cd lucky-compiler/adapters/dify/

# Compile a Lucky program to IR
lucky compile ../examples/data_analyzer.lk --output data_analyzer.lir

# Test the provider directly
python provider.py \
  --endpoint http://localhost:9700 \
  --ir-file data_analyzer.lir \
  --goal AnalyzeDataset \
  --context '{"dataset_name":"sales_q2"}' \
  --mode sync
```

## Files

| File | Purpose |
|---|---|
| `tool.yaml` | Dify tool definition (identity, parameters, output schema) |
| `provider.py` | Python tool provider — `validate_credentials` + `invoke` |
| `workflow-example.yml` | Importable Dify workflow demonstrating a 3-step pipeline |
| `README.md` | This file |

## Troubleshooting

**Tool not appearing in Dify:**
- Ensure `ltp_client.py` is importable from `provider.py` (same directory or on `sys.path`).
- Check Dify server logs: `docker compose logs api | grep lucky`.

**Connection refused:**
- Verify the LTP server is running: `curl http://localhost:9700/ltp/v1`.
- Ensure `ltp_endpoint` in credentials matches the server address.

**IR validation fails:**
- Run `lucky validate your_program.lk` to check the source.
- Use `lucky ir your_program.lk` to inspect the generated IR.

**Execution hangs:**
- Check the LTP server logs for agent errors or stuck LLM calls.
- For async mode, poll with `execution/get_status`.
