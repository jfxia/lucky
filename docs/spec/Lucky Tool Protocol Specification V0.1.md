# Lucky Tool Protocol (LTP)
<img src="../../logo/logo128.png" alt="Lucky logo" width="64" align="right" />


**Version:** 0.1 Draft
**Status:** Protocol Specification
**Target:** AI coding platform integrators, tool developers, runtime implementors

---

# Table of Contents

```
Part I      Protocol Foundation

Chapter 1   Introduction & Design Goals
Chapter 2   Architecture & Roles
Chapter 3   Transport Layer
Chapter 4   Message Format
Chapter 5   Session Lifecycle
Chapter 6   Error Model

----------------------------------------

Part II     Core RPC Methods

Chapter 7   Session Methods
Chapter 8   IR Methods
Chapter 9   Execution Methods
Chapter 10  Approval Methods
Chapter 11  Checkpoint Methods
Chapter 12  Query Methods

----------------------------------------

Part III    Streaming & Events

Chapter 13  Event Stream
Chapter 14  Notification Methods
Chapter 15  Progress Reporting

----------------------------------------

Part IV     Tool Integration

Chapter 16  Tool Registration
Chapter 17  Tool Discovery
Chapter 18  Tool Invocation
Chapter 19  Built-in Tools

----------------------------------------

Part V      Advanced Features

Chapter 20  Multi-Session Coordination
Chapter 21  Resource & Cost Controls
Chapter 22  Cancellation & Timeouts
Chapter 23  Batch Operations

----------------------------------------

Part VI     Platform Bindings

Chapter 24  Claude Code Binding
Chapter 25  Codex CLI Binding
Chapter 26  OpenCode Binding
Chapter 27  Cursor Binding
Chapter 28  Dify Binding
Chapter 29  Standalone Runtime

----------------------------------------

Part VII    Security

Chapter 30  Authentication
Chapter 31  Authorization & Permissions
Chapter 32  Transport Security
Chapter 33  Sandbox Integration

----------------------------------------

Appendix A  Complete Method Reference
Appendix B  Error Code Reference
Appendix C  JSON Schema for All Messages
Appendix D  Conformance Test Suite
Appendix E  Reference Implementation Guide
```

---

# Part I -- Protocol Foundation

---

## Chapter 1 -- Introduction & Design Goals

### 1.1 What Is LTP?

The Lucky Tool Protocol (LTP) is a JSON-RPC 2.0-based protocol that enables AI coding platforms (Claude Code, Codex CLI, OpenCode, Cursor, Dify, and others) to execute Lucky programs through a common interface. LTP is the wire protocol between a **client** (the AI platform) and a **server** (the Lucky runtime or a tool provider).

LTP is to Lucky programs what:
- **LSP** is to programming languages (editor ↔ language server)
- **MCP** is to AI models (model ↔ tool server)
- **HTTP** is to web applications (client ↔ server)

### 1.2 Design Goals

1. **Simplicity**: Based on JSON-RPC 2.0. Any platform with a JSON parser and HTTP/stdio can implement LTP in hours.
2. **Transport agnosticism**: Works over stdio (subprocess), HTTP, WebSocket, or in-process channels.
3. **Streaming first**: Supports server→client event streams for real-time progress, partial LLM outputs, and human approvals.
4. **Security by default**: All sensitive operations require explicit authorization. Permissions are capability-based.
5. **Backend neutrality**: The same LTP client works with any LTP-compliant server (Claude Code, standalone runtime, cloud service).
6. **Composability**: LTP servers can delegate to other LTP servers (tool chaining).
7. **Observability**: Every operation is traceable. The protocol mandates structured logging and metrics.

### 1.3 Non-Goals

- LTP does NOT define how LLMs work internally (prompt formats, tokenization, etc.).
- LTP does NOT define how Lucky source code is compiled (that's the compiler's job).
- LTP does NOT replace IR -- LTP carries IR, but IR can also exist independently.
- LTP is NOT a general-purpose RPC framework. It is specifically designed for Lucky program execution.

### 1.4 Relationship to Other Protocols

```
┌──────────────────────────────────────────────┐
│                AI Platform                    │
│  (Claude Code / Codex CLI / OpenCode / ...)  │
│                                              │
│  ┌────────────────────────────────────────┐  │
│  │           LTP Client                    │  │
│  │  - Sends Lucky IR                      │  │
│  │  - Receives execution events           │  │
│  │  - Handles human approvals             │  │
│  └──────────────┬─────────────────────────┘  │
└─────────────────┼────────────────────────────┘
                  │ LTP (JSON-RPC 2.0)
                  │ over stdio / HTTP / WS
┌─────────────────┼────────────────────────────┐
│  ┌──────────────▼─────────────────────────┐  │
│  │           LTP Server                    │  │
│  │  - Lucky Runtime Engine                │  │
│  │  - Scheduler + Executor                │  │
│  │  - Permission Enforcer                 │  │
│  └──────────────┬─────────────────────────┘  │
│                 │                             │
│  ┌──────────────▼─────────────────────────┐  │
│  │        Backend Adapters                 │  │
│  │  Claude API │ GPT API │ Tools │ Local   │  │
│  └────────────────────────────────────────┘  │
│                                              │
│              Lucky Runtime Server             │
└──────────────────────────────────────────────┘
```

---

## Chapter 2 -- Architecture & Roles

### 2.1 Roles

| Role | Description | Examples |
|---|---|---|
| **LTP Client** | Initiates sessions, submits IR, receives events | Claude Code, Codex CLI, OpenCode, Cursor, Dify |
| **LTP Server** | Executes Lucky programs, manages state, enforces permissions | Lucky Runtime, lucky serve, cloud executor |
| **LTP Tool Provider** | Exposes capabilities (tools) that the server can invoke | Filesystem server, Git server, Browser server |
| **LTP Proxy** | Forwards LTP messages between client and server, potentially adding auth/audit | API gateway, enterprise security layer |

### 2.2 Topologies

#### Direct (One-to-One)

```
Client ──LTP──> Server
```

The most common setup. A single AI platform talks to a single Lucky runtime.

#### Proxied

```
Client ──LTP──> Proxy ──LTP──> Server
                             └──> Server (sharded)
```

Enterprise deployments with authentication, load balancing, and audit logging.

#### Tool Chain

```
Client ──LTP──> Server ──LTP──> Tool Provider A
                     └──LTP──> Tool Provider B
```

The Lucky runtime delegates tool calls to specialized LTP tool providers.

#### Multi-Agent

```
Client ──LTP──> Orchestrator ──LTP──> Agent Server 1
                              └──LTP──> Agent Server 2
                              └──LTP──> Agent Server 3
```

Multiple LTP servers coordinate on a single workflow.

### 2.3 Protocol Versioning

LTP uses semantic versioning. The version is negotiated during session initialization:

```
Client → Server: initialize { "protocol_version": "0.1", "client_info": {...} }
Server → Client: { "protocol_version": "0.1", "server_info": {...} }
```

Rules:
- Major version must match exactly (0.x is development; breaking changes allowed).
- Minor version differences are tolerated (server advertises its version; client adapts).
- The server advertises supported features via `capabilities`.

---

## Chapter 3 -- Transport Layer

### 3.1 Supported Transports

LTP supports four transports. Every compliant implementation MUST support at least one.

| Transport | Use Case | URI Scheme |
|---|---|---|
| **stdio** | Subprocess communication (like LSP) | `ltp+stdio://` |
| **HTTP** | RESTful access, web integration | `ltp+http://` or `ltp+https://` |
| **WebSocket** | Persistent connections, streaming | `ltp+ws://` or `ltp+wss://` |
| **In-Process** | Embedded usage, testing | `ltp+inproc://` |

### 3.2 stdio Transport

Messages are newline-delimited JSON, one message per line, sent over stdin/stdout.

```
Framing:  Content-Length: <bytes>\r\n\r\n<JSON payload>
```

This is identical to LSP's framing. It allows binary-safe message boundaries without requiring the JSON parser to handle framing.

**Startup:**
```
$ lucky serve --transport stdio
[Server listens on stdin, writes to stdout]
```

**Client connection:**
```
$ claude-code --ltp "ltp+stdio://lucky serve"
[Claude Code spawns 'lucky serve' as subprocess, communicates via stdin/stdout]
```

### 3.3 HTTP Transport

Messages are HTTP POST requests with JSON bodies.

**Endpoint:** `POST /ltp/v1`

```
POST /ltp/v1 HTTP/1.1
Host: localhost:9700
Content-Type: application/json
Authorization: Bearer <token>
Ltp-Session-Id: <session-uuid>

{
  "jsonrpc": "2.0",
  "method": "execution/start",
  "params": { ... },
  "id": 1
}
```

**Response:**
```
HTTP/1.1 200 OK
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": 1
}
```

**Streaming:** For methods that produce streams, the server responds with `Transfer-Encoding: chunked` and sends Server-Sent Events (SSE):

```
POST /ltp/v1/stream HTTP/1.1
...

HTTP/1.1 200 OK
Content-Type: text/event-stream

event: execution/event
data: {"type": "node_started", "node_id": "n2", ...}

event: execution/event
data: {"type": "node_completed", "node_id": "n2", ...}
```

### 3.4 WebSocket Transport

Persistent bidirectional connection.

**Connect:** `ws://localhost:9700/ltp/v1`

Messages are JSON-RPC objects sent as WebSocket text frames.

```
Client → Server: {"jsonrpc":"2.0","method":"execution/start","params":{...},"id":1}
Server → Client: {"jsonrpc":"2.0","result":{...},"id":1}
Server → Client: {"jsonrpc":"2.0","method":"execution/event","params":{"type":"node_started",...}}
```

### 3.5 In-Process Transport

Direct function calls within the same process. Used for embedding the Lucky runtime in another application.

```rust
// Rust example
let client = LtpClient::new_inprocess(server);
let result = client.call("execution/start", params).await?;
```

### 3.6 Transport Selection

The client chooses the transport:

```
# stdio
lucky serve

# HTTP
lucky serve --transport http --port 9700

# WebSocket
lucky serve --transport ws --port 9700

# All transports
lucky serve --transport http,ws,stdio --port 9700
```

---

## Chapter 4 -- Message Format

### 4.1 JSON-RPC 2.0 Compliance

LTP messages are valid JSON-RPC 2.0 with LTP-specific `method` names and `params` schemas.

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "method/name",
  "params": { ... },
  "id": 1
}
```

- `method`: LTP method name (see method reference).
- `params`: Method-specific parameters (structured object, not positional array).
- `id`: Request identifier (integer or string). Used to match responses.

#### Response (Success)

```json
{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": 1
}
```

#### Response (Error)

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Task not found",
    "data": { "task_id": "abc-123" }
  },
  "id": 1
}
```

#### Notification (no response expected)

```json
{
  "jsonrpc": "2.0",
  "method": "notification/name",
  "params": { ... }
}
```

### 4.2 LTP-Specific Extensions

LTP adds these extensions to JSON-RPC 2.0:

#### Batch Requests

Multiple requests sent as a JSON array. The server processes them in order and returns an array of responses.

```json
[
  {"jsonrpc": "2.0", "method": "ir/load", "params": {...}, "id": 1},
  {"jsonrpc": "2.0", "method": "execution/start", "params": {...}, "id": 2}
]
```

#### Partial Results (Streaming)

For long-running operations, the server may send partial results before the final response:

```
Server → Client (notification):
{
  "jsonrpc": "2.0",
  "method": "execution/progress",
  "params": {
    "request_id": 1,
    "progress": 0.45,
    "message": "Executing task: AnalyzeRepo"
  }
}

... later ...

Server → Client (final response):
{
  "jsonrpc": "2.0",
  "result": { "status": "completed" },
  "id": 1
}
```

#### Cancellation

```json
{
  "jsonrpc": "2.0",
  "method": "execution/cancel",
  "params": {
    "request_id": 1
  },
  "id": 2
}
```

### 4.3 Common Parameter Patterns

#### IR Reference

```json
{
  "ir": "<inline JSON IR>"       // inline IR
}
```
```json
{
  "ir_uri": "file:///path/to/program.lir"  // IR by URI
}
```
```json
{
  "ir_hash": "sha256:abcdef..."  // IR by content hash (server cache)
}
```

#### Execution Context

```json
{
  "context": {
    "user": "alice@example.com",
    "repo": "https://github.com/org/repo",
    "environment": "staging"
  }
}
```

#### Model Selection

```json
{
  "model": "Claude",
  "model_config": {
    "temperature": 0.7,
    "max_tokens": 4096
  }
}
```

#### Policy Override

```json
{
  "policy": {
    "retry": 3,
    "timeout_ms": 600000,
    "cost_limit_usd": 5.0
  }
}
```

---

## Chapter 5 -- Session Lifecycle

### 5.1 Session State Machine

```
                 ┌──────────┐
                 │  Closed   │
                 └─────┬────┘
                       │ initialize
                       ▼
                 ┌──────────┐
          ┌──────│   Idle    │
          │      └─────┬────┘
          │            │ ir/load
          │            ▼
          │      ┌──────────┐
          │      │  Loaded   │
          │      └─────┬────┘
          │            │ execution/start
          │            ▼
          │      ┌──────────┐
          │      │  Running  │──────┐
          │      └─────┬────┘      │ execution/pause
          │            │           ▼
          │            │      ┌──────────┐
          │            │      │  Paused   │
          │            │      └─────┬────┘
          │            │           │ execution/resume
          │            │           ▼
          │            │      ┌──────────┐
          │            ├──────│ Completed │
          │            │      └──────────┘
          │            │ execution/cancel
          │            ▼
          │      ┌──────────┐
          │      │ Cancelled │
          │      └──────────┘
          │ session/close
          ▼
      ┌──────────┐
      │  Closed   │
      └──────────┘
```

### 5.2 Session Initialization

```
Client → Server:
{
  "jsonrpc": "2.0",
  "method": "session/initialize",
  "params": {
    "protocol_version": "0.1",
    "client_info": {
      "name": "Claude Code",
      "version": "2.0.0",
      "platform": "darwin-arm64"
    },
    "capabilities": {
      "streaming": true,
      "batch": true,
      "human_approval": true,
      "checkpoint_restore": true
    },
    "auth": {
      "type": "bearer",
      "token": "ltp-token-xyz"
    }
  },
  "id": 1
}
```

```
Server → Client:
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "protocol_version": "0.1",
    "server_info": {
      "name": "Lucky Runtime",
      "version": "0.1.0",
      "platform": "linux-x86_64"
    },
    "capabilities": {
      "streaming": true,
      "batch": true,
      "human_approval": true,
      "checkpoint_restore": true,
      "max_concurrency": 16,
      "supported_models": ["Claude", "GPT", "Gemini", "Local"],
      "supported_tools": ["Filesystem", "Git", "Browser", "Shell", "HTTP", "Database", "Search"],
      "supported_ir_levels": ["high", "mid", "low"],
      "cost_tracking": true,
      "distributed_execution": false
    }
  },
  "id": 1
}
```

### 5.3 Session Capabilities Negotiation

Both client and server declare capabilities. The effective capability is the intersection:

```
effective = client_caps ∩ server_caps

If client wants streaming but server doesn't support it:
  → Streaming is disabled for this session.
  → Server sends a warning in the initialize response.
```

### 5.4 Session Termination

```
Client → Server:
{
  "jsonrpc": "2.0",
  "method": "session/close",
  "params": {
    "reason": "workflow_completed"
  },
  "id": 999
}
```

The server responds, then closes the transport. The server should:
1. Cancel any running executions (or wait for completion if `reason` is `workflow_completed`).
2. Finalize all checkpoints.
3. Release resources.
4. Send the close response.
5. Close the transport.

### 5.5 Session Heartbeat

For long-running sessions over HTTP/WS, the client should send periodic heartbeats:

```
Client → Server (notification):
{
  "jsonrpc": "2.0",
  "method": "session/heartbeat",
  "params": {}
}
```

If the server does not receive a heartbeat within the configured timeout (default: 60s), it may close the session.

---

## Chapter 6 -- Error Model

### 6.1 Error Code Ranges

LTP uses JSON-RPC 2.0 error codes with LTP-specific ranges:

| Range | Category |
|---|---|
| -32768 to -32000 | JSON-RPC reserved |
| -32001 to -31000 | LTP protocol errors |
| -30999 to -30000 | LTP execution errors |
| -29999 to -29000 | LTP security errors |
| -28999 to -28000 | LTP resource errors |
| -1 to -100 | Server-defined (avoid these) |

### 6.2 Standard LTP Error Codes

| Code | Name | Description |
|---|---|---|
| -32001 | `SESSION_NOT_FOUND` | Invalid or expired session ID |
| -32002 | `SESSION_CLOSED` | Session has been closed |
| -32003 | `INVALID_STATE` | Operation not valid in current session state |
| -32004 | `IR_INVALID` | IR failed validation |
| -32005 | `IR_NOT_FOUND` | Referenced IR not loaded |
| -32006 | `EXECUTION_NOT_FOUND` | Referenced execution not found |
| -32007 | `EXECUTION_ALREADY_RUNNING` | Execution already in progress |
| -32008 | `METHOD_NOT_FOUND` | Unknown LTP method |
| -32009 | `INVALID_PARAMS` | Invalid method parameters |
| -32010 | `INTERNAL_ERROR` | Internal server error |
| -31001 | `TASK_FAILED` | Task execution failed |
| -31002 | `WORKFLOW_FAILED` | Workflow execution failed |
| -31003 | `GOAL_FAILED` | Goal verification failed |
| -31004 | `APPROVAL_REJECTED` | Human rejected the approval |
| -31005 | `APPROVAL_TIMEOUT` | Human approval timed out |
| -31006 | `CANCELLED` | Execution was cancelled |
| -30001 | `AUTH_REQUIRED` | Authentication required |
| -30002 | `AUTH_INVALID` | Invalid credentials |
| -30003 | `PERMISSION_DENIED` | Insufficient permissions |
| -30004 | `RATE_LIMITED` | Too many requests |
| -29001 | `BUDGET_EXCEEDED` | Cost budget exceeded |
| -29002 | `RESOURCE_EXHAUSTED` | Memory/CPU/disk exhausted |
| -29003 | `BACKEND_UNAVAILABLE` | LLM backend unreachable |
| -29004 | `TOOL_UNAVAILABLE` | Required tool not available |

### 6.3 Error Response Format

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -31001,
    "message": "Task execution failed: AnalyzeRepo",
    "data": {
      "execution_id": "exec-abc-123",
      "node_id": "n5",
      "task_ref": "AnalyzeRepo",
      "error_code": 40,
      "error_message": "Tool execution failed: git clone returned exit code 128",
      "recoverable": true,
      "recovery_attempted": true,
      "recovery_result": "exhausted",
      "stack": [
        {
          "node_id": "n5",
          "node_kind": "task",
          "step_index": 2,
          "agent_ref": "Researcher"
        }
      ]
    }
  },
  "id": 1
}
```

### 6.4 Error Recovery Hints

The `data.recoverable` field indicates whether the client can retry. The `data.recovery_attempted` field indicates server-side recovery was already tried.

---

# Part II -- Core RPC Methods

---

## Chapter 7 -- Session Methods

### 7.1 session/initialize

Initialize a new LTP session.

```
Request:
{
  "method": "session/initialize",
  "params": {
    "protocol_version": "0.1",
    "client_info": { "name": "...", "version": "...", "platform": "..." },
    "capabilities": { ... },
    "auth": { "type": "bearer", "token": "..." }
  }
}

Response:
{
  "session_id": "uuid",
  "protocol_version": "0.1",
  "server_info": { "name": "Lucky Runtime", "version": "0.1.0" },
  "capabilities": { ... }
}
```

### 7.2 session/close

Close the session.

```
Request:
{
  "method": "session/close",
  "params": {
    "reason": "workflow_completed" | "user_requested" | "error" | "timeout"
  }
}

Response: { "acknowledged": true }
```

### 7.3 session/heartbeat (Notification)

```
{
  "method": "session/heartbeat",
  "params": {}
}
```

### 7.4 session/get_status

Get current session status.

```
Request: { "method": "session/get_status" }
Response: {
  "session_id": "uuid",
  "state": "idle" | "loaded" | "running" | "paused" | "completed",
  "uptime_seconds": 3600,
  "loaded_ir_hash": "sha256:...",
  "active_executions": ["exec-1", "exec-2"],
  "cost": {
    "total_usd": 1.23,
    "by_model": { "Claude": 0.80, "GPT": 0.43 }
  }
}
```

---

## Chapter 8 -- IR Methods

### 8.1 ir/load

Load a Lucky IR program into the session.

```
Request:
{
  "method": "ir/load",
  "params": {
    "ir": { ... },               // inline IR (JSON)
    // OR:
    "ir_uri": "file:///path/to/program.lir",
    // OR:
    "ir_hash": "sha256:abcdef...",

    "options": {
      "validate": true,          // validate IR before accepting (default: true)
      "optimize": true,          // run optimization passes (default: true)
      "optimization_level": "O2" // "O0" | "O1" | "O2" | "O3"
    }
  }
}

Response:
{
  "ir_hash": "sha256:abcdef...",
  "validation": {
    "valid": true,
    "warnings": [
      { "code": "W001", "message": "Node n7 has no estimated cost" }
    ]
  },
  "metadata": {
    "project_name": "MyProject",
    "source_file": "main.lk",
    "compiled_at": "2026-07-02T10:30:00Z",
    "entry_points": [
      { "kind": "goal", "name": "BuildWebsite", "workflows": ["MainWorkflow"] }
    ],
    "node_count": 15,
    "edge_count": 18
  }
}
```

### 8.2 ir/unload

Unload the current IR. Fails if an execution is running.

```
Request: { "method": "ir/unload" }
Response: { "acknowledged": true }
```

### 8.3 ir/validate

Validate IR without loading it for execution. Useful for pre-flight checks.

```
Request:
{
  "method": "ir/validate",
  "params": {
    "ir": { ... }
  }
}

Response:
{
  "valid": true | false,
  "errors": [
    {
      "code": "E001",
      "message": "Data edge n3→n4 type mismatch: String vs Int",
      "location": { "from": "n3", "to": "n4", "port": "data" }
    }
  ],
  "warnings": [ ... ],
  "stats": {
    "node_count": 15,
    "critical_path_nodes": 8,
    "estimated_cost_usd": 2.50,
    "estimated_duration_ms": 45000
  }
}
```

### 8.4 ir/optimize

Run optimization passes on loaded IR and return the optimized version.

```
Request:
{
  "method": "ir/optimize",
  "params": {
    "level": "O2",
    "passes": ["inline", "dce", "gvn"]  // specific passes (optional; overrides level)
  }
}

Response:
{
  "optimized_ir": { ... },       // the optimized IR (JSON)
  "changes": [
    { "pass": "dce", "nodes_removed": 3 },
    { "pass": "inline", "functions_inlined": 2 },
    { "pass": "gvn", "redundant_eliminated": 5 }
  ],
  "cost_savings": {
    "estimated_usd_before": 2.50,
    "estimated_usd_after": 1.80,
    "savings_usd": 0.70,
    "savings_percent": 28.0
  }
}
```

---

## Chapter 9 -- Execution Methods

### 9.1 execution/start

Start executing a loaded IR program.

```
Request:
{
  "method": "execution/start",
  "params": {
    "entry_point": "BuildWebsite",       // goal or workflow name
    "entry_kind": "goal",                // "goal" | "workflow" | "task"
    "context": {
      "user": "alice@example.com",
      "repo": "https://github.com/org/repo.git",
      "branch": "main"
    },
    "model_overrides": {
      "Planner": "GPT",
      "Reviewer": "Claude"
    },
    "policy_override": {
      "cost_limit_usd": 10.0,
      "max_concurrency": 8,
      "checkpoint_interval_ms": 300000
    },
    "mode": "sync" | "async"
  }
}

Response (mode: "sync"):
{
  "execution_id": "exec-abc-123",
  "status": "completed",
  "result": "success" | "failure",
  "outputs": {
    "website": { "url": "https://...", "deployed": true }
  },
  "cost": {
    "total_usd": 3.45,
    "by_model": { "Claude": 3.00, "GPT": 0.45 },
    "tokens_total": 8500
  },
  "duration_ms": 124000,
  "checkpoints": ["ckp-1", "ckp-2"]
}

Response (mode: "async"):
{
  "execution_id": "exec-abc-123",
  "status": "running"
}
# Server then sends execution/event notifications as the workflow progresses.
```

### 9.2 execution/get_status

Query the status of an execution.

```
Request:
{
  "method": "execution/get_status",
  "params": { "execution_id": "exec-abc-123" }
}

Response:
{
  "execution_id": "exec-abc-123",
  "status": "running" | "completed" | "failed" | "cancelled" | "paused",
  "progress": 0.45,
  "current_node": "n5",
  "current_node_label": "AnalyzeRepo",
  "started_at": "2026-07-02T10:30:00Z",
  "elapsed_ms": 60000,
  "estimated_remaining_ms": 72000,
  "node_states": {
    "n1": "completed",
    "n2": "completed",
    "n3": "completed",
    "n4": "completed",
    "n5": "running",
    "n6": "ready",
    "n7": "pending"
  },
  "cost": { "total_usd": 1.50 }
}
```

### 9.3 execution/pause

Pause a running execution.

```
Request:
{
  "method": "execution/pause",
  "params": {
    "execution_id": "exec-abc-123",
    "checkpoint": true          // also create a checkpoint (default: true)
  }
}

Response:
{
  "status": "paused",
  "checkpoint_id": "ckp-3"     // if checkpoint was created
}
```

Pausing waits for the current node to reach a safe point (checkpoint boundary) before suspending.

### 9.4 execution/resume

Resume a paused execution.

```
Request:
{
  "method": "execution/resume",
  "params": {
    "execution_id": "exec-abc-123",
    "from_checkpoint": "ckp-3" // optional; defaults to latest
  }
}

Response:
{
  "status": "running"
}
```

### 9.5 execution/cancel

Cancel a running or paused execution.

```
Request:
{
  "method": "execution/cancel",
  "params": {
    "execution_id": "exec-abc-123",
    "reason": "User requested cancellation"
  }
}

Response:
{
  "status": "cancelled",
  "checkpoint_id": "ckp-4"     // auto-checkpoint before cancellation
}
```

### 9.6 execution/list

List all executions in the current session.

```
Request: { "method": "execution/list" }

Response:
{
  "executions": [
    {
      "execution_id": "exec-abc-123",
      "entry_point": "BuildWebsite",
      "status": "completed",
      "started_at": "...",
      "completed_at": "...",
      "cost_usd": 3.45
    },
    {
      "execution_id": "exec-def-456",
      "entry_point": "MainWorkflow",
      "status": "running",
      "started_at": "..."
    }
  ]
}
```

---

## Chapter 10 -- Approval Methods

### 10.1 approval/request (Notification: Server → Client)

When an execution reaches an `approval` node, the server sends this notification:

```
{
  "jsonrpc": "2.0",
  "method": "approval/request",
  "params": {
    "approval_id": "appr-001",
    "execution_id": "exec-abc-123",
    "node_id": "n10",
    "gate": "before deploy",
    "message": "Deploy version 2.3.1 to production?",
    "context": {
      "changelog": "- Fix: null pointer in auth\n- Feat: add rate limiting",
      "risk_assessment": "medium",
      "affected_services": ["api", "web"],
      "rollback_plan": "Revert to version 2.3.0 via `lucky rollback`"
    },
    "options": [
      { "value": "approve", "label": "Approve deployment" },
      { "value": "reject", "label": "Reject deployment" },
      { "value": "modify", "label": "Modify and deploy" }
    ],
    "timeout_ms": 14400000,     // 4 hours
    "required_roles": ["devops", "sre"]
  }
}
```

### 10.2 approval/respond

Client responds to an approval request.

```
Request:
{
  "method": "approval/respond",
  "params": {
    "approval_id": "appr-001",
    "decision": "approve" | "reject" | "modify",
    "reason": "Changes look good, proceed.",
    "modifications": {           // if decision == "modify"
      "max_concurrency": 4,
      "additional_checks": ["run_smoke_tests"]
    }
  }
}

Response:
{
  "acknowledged": true
}
```

### 10.3 approval/list

List pending approvals for the session.

```
Request: { "method": "approval/list" }

Response:
{
  "pending": [
    {
      "approval_id": "appr-001",
      "execution_id": "exec-abc-123",
      "gate": "before deploy",
      "message": "Deploy version 2.3.1 to production?",
      "created_at": "...",
      "timeout_at": "...",
      "time_remaining_ms": 13200000
    }
  ]
}
```

---

## Chapter 11 -- Checkpoint Methods

### 11.1 checkpoint/create

Create a manual checkpoint of an execution.

```
Request:
{
  "method": "checkpoint/create",
  "params": {
    "execution_id": "exec-abc-123",
    "label": "Pre-deploy snapshot"
  }
}

Response:
{
  "checkpoint_id": "ckp-5",
  "execution_id": "exec-abc-123",
  "label": "Pre-deploy snapshot",
  "created_at": "2026-07-02T11:30:00Z",
  "size_bytes": 245760,
  "node_progress": {
    "completed": 10,
    "active": 1,
    "pending": 5
  }
}
```

### 11.2 checkpoint/restore

Restore an execution from a checkpoint.

```
Request:
{
  "method": "checkpoint/restore",
  "params": {
    "execution_id": "exec-abc-123",
    "checkpoint_id": "ckp-5"
  }
}

Response:
{
  "execution_id": "exec-abc-123",
  "status": "paused",          // execution is paused after restore
  "restored_from": "ckp-5"
}
```

The client must then call `execution/resume` to continue execution.

### 11.3 checkpoint/list

List checkpoints for an execution.

```
Request:
{
  "method": "checkpoint/list",
  "params": { "execution_id": "exec-abc-123" }
}

Response:
{
  "checkpoints": [
    {
      "checkpoint_id": "ckp-1",
      "label": "After Research",
      "created_at": "...",
      "size_bytes": 102400,
      "node_progress": { "completed": 3, "active": 0, "pending": 12 }
    },
    {
      "checkpoint_id": "ckp-2",
      "label": "After Plan",
      "created_at": "...",
      "size_bytes": 204800,
      "node_progress": { "completed": 6, "active": 0, "pending": 9 }
    }
  ]
}
```

### 11.4 checkpoint/delete

Delete a checkpoint.

```
Request:
{
  "method": "checkpoint/delete",
  "params": {
    "execution_id": "exec-abc-123",
    "checkpoint_id": "ckp-1"
  }
}

Response: { "acknowledged": true }
```

---

## Chapter 12 -- Query Methods

### 12.1 query/cost

Get cost information.

```
Request:
{
  "method": "query/cost",
  "params": {
    "execution_id": "exec-abc-123",   // optional; session-level if omitted
    "detail": "summary" | "by_model" | "by_node" | "full"
  }
}

Response:
{
  "total_usd": 3.45,
  "by_model": {
    "Claude": { "cost_usd": 3.00, "tokens_prompt": 5000, "tokens_completion": 2000 },
    "GPT": { "cost_usd": 0.45, "tokens_prompt": 1000, "tokens_completion": 500 }
  },
  "by_node": [
    { "node_id": "n2", "label": "Research", "cost_usd": 1.20 },
    { "node_id": "n3", "label": "Plan", "cost_usd": 0.80 }
  ],
  "budget": {
    "limit_usd": 10.0,
    "remaining_usd": 6.55,
    "consumed_percent": 34.5
  }
}
```

### 12.2 query/context

Get the execution context at any point.

```
Request:
{
  "method": "query/context",
  "params": {
    "execution_id": "exec-abc-123",
    "node_id": "n5"                  // optional; current context if omitted
  }
}

Response:
{
  "context": {
    "user": "alice@example.com",
    "repo": "https://github.com/org/repo.git",
    "branch": "main",
    "n2.output.result": { ... },
    "n3.output.plan": { ... }
  }
}
```

### 12.3 query/artifact

Retrieve an execution artifact.

```
Request:
{
  "method": "query/artifact",
  "params": {
    "execution_id": "exec-abc-123",
    "node_id": "n5",
    "artifact_name": "report"
  }
}

Response:
{
  "artifact_id": "art-001",
  "kind": "text/markdown",
  "name": "AnalyzeRepo report",
  "size_bytes": 15360,
  "checksum": "sha256:def...",
  "content": "# Analysis Report\n\n..."     // inline for text artifacts
}
```

For large binary artifacts, the response includes a `content_uri` instead of `content`:

```json
{
  "artifact_id": "art-002",
  "kind": "application/pdf",
  "size_bytes": 2048000,
  "content_uri": "ltp+http://server:9700/artifacts/art-002",
  "content": null
}
```

### 12.4 query/node

Get detailed information about a specific node.

```
Request:
{
  "method": "query/node",
  "params": {
    "execution_id": "exec-abc-123",
    "node_id": "n5"
  }
}

Response:
{
  "node_id": "n5",
  "kind": "task",
  "label": "AnalyzeRepo",
  "status": "completed",
  "agent": "Researcher",
  "started_at": "...",
  "completed_at": "...",
  "duration_ms": 35000,
  "retries": 0,
  "inputs": { "repo": "https://...", "depth": 2 },
  "outputs": { "report": { "artifact_id": "art-001" } },
  "cost": { "usd": 0.80, "tokens": 3000 },
  "checkpoint_ids": ["ckp-1"]
}
```

### 12.5 query/tools

List tools available on the server.

```
Request: { "method": "query/tools" }

Response:
{
  "tools": [
    {
      "name": "Filesystem",
      "description": "File system operations",
      "methods": [
        { "name": "read", "description": "Read a file", "parameters": { ... } },
        { "name": "write", "description": "Write a file", "parameters": { ... } }
      ]
    },
    ...
  ]
}
```

### 12.6 query/models

List models available on the server.

```
Request: { "method": "query/models" }

Response:
{
  "models": [
    {
      "name": "Claude",
      "provider": "anthropic",
      "version": "claude-sonnet-4-20250514",
      "context_window": 200000,
      "max_output_tokens": 4096,
      "cost_per_1k_prompt_tokens": 0.003,
      "cost_per_1k_completion_tokens": 0.015,
      "supports_vision": true,
      "supports_tools": true
    },
    ...
  ]
}
```

---

# Part III -- Streaming & Events

---

## Chapter 13 -- Event Stream

### 13.1 Event Delivery

Events are delivered as JSON-RPC notifications from server to client.

```
{
  "jsonrpc": "2.0",
  "method": "execution/event",
  "params": {
    "execution_id": "exec-abc-123",
    "sequence": 42,                   // monotonically increasing
    "timestamp": "2026-07-02T10:30:05.123Z",
    "type": "event_type",
    "data": { ... }
  }
}
```

### 13.2 Event Types

#### execution.started

```json
{
  "type": "execution.started",
  "data": {
    "entry_point": "BuildWebsite",
    "entry_kind": "goal",
    "workflow": "MainWorkflow",
    "context": { ... }
  }
}
```

#### node.started

```json
{
  "type": "node.started",
  "data": {
    "node_id": "n5",
    "kind": "task",
    "label": "AnalyzeRepo",
    "agent": "Researcher",
    "estimated_cost_usd": 0.80,
    "estimated_duration_ms": 30000
  }
}
```

#### node.progress

```json
{
  "type": "node.progress",
  "data": {
    "node_id": "n5",
    "step_index": 2,
    "step_label": "analyze_structure",
    "step_total": 4,
    "message": "Analyzing code structure..."
  }
}
```

#### node.completed

```json
{
  "type": "node.completed",
  "data": {
    "node_id": "n5",
    "duration_ms": 35000,
    "cost_usd": 0.80,
    "outputs": { "report": { "artifact_id": "art-001" } },
    "retries": 0
  }
}
```

#### node.failed

```json
{
  "type": "node.failed",
  "data": {
    "node_id": "n5",
    "error": {
      "code": 40,
      "message": "Tool execution failed",
      "recoverable": true
    },
    "recovery_action": "retry",
    "recovery_attempt": 1,
    "recovery_max": 3
  }
}
```

#### node.retrying

```json
{
  "type": "node.retrying",
  "data": {
    "node_id": "n5",
    "attempt": 2,
    "max_attempts": 3,
    "delay_ms": 4000,
    "reason": "Transient error; retrying with exponential backoff"
  }
}
```

#### llm.token (Streaming)

```json
{
  "type": "llm.token",
  "data": {
    "node_id": "n5",
    "model": "Claude",
    "token": " the",
    "sequence": 142
  }
}
```

#### llm.completed

```json
{
  "type": "llm.completed",
  "data": {
    "node_id": "n5",
    "model": "Claude",
    "tokens_prompt": 1200,
    "tokens_completion": 350,
    "cost_usd": 0.005,
    "duration_ms": 2800,
    "finish_reason": "stop"
  }
}
```

#### checkpoint.created

```json
{
  "type": "checkpoint.created",
  "data": {
    "checkpoint_id": "ckp-2",
    "node_id": "n5",
    "trigger": "after_task",
    "size_bytes": 204800
  }
}
```

#### approval.requested

```json
{
  "type": "approval.requested",
  "data": {
    "approval_id": "appr-001",
    "node_id": "n10",
    "gate": "before deploy",
    "message": "Deploy to production?",
    "timeout_ms": 14400000
  }
}
```

#### cost.updated

```json
{
  "type": "cost.updated",
  "data": {
    "total_usd": 2.35,
    "last_node_cost_usd": 0.50,
    "budget_remaining_usd": 7.65,
    "budget_percent": 23.5
  }
}
```

#### execution.paused

```json
{
  "type": "execution.paused",
  "data": {
    "reason": "user_requested",
    "checkpoint_id": "ckp-3"
  }
}
```

#### execution.resumed

```json
{
  "type": "execution.resumed",
  "data": {
    "from_checkpoint": "ckp-3"
  }
}
```

#### execution.completed

```json
{
  "type": "execution.completed",
  "data": {
    "result": "success",
    "outputs": { ... },
    "total_cost_usd": 3.45,
    "total_duration_ms": 124000,
    "nodes_completed": 15,
    "nodes_failed": 0,
    "nodes_skipped": 0
  }
}
```

#### execution.failed

```json
{
  "type": "execution.failed",
  "data": {
    "result": "failure",
    "error": { "code": -31002, "message": "Workflow execution failed" },
    "failed_node": "n7",
    "total_cost_usd": 1.20,
    "recovery_exhausted": true
  }
}
```

### 13.3 Event Filtering

Clients can subscribe to specific event types:

```
Request:
{
  "method": "execution/start",
  "params": {
    ...
    "event_filter": ["node.started", "node.completed", "node.failed", "cost.updated"]
  }
}
```

If no filter is specified, all events are sent.

---

## Chapter 14 -- Notification Methods

### 14.1 Server → Client Notifications

These are unsolicited messages from server to client:

| Method | Description |
|---|---|
| `execution/event` | Execution event (see Chapter 13) |
| `approval/request` | Human approval required (see Chapter 10) |
| `session/error` | Non-fatal session error |
| `session/warning` | Warning message |

### 14.2 Client → Server Notifications

| Method | Description |
|---|---|
| `session/heartbeat` | Keep-alive |
| `session/cancel_request` | Cancel a pending request by ID |

---

## Chapter 15 -- Progress Reporting

### 15.1 Progress Tokens

Long-running requests can include a `progress_token`:

```
Request:
{
  "method": "execution/start",
  "params": {
    ...
    "progress_token": "my-progress-token-42"
  }
}
```

The server sends progress notifications referencing this token:

```
Server → Client:
{
  "jsonrpc": "2.0",
  "method": "$/progress",
  "params": {
    "token": "my-progress-token-42",
    "value": {
      "kind": "percentage",
      "percentage": 45.0,
      "message": "Executing task 5 of 12"
    }
  }
}
```

### 15.2 Progress Value Kinds

| Kind | Description | Example |
|---|---|---|
| `percentage` | 0-100 completion | `{ "percentage": 45.0 }` |
| `steps` | Step counter | `{ "step": 3, "total": 8 }` |
| `message` | Text status | `{ "message": "Compiling..." }` |
| `token_count` | Tokens used | `{ "tokens": 1500, "max": 4096 }` |
| `cost` | Cost incurred | `{ "cost_usd": 1.50, "budget_usd": 5.0 }` |

---

# Part IV -- Tool Integration

---

## Chapter 16 -- Tool Registration

### 16.1 Tool Registration Model

Tools can be:
1. **Built-in**: Shipped with the Lucky runtime (Filesystem, Git, etc.).
2. **Server-registered**: Registered with the LTP server at startup.
3. **Dynamic**: Registered during a session by the client.

### 16.2 tool/register

Register a new tool with the server.

```
Request:
{
  "method": "tool/register",
  "params": {
    "name": "JiraClient",
    "description": "Interact with Jira issues",
    "transport": {
      "type": "stdio",
      "command": "jira-ltp-server",
      "args": ["--config", "jira.toml"],
      "env": { "JIRA_TOKEN": "secret" }
    },
    "methods": [
      {
        "name": "create_issue",
        "description": "Create a new Jira issue",
        "parameters": {
          "type": "object",
          "properties": {
            "project": { "type": "string", "description": "Project key" },
            "summary": { "type": "string", "description": "Issue summary" },
            "description": { "type": "string", "description": "Issue description" },
            "type": { "type": "string", "enum": ["bug", "task", "story"] }
          },
          "required": ["project", "summary", "type"]
        }
      },
      {
        "name": "search_issues",
        "description": "Search Jira issues by JQL",
        "parameters": {
          "type": "object",
          "properties": {
            "jql": { "type": "string", "description": "JQL query" },
            "max_results": { "type": "integer", "default": 10 }
          },
          "required": ["jql"]
        }
      }
    ]
  }
}

Response:
{
  "tool_id": "tool-jira-001",
  "status": "registered",
  "methods_registered": 2
}
```

### 16.3 tool/unregister

```
Request:
{
  "method": "tool/unregister",
  "params": { "tool_id": "tool-jira-001" }
}
```

### 16.4 Tool Provider Protocol

When the server needs to invoke a registered tool, it spawns the tool provider as a subprocess and communicates via LTP (sub-protocol):

```
Server → Tool Provider:
{
  "jsonrpc": "2.0",
  "method": "tool/invoke",
  "params": {
    "invocation_id": "inv-001",
    "tool": "JiraClient",
    "method": "create_issue",
    "arguments": {
      "project": "DEV",
      "summary": "Fix login bug",
      "type": "bug"
    },
    "context": {
      "session_id": "...",
      "agent": "Researcher"
    }
  },
  "id": 1
}

Tool Provider → Server:
{
  "jsonrpc": "2.0",
  "result": {
    "issue_key": "DEV-1234",
    "url": "https://jira.example.com/browse/DEV-1234"
  },
  "id": 1
}
```

---

## Chapter 17 -- Tool Discovery

### 17.1 tool/list

List all registered tools.

```
Request: { "method": "tool/list" }

Response:
{
  "tools": [
    {
      "tool_id": "builtin-filesystem",
      "name": "Filesystem",
      "kind": "builtin",
      "methods": ["read", "write", "list", "exists", "glob", ...]
    },
    {
      "tool_id": "tool-jira-001",
      "name": "JiraClient",
      "kind": "registered",
      "status": "ready",
      "methods": ["create_issue", "search_issues"]
    }
  ]
}
```

### 17.2 tool/describe

Get detailed information about a tool.

```
Request:
{
  "method": "tool/describe",
  "params": { "tool_id": "builtin-git" }
}

Response:
{
  "tool_id": "builtin-git",
  "name": "Git",
  "description": "Git version control operations",
  "kind": "builtin",
  "status": "ready",
  "methods": [
    {
      "name": "clone",
      "description": "Clone a repository",
      "parameters": {
        "type": "object",
        "properties": {
          "url": { "type": "string", "format": "uri" },
          "path": { "type": "string" },
          "branch": { "type": "string" }
        },
        "required": ["url"]
      }
    },
    ...
  ],
  "permissions_required": ["git.clone", "filesystem.write"],
  "cost_model": "free"
}
```

---

## Chapter 18 -- Tool Invocation

### 18.1 tool/invoke

Invoke a tool method directly (bypassing the execution DAG).

```
Request:
{
  "method": "tool/invoke",
  "params": {
    "tool_id": "builtin-git",
    "method": "status",
    "arguments": {
      "path": "./my-project"
    },
    "timeout_ms": 30000
  }
}

Response:
{
  "invocation_id": "inv-002",
  "result": {
    "branch": "main",
    "ahead": 0,
    "behind": 3,
    "staged": [],
    "unstaged": [
      { "path": "src/main.lk", "status": "modified" }
    ],
    "untracked": ["new-file.md"],
    "is_clean": false
  },
  "duration_ms": 450
}
```

### 18.2 Direct Tool Calls vs DAG Execution

Direct tool calls (via `tool/invoke`) are useful for:
- Pre-flight checks before starting an execution.
- Interactive exploration by the AI platform.
- Debugging and diagnostics.

During DAG execution, tool calls happen implicitly through `tool_invoke` IR instructions -- the client does not manually issue them.

---

## Chapter 19 -- Built-in Tools

### 19.1 Standard Built-in Tools

Every LTP server MUST implement these tools:

| Tool | Minimum Required Methods |
|---|---|
| `Filesystem` | `read`, `write`, `exists`, `list`, `glob` |
| `Shell` | `exec` |
| `HTTP` | `get`, `post` |

These tools MAY also be available:

| Tool | Description |
|---|---|
| `Git` | Version control |
| `Browser` | Web automation |
| `Database` | SQL queries |
| `Search` | Web search |
| `Memory` | Agent memory |
| `Knowledge` | RAG queries |

### 19.2 Tool Method Schemas

All tool methods use JSON Schema for parameter validation:

```json
{
  "name": "read",
  "parameters": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Path to the file to read, relative to the sandbox root"
      },
      "encoding": {
        "type": "string",
        "enum": ["utf8", "base64", "hex"],
        "default": "utf8"
      },
      "max_bytes": {
        "type": "integer",
        "description": "Maximum bytes to read",
        "default": 1048576
      }
    },
    "required": ["path"]
  },
  "returns": {
    "type": "object",
    "properties": {
      "content": { "type": "string" },
      "size_bytes": { "type": "integer" },
      "encoding": { "type": "string" }
    }
  }
}
```

---

# Part V -- Advanced Features

---

## Chapter 20 -- Multi-Session Coordination

### 20.1 Session Groups

Multiple LTP sessions can be grouped for coordination:

```
Request:
{
  "method": "session/create_group",
  "params": {
    "group_id": "my-workflow-group",
    "sessions": ["session-1", "session-2", "session-3"]
  }
}
```

### 20.2 Cross-Session Events

Sessions in a group can send events to each other:

```
Session A → Server:
{
  "method": "session/send_event",
  "params": {
    "target": "session-2",
    "event": "custom.data_ready",
    "data": { "artifact_id": "art-001" }
  }
}
```

---

## Chapter 21 -- Resource & Cost Controls

### 21.1 Budget Management

```
Request:
{
  "method": "session/set_budget",
  "params": {
    "total_usd": 50.0,
    "per_execution_usd": 10.0,
    "alert_thresholds": [0.5, 0.75, 0.9],
    "on_exceeded": "pause" | "cancel" | "notify"
  }
}
```

### 21.2 Rate Limiting

```
Request:
{
  "method": "session/set_rate_limits",
  "params": {
    "max_concurrent_llm_calls": 8,
    "max_requests_per_minute": 60,
    "max_tokens_per_minute": 100000
  }
}
```

### 21.3 Priority Control

```
Request:
{
  "method": "execution/set_priority",
  "params": {
    "execution_id": "exec-abc-123",
    "priority": "high" | "normal" | "low" | "background"
  }
}
```

---

## Chapter 22 -- Cancellation & Timeouts

### 22.1 Request Cancellation

```
Client → Server (notification):
{
  "jsonrpc": "2.0",
  "method": "$/cancel_request",
  "params": {
    "id": 42                    // request ID to cancel
  }
}
```

The server responds to the original request with:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32800,
    "message": "Request cancelled"
  },
  "id": 42
}
```

### 22.2 Execution Timeouts

Set a deadline for an entire execution:

```
Request:
{
  "method": "execution/start",
  "params": {
    ...
    "deadline_ms": 3600000     // 1 hour
  }
}
```

The server sends `execution.failed` with `DEADLINE_EXCEEDED` if the deadline is reached.

### 22.3 Node Timeouts

Node-level timeouts are specified in the IR (`timeout_ms` in resource requirements) and enforced by the scheduler.

---

## Chapter 23 -- Batch Operations

### 23.1 Batch Requests

Multiple requests can be sent as a JSON array:

```json
[
  {"jsonrpc": "2.0", "method": "query/cost", "params": {"execution_id": "exec-1"}, "id": 1},
  {"jsonrpc": "2.0", "method": "query/cost", "params": {"execution_id": "exec-2"}, "id": 2},
  {"jsonrpc": "2.0", "method": "query/context", "params": {"execution_id": "exec-1"}, "id": 3}
]
```

The server processes them in order and returns an array of responses. A batch is atomic for notifications but individual requests within a batch can fail independently.

### 23.2 Batch IR Loading

Load multiple IR modules at once:

```
Request:
{
  "method": "ir/load_batch",
  "params": {
    "modules": [
      { "name": "agents.coder", "ir": { ... } },
      { "name": "agents.reviewer", "ir": { ... } }
    ]
  }
}
```

---

# Part VI -- Platform Bindings

---

## Chapter 24 -- Claude Code Binding

### 24.1 Integration Model

Claude Code integrates with LTP as a **client**. It:
1. Compiles Lucky source to IR (or accepts pre-compiled IR).
2. Connects to a Lucky runtime via LTP.
3. Sends IR and receives execution events.
4. Handles human approvals through Claude Code's UI.

### 24.2 Configuration

```json
// .claude/settings.json
{
  "ltp": {
    "enabled": true,
    "server": {
      "transport": "stdio",
      "command": "lucky",
      "args": ["serve", "--transport", "stdio"]
    },
    "defaults": {
      "model": "Claude",
      "cost_limit_usd": 10.0,
      "auto_approve": ["filesystem.read", "git.status"]
    }
  }
}
```

### 24.3 Command Integration

```
# In Claude Code session:
> /lucky run main.lk
[Lucky] Compiling main.lk...
[Lucky] IR loaded. 15 nodes, 18 edges.
[Lucky] Starting execution: BuildWebsite
[Lucky] [n1] Research... ✓ (12s, $0.45)
[Lucky] [n2] Plan... ✓ (8s, $0.30)
[Lucky] [n3] Implement... ⏳
[Lucky] [n4] Test... ⏳
[Lucky] ⚠ Approval required: Deploy to production? [approve/reject]
```

### 24.4 MCP Tool Exposure

Claude Code can expose Lucky tools as MCP tools:

```json
// When LTP is connected, these MCP tools become available:
{
  "tools": [
    {
      "name": "lucky_execute",
      "description": "Execute a Lucky program",
      "inputSchema": { ... }
    },
    {
      "name": "lucky_status",
      "description": "Check Lucky execution status",
      "inputSchema": { ... }
    },
    {
      "name": "lucky_approve",
      "description": "Respond to a Lucky approval request",
      "inputSchema": { ... }
    }
  ]
}
```

---

## Chapter 25 -- Codex CLI Binding

### 25.1 Integration Model

Codex CLI integrates with LTP similarly, using its agent framework:

```
# codex.yaml
agents:
  lucky-runner:
    description: "Executes Lucky programs via LTP"
    tools:
      - lucky_execute
      - lucky_status
      - lucky_approve
    prompt: |
      You are a Lucky program executor. When asked to run a Lucky program:
      1. Use lucky_execute to start execution.
      2. Monitor progress with lucky_status.
      3. Handle approvals with lucky_approve.
```

### 25.2 Environment Setup

```bash
# Start LTP server
lucky serve --transport http --port 9700 &

# Configure Codex CLI
export CODX_LTP_ENDPOINT="http://localhost:9700/ltp/v1"
export CODX_LTP_TOKEN="ltp-token-xyz"

# Run
codex "execute the Lucky workflow in main.lk"
```

---

## Chapter 26 -- OpenCode Binding

### 26.1 Integration Model

OpenCode integrates LTP through its skill system:

```yaml
# skills/lucky-executor/SKILL.md
name: lucky-executor
description: Execute Lucky programs via the Lucky Tool Protocol

tools:
  - name: lucky_run
    description: Run a Lucky workflow
    parameters:
      ir_path: string
      goal: string
      context: object
```

### 26.2 OpenCode Skill Implementation

```python
# skills/lucky-executor/run.py
import json
import subprocess
import sys

def lucky_run(ir_path, goal, context):
    """Execute a Lucky program via LTP."""
    server = subprocess.Popen(
        ["lucky", "serve", "--transport", "stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True
    )

    # Initialize
    send_request(server, "session/initialize", { ... })
    response = read_response(server)

    # Load IR
    with open(ir_path) as f:
        ir = json.load(f)
    send_request(server, "ir/load", {"ir": ir})
    response = read_response(server)

    # Execute
    send_request(server, "execution/start", {
        "entry_point": goal,
        "context": context,
        "mode": "sync"
    })
    result = read_response(server)

    return result
```

---

## Chapter 27 -- Cursor Binding

### 27.1 Integration Model

Cursor integrates LTP as an extension:

```typescript
// cursor-ltp-extension/src/extension.ts
import * as vscode from 'vscode';
import { LtpClient } from './ltp-client';

export function activate(context: vscode.ExtensionContext) {
    const client = new LtpClient({
        transport: 'stdio',
        command: 'lucky',
        args: ['serve', '--transport', 'stdio']
    });

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('lucky.run', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;

            const ir = await compileToIR(editor.document);
            await client.initialize();
            await client.loadIR(ir);
            const result = await client.startExecution({
                entryPoint: 'MainWorkflow',
                mode: 'sync'
            });

            vscode.window.showInformationMessage(
                `Lucky execution ${result.result}: $${result.cost.total_usd}`
            );
        })
    );
}
```

### 27.2 Cursor UI Integration

- **Status bar**: Shows current Lucky execution status.
- **Output panel**: Streams Lucky execution events.
- **Problems panel**: Shows IR validation errors.
- **Notifications**: Approval requests appear as VS Code notifications with action buttons.

---

## Chapter 28 -- Dify Binding

### 28.1 Integration Model

Dify integrates LTP as a **Tool** in its workflow editor:

```yaml
# Dify tool definition
identity:
  name: lucky_executor
  author: Lucky Team
  label:
    en_US: Lucky Program Executor

parameters:
  - name: ir
    type: string
    required: true
    label:
      en_US: Lucky IR (JSON)
  - name: goal
    type: string
    required: true
    label:
      en_US: Goal to pursue
  - name: context
    type: object
    required: false

output:
  type: object
  properties:
    result:
      type: string
    cost_usd:
      type: number
    outputs:
      type: object
```

### 28.2 Dify Workflow Integration

Dify workflows can use Lucky as a step:

```
[Dify Workflow]
    │
    ├── [HTTP Request] → Fetch data
    │
    ├── [Lucky Executor] → Run analysis workflow
    │       input: { ir: $http_result.lucky_ir, goal: "Analyze" }
    │
    └── [LLM] → Summarize results
            input: { data: $lucky_result.outputs.report }
```

---

## Chapter 29 -- Standalone Runtime

### 29.1 lucky serve

The standalone runtime exposes LTP over all supported transports:

```bash
# Start LTP server
lucky serve \
  --transport http,ws,stdio \
  --port 9700 \
  --host 0.0.0.0 \
  --auth-token "ltp-token-xyz" \
  --max-concurrency 16 \
  --cost-limit-usd 50.0 \
  --checkpoint-dir ./.lucky/checkpoints \
  --log-level info \
  --log-format json
```

### 29.2 lucky CLI (Client)

```bash
# Run a Lucky program (starts server, loads IR, executes, exits)
lucky run main.lk --goal BuildWebsite

# Check status
lucky status --endpoint http://localhost:9700

# List executions
lucky list --endpoint http://localhost:9700

# Cancel execution
lucky cancel exec-abc-123 --endpoint http://localhost:9700

# Approve
lucky approve appr-001 --decision approve --reason "Looks good"

# Create checkpoint
lucky checkpoint create exec-abc-123 --label "Pre-deploy"

# Restore from checkpoint
lucky checkpoint restore exec-abc-123 ckp-5 --endpoint http://localhost:9700
```

### 29.3 lucky-client SDK

```rust
// Rust
use lucky_client::{LtpClient, LtpClientConfig};

let client = LtpClient::connect(LtpClientConfig {
    transport: Transport::Stdio,
    command: "lucky serve".into(),
}).await?;

client.initialize().await?;
client.load_ir_from_file("program.lir").await?;
let result = client.start_execution(StartExecutionParams {
    entry_point: "BuildWebsite".into(),
    mode: ExecutionMode::Sync,
    ..Default::default()
}).await?;

println!("Result: {:?}", result);
```

```python
# Python
from lucky_client import LtpClient

client = LtpClient.http("http://localhost:9700", token="ltp-token-xyz")
client.initialize()
client.load_ir_file("program.lir")
result = client.start_execution(
    entry_point="BuildWebsite",
    mode="sync"
)
print(f"Result: {result['result']}, Cost: ${result['cost']['total_usd']}")
```

```typescript
// TypeScript
import { LtpClient } from '@lucky-lang/ltp-client';

const client = new LtpClient({ transport: 'http', endpoint: 'http://localhost:9700' });
await client.initialize();
await client.loadIR(ir);
const result = await client.startExecution({
    entryPoint: 'BuildWebsite',
    mode: 'sync'
});
console.log(`Result: ${result.result}, Cost: $${result.cost.total_usd}`);
```

---

# Part VII -- Security

---

## Chapter 30 -- Authentication

### 30.1 Authentication Methods

LTP supports multiple authentication methods, negotiated during session initialization:

| Method | Description | Use Case |
|---|---|---|
| `none` | No authentication | Local development, testing |
| `bearer` | Bearer token in Authorization header | Production, API access |
| `basic` | HTTP Basic auth (username:password) | Legacy systems |
| `mtls` | Mutual TLS with client certificates | Enterprise, zero-trust |
| `oauth2` | OAuth 2.0 Bearer token | Cloud deployments |

### 30.2 Bearer Token Auth

```
Request:
{
  "method": "session/initialize",
  "params": {
    "auth": {
      "type": "bearer",
      "token": "ltp-token-v1-xyz-abc-123"
    }
  }
}
```

The server validates the token against its configured token store (file, environment variable, or external auth service).

### 30.3 Token Generation

```
$ lucky auth generate-token --name "claude-code-agent" --expires-in 90d
Token: ltp-token-v1-xyz-abc-123
Scopes: execution:start, execution:cancel, approval:respond
Expires: 2026-09-30T10:30:00Z
```

### 30.4 Token Scopes

| Scope | Permissions |
|---|---|
| `session:*` | Full session management |
| `ir:*` | Load, validate, unload IR |
| `execution:start` | Start new executions |
| `execution:cancel` | Cancel running executions |
| `execution:pause` | Pause/resume executions |
| `approval:respond` | Respond to approval requests |
| `checkpoint:*` | Create, restore, delete checkpoints |
| `query:*` | Read-only queries |
| `tool:invoke` | Direct tool invocation |
| `tool:register` | Register new tools |

---

## Chapter 31 -- Authorization & Permissions

### 31.1 Capability-Based Authorization

LTP inherits Lucky's capability-security model. Every operation within an execution is gated by permissions. The LTP server enforces:

1. **Session-level permissions**: What the client (AI platform) is allowed to do.
2. **Agent-level permissions**: What each agent within a Lucky program is allowed to do.
3. **Tool-level permissions**: What each tool invocation is allowed to do.

### 31.2 Permission Checking Flow

```
Client sends execution/start
  ↓
Server checks session permissions:
  - Does the client have execution:start scope?
  ↓
Execution DAG runs; agent tries to invoke tool
  ↓
Server checks agent permissions:
  - Does the agent have filesystem.write permission?
  - Is the target path within the agent's sandbox?
  ↓
If denied:
  - Node transitions to Failed with PERMISSION_DENIED error.
  - Recovery policy is invoked.
```

### 31.3 Permission Inheritance

Permissions set at session initialization are inherited by all executions in that session. Executions can further restrict but never expand permissions:

```
Session: allow filesystem.read, git.*; deny shell.*
├── Execution 1 (BuildWorkflow): allow filesystem.read; deny git.push
│   └── Agent Coder: allow filesystem.read; deny filesystem.write
└── Execution 2 (DeployWorkflow): allow git.push, shell.exec("deploy")
```

---

## Chapter 32 -- Transport Security

### 32.1 TLS

For HTTP and WebSocket transports, TLS MUST be used in production:

```
ltp+https://runtime.example.com:443/ltp/v1
ltp+wss://runtime.example.com:443/ltp/v1
```

Minimum TLS version: 1.3. Cipher suites: TLS_AES_256_GCM_SHA384, TLS_CHACHA20_POLY1305_SHA256.

### 32.2 Mutual TLS (mTLS)

For enterprise deployments, mTLS provides both authentication and encryption:

```
$ lucky serve \
  --transport https \
  --port 443 \
  --tls-cert server.crt \
  --tls-key server.key \
  --tls-ca ca.crt \
  --mtls required
```

### 32.3 Input Validation

All LTP messages are validated before processing:
- JSON is parsed with strict mode (no duplicate keys, no trailing garbage).
- String fields are checked for length limits (default max: 1MB per message).
- IR payloads are validated against the IR JSON Schema.
- Nested structures are checked for depth limits (max depth: 100).

### 32.4 Rate Limiting

The server enforces rate limits:
- Max requests per second per session (configurable, default: 100).
- Max concurrent executions per session (configurable, default: 4).
- Max IR size (configurable, default: 100MB).
- Max event stream connections per session (default: 1).

---

## Chapter 33 -- Sandbox Integration

### 33.1 Sandbox Levels

The LTP server runs tool invocations within sandboxes:

| Level | Description | Configuration |
|---|---|---|
| `none` | Direct execution (development only) | `--sandbox none` |
| `process` | Separate subprocess | `--sandbox process` |
| `docker` | Docker container | `--sandbox docker --sandbox-image lucky-sandbox:latest` |
| `firecracker` | microVM | `--sandbox firecracker --sandbox-kernel vmlinux.bin` |

### 33.2 Filesystem Sandboxing

```
Sandbox root: /tmp/lucky-sandbox-abc123/

Agent's view:
  / → /tmp/lucky-sandbox-abc123/
  /project → /tmp/lucky-sandbox-abc123/project/  (agent's working dir)

Allowed:
  read("project/src/main.lk")  → resolved to sandbox path, checked against permissions

Denied:
  read("/etc/passwd")           → escapes sandbox root, blocked
  read("../../secrets/token")   → resolves outside sandbox, blocked
```

### 33.3 Network Sandboxing

```
Default policy: deny all outbound

Allowed:
  - HTTP GET to api.example.com (from agent permissions)
  - Git clone from github.com (from agent permissions)

Denied:
  - HTTP POST to arbitrary hosts
  - Connections to internal IPs (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
  - DNS rebinding attacks (IP validation after resolution)
```

---

# Appendix A -- Complete Method Reference

```
Method                          | Direction     | Description
────────────────────────────────┼───────────────┼──────────────────────────────────
session/initialize              | C→S           | Initialize LTP session
session/close                   | C→S           | Close session
session/heartbeat               | C→S           | Keep-alive (notification)
session/get_status              | C→S           | Get session status
session/create_group            | C→S           | Create session group
session/send_event              | C→S           | Send cross-session event
session/set_budget              | C→S           | Set cost budget
session/set_rate_limits         | C→S           | Set rate limits
session/error                   | S→C           | Non-fatal error (notification)
session/warning                 | S→C           | Warning (notification)
────────────────────────────────┼───────────────┼──────────────────────────────────
ir/load                         | C→S           | Load IR program
ir/load_batch                   | C→S           | Load multiple IR modules
ir/unload                       | C→S           | Unload IR
ir/validate                     | C→S           | Validate IR without loading
ir/optimize                     | C→S           | Run optimization passes
────────────────────────────────┼───────────────┼──────────────────────────────────
execution/start                 | C→S           | Start execution
execution/get_status            | C→S           | Get execution status
execution/pause                 | C→S           | Pause execution
execution/resume                | C→S           | Resume execution
execution/cancel                | C→S           | Cancel execution
execution/list                  | C→S           | List executions
execution/set_priority          | C→S           | Set execution priority
execution/event                 | S→C           | Execution event (notification)
execution/progress              | S→C           | Progress update (notification)
────────────────────────────────┼───────────────┼──────────────────────────────────
approval/request                | S→C           | Approval required (notification)
approval/respond                | C→S           | Respond to approval
approval/list                   | C→S           | List pending approvals
────────────────────────────────┼───────────────┼──────────────────────────────────
checkpoint/create               | C→S           | Create manual checkpoint
checkpoint/restore              | C→S           | Restore from checkpoint
checkpoint/list                 | C→S           | List checkpoints
checkpoint/delete               | C→S           | Delete checkpoint
────────────────────────────────┼───────────────┼──────────────────────────────────
query/cost                      | C→S           | Query cost information
query/context                   | C→S           | Query execution context
query/artifact                  | C→S           | Retrieve artifact
query/node                      | C→S           | Query node details
query/tools                     | C→S           | List available tools
query/models                    | C→S           | List available models
────────────────────────────────┼───────────────┼──────────────────────────────────
tool/register                   | C→S           | Register a custom tool
tool/unregister                 | C→S           | Unregister a tool
tool/list                       | C→S           | List registered tools
tool/describe                   | C→S           | Describe a tool
tool/invoke                     | C→S           | Invoke tool directly
────────────────────────────────┼───────────────┼──────────────────────────────────
$/cancel_request                | C→S           | Cancel a pending request
$/progress                      | S→C           | Progress notification
```

---

# Appendix B -- Error Code Reference

```
Code     | Name                      | Description
─────────┼───────────────────────────┼──────────────────────────────────────────
-32700   | PARSE_ERROR               | Invalid JSON
-32600   | INVALID_REQUEST           | Invalid JSON-RPC request
-32601   | METHOD_NOT_FOUND          | Unknown method
-32602   | INVALID_PARAMS            | Invalid method parameters
-32603   | INTERNAL_ERROR            | Internal JSON-RPC error
-32001   | SESSION_NOT_FOUND         | Invalid session ID
-32002   | SESSION_CLOSED            | Session is closed
-32003   | INVALID_STATE             | Invalid session state for operation
-32004   | IR_INVALID                | IR validation failed
-32005   | IR_NOT_FOUND              | IR not loaded
-32006   | EXECUTION_NOT_FOUND       | Execution ID not found
-32007   | EXECUTION_ALREADY_RUNNING | Execution in progress
-32008   | UNSUPPORTED_OPERATION     | Operation not supported by server
-32009   | REQUEST_TOO_LARGE         | Request exceeds size limit
-32010   | SERVER_ERROR              | Internal server error
-32011   | TRANSPORT_ERROR           | Transport-level error
-31001   | TASK_FAILED               | Task execution failed
-31002   | WORKFLOW_FAILED           | Workflow execution failed
-31003   | GOAL_FAILED               | Goal criteria not met
-31004   | APPROVAL_REJECTED         | Human rejected
-31005   | APPROVAL_TIMEOUT          | Approval timed out
-31006   | CANCELLED                 | Execution cancelled
-31007   | DEADLINE_EXCEEDED         | Execution deadline exceeded
-31008   | RECOVERY_EXHAUSTED        | All recovery attempts failed
-30001   | AUTH_REQUIRED             | Authentication required
-30002   | AUTH_INVALID              | Invalid credentials
-30003   | PERMISSION_DENIED         | Insufficient permissions
-30004   | RATE_LIMITED              | Rate limit exceeded
-30005   | TOKEN_EXPIRED             | Auth token expired
-30006   | SCOPE_INSUFFICIENT        | Token scope insufficient
-29001   | BUDGET_EXCEEDED           | Cost budget exceeded
-29002   | RESOURCE_EXHAUSTED        | Memory/CPU/disk full
-29003   | BACKEND_UNAVAILABLE       | LLM backend unreachable
-29004   | TOOL_UNAVAILABLE          | Required tool unavailable
-29005   | SANDBOX_ERROR             | Sandbox violation
-29006   | CHECKPOINT_ERROR          | Checkpoint operation failed
-28001   | NOT_IMPLEMENTED           | Feature not implemented
-28002   | DEPRECATED                | Deprecated feature used
```

### F.1 Mapping to Standard Library Error Codes

LTP error codes (negative JSON-RPC codes) map to Lucky Standard Library error codes (positive integers) as follows:

| LTP Code | LTP Name | StdLib Code | Notes |
|---|---|---|---|
| -31001 | TASK_FAILED | 40 | Map to TOOL_ERROR or MODEL_ERROR depending on cause |
| -31002 | WORKFLOW_FAILED | 1 | Internal runtime failure |
| -31003 | GOAL_FAILED | 1 | Internal runtime failure |
| -31004 | APPROVAL_REJECTED | 50 | |
| -31005 | APPROVAL_TIMEOUT | 51 | |
| -31006 | CANCELLED | 6 | |
| -31007 | DEADLINE_EXCEEDED | 10 | |
| -32004 | IR_INVALID | 21 | Parse/validation error |
| -30001 | AUTH_REQUIRED | 3 | Permission denial |
| -30002 | AUTH_INVALID | 3 | Permission denial |
| -30003 | PERMISSION_DENIED | 3 | |
| -30004 | RATE_LIMITED | 61 | |
| -29001 | BUDGET_EXCEEDED | 60 | |
| -29002 | RESOURCE_EXHAUSTED | 9 | |
| -29003 | BACKEND_UNAVAILABLE | 8 | Transient |
| -29004 | TOOL_UNAVAILABLE | 40 | |
| -29005 | SANDBOX_ERROR | 41 | |
| -28001 | NOT_IMPLEMENTED | 14 | |

The `data` field in LTP error responses carries the StdLib error code in `data.error_code` for runtimes that need to map back.

```

---

# Appendix C -- JSON Schema for All Messages

### C.1 Session Initialize Request

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://lucky-lang.dev/ltp/v0.1/session-initialize-request.json",
  "type": "object",
  "required": ["jsonrpc", "method", "params", "id"],
  "properties": {
    "jsonrpc": { "const": "2.0" },
    "method": { "const": "session/initialize" },
    "params": {
      "type": "object",
      "required": ["protocol_version", "client_info"],
      "properties": {
        "protocol_version": { "type": "string" },
        "client_info": {
          "type": "object",
          "required": ["name", "version"],
          "properties": {
            "name": { "type": "string" },
            "version": { "type": "string" },
            "platform": { "type": "string" },
            "extra": { "type": "object" }
          }
        },
        "capabilities": {
          "type": "object",
          "properties": {
            "streaming": { "type": "boolean" },
            "batch": { "type": "boolean" },
            "human_approval": { "type": "boolean" },
            "checkpoint_restore": { "type": "boolean" }
          }
        },
        "auth": {
          "type": "object",
          "required": ["type"],
          "properties": {
            "type": { "enum": ["none", "bearer", "basic", "mtls", "oauth2"] },
            "token": { "type": "string" }
          }
        }
      }
    },
    "id": { "type": ["integer", "string"] }
  }
}
```

---

# Appendix D -- Conformance Test Suite

### D.1 Conformance Levels

| Level | Requirements |
|---|---|
| **L1 - Basic** | Initialize session, load IR, start sync execution, close session. stdio transport only. |
| **L2 - Standard** | L1 + async execution, event streaming, pause/resume, checkpoint create/restore, HTTP transport. |
| **L3 - Advanced** | L2 + approvals, batch operations, tool registration, WebSocket transport, cost tracking. |
| **L4 - Enterprise** | L3 + multi-session, mTLS, rate limiting, sandbox integration, distributed execution. |

### D.2 Test Cases (Excerpt)

```
Test: l1_init
  Description: Basic session initialization
  Steps:
    1. Client sends session/initialize
    2. Server responds with session_id and capabilities
  Assert:
    - Response has jsonrpc "2.0"
    - session_id is valid UUID
    - capabilities is non-empty object

Test: l1_load_ir
  Description: Load valid IR
  Steps:
    1. Initialize session
    2. Send ir/load with valid IR JSON
  Assert:
    - Response includes ir_hash
    - validation.valid == true
    - metadata includes entry_points

Test: l1_execute_sync
  Description: Execute a simple workflow synchronously
  Steps:
    1. Initialize, load IR with a 2-node workflow
    2. Send execution/start with mode "sync"
  Assert:
    - Response status is "completed"
    - result is "success"

Test: l2_stream_events
  Description: Receive execution events during async execution
  Steps:
    1. Initialize, load IR
    2. Start async execution
    3. Collect events for 5 seconds
  Assert:
    - Received "execution.started" event
    - Received at least one "node.completed" event
    - Event sequence numbers are monotonically increasing

Test: l2_checkpoint
  Description: Create and restore from checkpoint
  Steps:
    1. Execute a workflow with checkpointing
    2. Create manual checkpoint mid-execution
    3. Cancel execution
    4. Restore from checkpoint
    5. Resume execution
  Assert:
    - Restored execution completes successfully
    - Nodes before checkpoint are not re-executed

Test: l3_approval
  Description: Handle human approval
  Steps:
    1. Execute a workflow with an approval node
    2. Receive approval/request notification
    3. Send approval/respond with "approve"
  Assert:
    - Execution continues after approval
    - Approval node transitions to completed
```

---

# Appendix E -- Reference Implementation Guide

### E.1 Minimal Server (Python)

```python
#!/usr/bin/env python3
"""Minimal LTP server implementation."""

import json
import sys
import uuid
from typing import Any, Dict


class MinimalLtpServer:
    def __init__(self):
        self.session_id = None
        self.sessions: Dict[str, Dict[str, Any]] = {}
        self.executions: Dict[str, Dict[str, Any]] = {}
        self.ir = None

    def handle_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        method = request.get("method", "")
        params = request.get("params", {})
        req_id = request.get("id")

        try:
            if method == "session/initialize":
                return self._initialize(params, req_id)
            elif method == "session/close":
                return self._close(params, req_id)
            elif method == "ir/load":
                return self._load_ir(params, req_id)
            elif method == "ir/unload":
                return self._unload_ir(params, req_id)
            elif method == "execution/start":
                return self._start_execution(params, req_id)
            elif method == "execution/get_status":
                return self._get_status(params, req_id)
            elif method == "execution/cancel":
                return self._cancel(params, req_id)
            else:
                return self._error(-32601, f"Method not found: {method}", req_id)
        except Exception as e:
            return self._error(-32603, str(e), req_id)

    def _initialize(self, params, req_id):
        self.session_id = str(uuid.uuid4())
        return {
            "jsonrpc": "2.0",
            "result": {
                "session_id": self.session_id,
                "protocol_version": "0.1",
                "server_info": {"name": "MinimalLTP", "version": "0.1.0"},
                "capabilities": {
                    "streaming": False,
                    "batch": False,
                    "human_approval": False,
                    "checkpoint_restore": False,
                    "supported_models": ["Local"],
                    "supported_tools": ["Filesystem", "Shell"],
                    "cost_tracking": False
                }
            },
            "id": req_id
        }

    def _load_ir(self, params, req_id):
        self.ir = params.get("ir")
        return {
            "jsonrpc": "2.0",
            "result": {
                "ir_hash": "sha256:minimal",
                "validation": {"valid": True, "warnings": []},
                "metadata": {"project_name": "minimal", "node_count": 0}
            },
            "id": req_id
        }

    def _start_execution(self, params, req_id):
        exec_id = str(uuid.uuid4())
        self.executions[exec_id] = {
            "id": exec_id,
            "status": "running",
            "entry_point": params.get("entry_point"),
            "started_at": "2026-01-01T00:00:00Z",
            "cost": {"total_usd": 0.0}
        }
        if params.get("mode") == "sync":
            # Simulate immediate completion
            self.executions[exec_id]["status"] = "completed"
            return {
                "jsonrpc": "2.0",
                "result": {
                    "execution_id": exec_id,
                    "status": "completed",
                    "result": "success",
                    "cost": {"total_usd": 0.0},
                    "duration_ms": 0
                },
                "id": req_id
            }
        else:
            return {
                "jsonrpc": "2.0",
                "result": {
                    "execution_id": exec_id,
                    "status": "running"
                },
                "id": req_id
            }

    def _error(self, code, message, req_id):
        return {
            "jsonrpc": "2.0",
            "error": {"code": code, "message": message},
            "id": req_id
        }

    def run_stdio(self):
        """Run server over stdin/stdout."""
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            try:
                request = json.loads(line)
                response = self.handle_request(request)
                sys.stdout.write(json.dumps(response) + "\n")
                sys.stdout.flush()
            except json.JSONDecodeError:
                error = self._error(-32700, "Parse error", None)
                sys.stdout.write(json.dumps(error) + "\n")
                sys.stdout.flush()


if __name__ == "__main__":
    server = MinimalLtpServer()
    server.run_stdio()
```

### E.2 Minimal Client (Python)

```python
#!/usr/bin/env python3
"""Minimal LTP client."""

import json
import subprocess
import sys


class LtpClient:
    def __init__(self, command: list[str]):
        self.process = subprocess.Popen(
            command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True
        )
        self._next_id = 1

    def _send(self, method: str, params: dict = None) -> dict:
        request = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or {},
            "id": self._next_id
        }
        self._next_id += 1
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        response = json.loads(self.process.stdout.readline())
        if "error" in response:
            raise Exception(f"LTP Error {response['error']['code']}: {response['error']['message']}")
        return response["result"]

    def initialize(self):
        return self._send("session/initialize", {
            "protocol_version": "0.1",
            "client_info": {"name": "MinimalClient", "version": "0.1.0"}
        })

    def load_ir(self, ir: dict):
        return self._send("ir/load", {"ir": ir})

    def execute(self, goal: str, context: dict = None):
        return self._send("execution/start", {
            "entry_point": goal,
            "entry_kind": "goal",
            "context": context or {},
            "mode": "sync"
        })

    def close(self):
        self._send("session/close", {"reason": "completed"})
        self.process.terminate()


# Usage
if __name__ == "__main__":
    client = LtpClient(["python", "minimal_ltp_server.py"])
    result = client.initialize()
    print(f"Connected to: {result['server_info']['name']} v{result['server_info']['version']}")

    # Load a minimal IR
    ir = {
        "version": "0.1",
        "meta": {"ir_level": "high"},
        "graph": {"nodes": [], "edges": []}
    }
    client.load_ir(ir)

    # Execute
    result = client.execute("TestGoal")
    print(f"Execution: {result['result']}")

    client.close()
```

---

*End of Lucky Tool Protocol Specification, Version 0.1*
