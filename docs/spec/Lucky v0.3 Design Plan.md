# Lucky v0.3 — Ecosystem Maturity: Design Plan

<img src="../../logo/logo128.png" alt="Lucky logo" width="64" align="right" />

**Version:** 0.3 Draft  
**Status:** Design Plan  
**Based on:** v0.2 Production-Ready Compiler & Runtime  

---

## Table of Contents

1. [Strategic Overview](#1-strategic-overview)
2. [Architecture Vision](#2-architecture-vision)
3. [Work Packages](#3-work-packages)
   - [A: Platform Integration](#a-platform-integration-30)
   - [B: Security & Sandboxing](#b-security--sandboxing-15)
   - [C: Standard Library Runtime](#c-standard-library-runtime-15)
   - [D: Language Completeness](#d-language-completeness-15)
   - [E: Observability & Telemetry](#e-observability--telemetry-10)
   - [F: Distributed Runtime](#f-distributed-runtime-10)
   - [G: Deferred to v0.4](#g-deferred-to-v04-5)
4. [Timeline & Milestones](#4-timeline--milestones)
5. [Design Decisions](#5-design-decisions)
6. [Risk Assessment](#6-risk-assessment)
7. [Success Criteria](#7-success-criteria)

---

## 1. Strategic Overview

### 1.1 Where v0.2 Left Us

v0.2 produced a **production-ready, single-node** Lucky system:

| Capability | Status |
|---|---|
| Full compiler pipeline (Lexer → AST → HIR → MIR → Opt → JSON IR) | ✅ Complete |
| 16-command CLI (compile, run, test, debug, lsp, watch, doc, config, pkg, serve) | ✅ Complete |
| VS Code extension + LSP + DAP debugger | ✅ Complete |
| 4 LLM backends (DeepSeek, OpenAI, Ollama, Anthropic) | ✅ Complete |
| Runtime engine (scheduler, context, memory, permissions, tools) | ✅ Complete |
| Checkpoint/resume, cost budget, audit trail | ✅ Complete |
| Platform adapters (Claude Code, Codex CLI, OpenCode, Cursor, Dify) | ✅ Complete |
| 6 spec documents detailing language, runtime, std lib, IR, LTP | ✅ Complete |

**But there's a problem.** Lucky is an *agent orchestration language* — its value lives in describing multi-agent workflows that actual AI coding tools execute. A standalone Lucky program that doesn't run on any agent platform is a dead program. The current adapters work, but they're static config files with no CI pipeline, no embedding story, and no easy path for new platforms to integrate.

### 1.2 The Core Insight

> **Lucky isn't where the work happens — it describes the work, and the agents execute it.**

The Lucky value chain:

```
.lk source  ──→  .lir IR  ──→  LTP  ──→  Agent Platform
                                            → Claude Code
                                            → Codex CLI
                                            → OpenCode
                                            → Cursor
                                            → WorkBuddy
                                            → Windsurf
                                            → Cline
                                            → ...
```

If an agent platform can embed the Lucky runtime in an afternoon, Lucky wins. If it takes a week, platforms won't bother. **v0.3's top priority is making platform integration trivially easy.**

### 1.3 v0.3 Goal

> **Make Lucky the language that every agent platform wants to embed.**

Concretely:
- Any agent coding tool can embed the Lucky runtime with a single C header and a 5-line initialization.
- Existing adapters (Claude Code, Codex, OpenCode, Cursor) have CI pipelines proving they work against real platforms.
- At least 2 new platforms (WorkBuddy, Windsurf/Cline) ship with Lucky support.
- `lucky run` on any embedded platform just works — approvals, checkpoints, cost tracking, the whole stack.

Language features, optimizer passes, distributed scheduling — these only matter if people are actually running Lucky. First, make it embeddable.

### 1.4 Guiding Principles

1. **Integration first.** Every feature is evaluated against "does this help a platform embed Lucky?". If no, it's deferred.
2. **C SDK is king.** The embedding surface is a tiny C library with no dependency on Rust tooling. Any platform can link it, in any language.
3. **Backward compatibility.** v0.3 programs must compile on v0.2. Breaking changes require a migration path.
4. **Proven adapters.** Every supported platform has a CI test that compiles a real Lucky workflow, sends it through LTP, and asserts the expected events come back.

---

## 2. Architecture Vision

### 2.1 v0.3 Component Map

```
┌──────────────────────────────────────────────────────────────────────────┐
│                          Lucky v0.3 System                               │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    Platform Integration Layer                      │    │
│  │                                                                   │    │
│  │  ┌─────────────────────┐  ┌─────────────────────┐                │    │
│  │  │  LTP Embedding SDK  │  │  Adapter Framework   │                │    │
│  │  │  (C99, ~200LOC)     │  │  + CI Pipelines      │                │    │
│  │  └─────────┬───────────┘  └──────────┬──────────┘                │    │
│  │            │                         │                            │    │
│  │            ▼                         ▼                            │    │
│  │  ┌─────────────────┐  ┌───────────────────────┐  ┌────────────┐  │    │
│  │  │  Claude Code    │  │  Codex CLI / OpenCode  │  │  Cursor    │  │    │
│  │  │  (MCP adapter)  │  │  (YAML/Python adapter) │  │  (VS Code) │  │    │
│  │  └─────────────────┘  └───────────────────────┘  └────────────┘  │    │
│  │  ┌─────────────────┐  ┌───────────────────────┐  ┌────────────┐  │    │
│  │  │  WorkBuddy      │  │  Windsurf / Cline      │  │  Dify      │  │    │
│  │  │  (plugin)       │  │  (MCP adapter)         │  │  (YAML)    │  │    │
│  │  └─────────────────┘  └───────────────────────┘  └────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    Lucky Runtime Engine                           │    │
│  │                                                                   │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐  │    │
│  │  │  Scheduler   │  │  Context     │  │  Sandbox (Docker)       │  │    │
│  │  │  + Executor  │  │  Manager     │  │  Tool isolation         │  │    │
│  │  └──────────────┘  └──────────────┘  └────────────────────────┘  │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐  │    │
│  │  │  Std Lib     │  │  Checkpoint  │  │  Telemetry             │  │    │
│  │  │  Runtime     │  │  + Audit     │  │  (OpenTelemetry)       │  │    │
│  │  └──────────────┘  └──────────────┘  └────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    Compiler Pipeline                              │    │
│  │                                                                   │    │
│  │  Lexer → AST → Semantic → HIR → MIR → Opt → JSON IR             │    │
│  │                                                                   │    │
│  │  (MIR optimizer kept at v0.2 level — no new passes)              │    │
│  └─────────────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Key New Interfaces

```c
// The entire Embedding API — one header file
// lucky.h — C99, no dependencies beyond libc

typedef struct lucky_rt lucky_rt_t;
typedef struct lucky_session lucky_session_t;

typedef enum {
    LUCKY_OK = 0,
    LUCKY_NODE_STARTED,
    LUCKY_NODE_COMPLETED,
    LUCKY_NODE_FAILED,
    LUCKY_APPROVAL_REQUIRED,
    LUCKY_CHECKPOINT_CREATED,
    LUCKY_COMPLETED,
    LUCKY_FAILED,
} lucky_event_kind_t;

typedef struct {
    lucky_event_kind_t kind;
    const char* node_id;
    const char* label;
    const char* message;   // for approvals
    const char* json_data; // arbitrary structured payload
} lucky_event_t;

// Lifecycle
lucky_rt_t*    lucky_init(void);
void           lucky_destroy(lucky_rt_t* rt);

// Session — one per workflow run
lucky_session_t* lucky_session_create(lucky_rt_t* rt, const char* ir_json);
void             lucky_session_destroy(lucky_session_t* session);

// Poll — returns next event, blocks up to `timeout_ms`
lucky_event_t  lucky_session_poll(lucky_session_t* session, int timeout_ms);

// Respond to an approval request
void           lucky_session_approve(lucky_session_t* session, const char* gate_id);
void           lucky_session_reject(lucky_session_t* session, const char* gate_id);

// Check status
int            lucky_session_is_done(lucky_session_t* session);
const char*    lucky_session_error(lucky_session_t* session);
```

The entire integration for an agent platform:

```python
# Python binding via ctypes — trivially generated
import ctypes
lucky = ctypes.CDLL("./liblucky.so")

rt = lucky.lucky_init()
session = lucky.lucky_session_create(rt, ir_json)
while not lucky.lucky_session_is_done(session):
    ev = lucky.lucky_session_poll(session, 100)
    if ev.kind == LUCKY_APPROVAL_REQUIRED:
        show_notification(ev.message)
        if user_clicks_approve():
            lucky.lucky_session_approve(session, ev.node_id)
    elif ev.kind == LUCKY_NODE_STARTED:
        log(f"Starting: {ev.label}")
lucky.lucky_destroy(rt)
```

In a Rust agent platform (like WorkBuddy), the same API binds natively:

```rust
let rt = lucky_sys::lucky_init();
let session = lucky_sys::lucky_session_create(rt, ir);
loop {
    let ev = lucky_sys::lucky_session_poll(session, 100);
    match ev.kind {
        LUCKY_APPROVAL_REQUIRED => platform.prompt_user(ev.message),
        LUCKY_COMPLETED => break,
        _ => {}
    }
}
```

---

## 3. Work Packages

### A) Platform Integration (30% of effort)

This is the **centerpiece** of v0.3. Everything else supports it.

| # | Feature | Effort | Description |
|---|---|---|---|
| **A1** | LTP Embedding C SDK | M | `lucky.h` + `liblucky.so` (100KB static library). Compile-once, link-anywhere. Wraps the runtime's event loop behind a pure-C API. No Rust, no cargo, no LLVM. |
| **A2** | Python/Node/Rust bindings | M | Auto-generated language bindings from the C SDK. `pip install lucky-sdk`, `npm install lucky-sdk`, `cargo add lucky-sdk`. |
| **A3** | Adapter CI pipelines | M | Every adapter gets a GitHub Actions workflow that: (1) starts `lucky serve`, (2) submits a test IR, (3) asserts expected events. This is the proof that an adapter actually works. |
| **A4** | WorkBuddy integration | M | New adapter for [WorkBuddy](https://workbuddy.ai). Plugin or MCP-based. Demo workflow that shows Lucky supervising multi-agent code review on WorkBuddy. |
| **A5** | Windsurf / Cline integration | M | New MCP-based adapter. Windsurf and Cline both support MCP tools, so LTP as an MCP server is the natural fit. |
| **A6** | "Lucky in 5 Minutes" integration guide | S | A single markdown page that any platform author reads: "Add Lucky to your agent tool in 5 minutes." The C SDK header, the event loop pattern, the approval handling. |
| **A7** | LTP MCP bridge | M | Package LTP as a Model Context Protocol server. Any MCP-compatible client (Claude Desktop, Windsurf, Cline, Continue, etc.) talks to Lucky through MCP natively. |
| **A8** | Adapter health dashboard | S | `lucky adapter check` — runs a quick smoke test against each configured platform adapter and reports pass/fail with diagnostics. |

#### A1 — LTP Embedding C SDK

The C SDK is the foundation for everything else. Design:

```
┌────────────────────┐
│  Platform Code     │  (Python, Rust, Go, C#, etc.)
├────────────────────┤
│  lucky.h / liblucky│  C99 static library
├────────────────────┤
│  lucky_runtime     │  Rust core — statically linked, no export of Rust symbols
│  (from compiler)   │
├────────────────────┤
│  scheduler         │
│  context           │
│  memory            │
│  permissions       │
│  tools             │
└────────────────────┘
```

The C SDK is built by compiling the Rust runtime with `cargo build --features c-api`, which produces `liblucky.a`. The C header maps the Rust event-driven execution loop to a simple poll interface.

**Platform compatibility:**

| Platform | SDK form | Binding |
|---|---|---|
| Claude Code (TypeScript) | shared lib | node-ffi / MCP bridge |
| Codex CLI (Python) | shared lib | ctypes |
| OpenCode (Python) | shared lib | ctypes |
| Cursor (TypeScript) | shared lib | node-ffi / VS Code ext |
| WorkBuddy (Rust) | static lib | native `extern "C"` |
| Windsurf/Cline (TS) | shared lib | MCP bridge |
| Dify (Python/YAML) | shared lib | ctypes or subprocess |
| Any C/C++ tool | static lib | direct `#include "lucky.h"` |

#### A3 — Adapter CI Pattern

```yaml
# .github/workflows/adapter-claude-code.yml
name: Test Claude Code adapter
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Lucky runtime
        run: cargo build --release
      - name: Start LTP server
        run: ./target/release/lucky serve --port 9700 &
      - name: Install Claude Code
        run: npm install -g @anthropic-ai/claude-code
      - name: Run adapter test
        run: |
          # Compile a test workflow
          ./target/release/lucky compile tests/adapter_test.lk --ir > test.lir
          # Submit IR via LTP and assert events
          python tests/ltp_test_client.py test.lir \
            --expect-nodes 5 \
            --expect-approvals 1 \
            --timeout 60s
```

Each adapter has its own workflow. The test IR exercises:
- Sequential chain
- Parallel fan-out
- Approval gate (auto-approved in CI)
- Error recovery (intentionally fail a task, assert retry)

#### A7 — LTP as MCP Server

Many modern agent tools (Claude Desktop, Windsurf, Cline, Continue) speak MCP natively. Running Lucky as an MCP tool server means any MCP client gains Lucky orchestration without writing a single line of adapter code.

```json
// MCP tool definition — auto-generated from Lucky IR
{
  "name": "lucky-run-workflow",
  "description": "Execute a Lucky orchestration workflow",
  "inputSchema": {
    "type": "object",
    "properties": {
      "workflow": { "type": "string", "description": "Path to .lir file" },
      "context": { "type": "object", "description": "Initial context values" }
    }
  }
}
```

Usage: `lucky serve --mcp` starts an MCP-compatible server alongside the LTP server.

---

### B) Security & Sandboxing (15% of effort)

Platforms won't embed Lucky unless they trust it. Sandboxing and security are the **gate** to platform adoption.

| # | Feature | Effort | Description |
|---|---|---|---|
| **B1** | Docker sandbox provider | M | Tool execution in ephemeral Docker containers. Filesystem isolation (read-only workspace, temp outputs), network policies (allow-list by domain), resource limits (CPU/memory/disk). |
| **B2** | Permission audit at runtime | S | Confirm that lexical permission inheritance (restrict-only, never widen) is enforced at runtime, not just compile time. Runtime logs every allow/deny decision. |
| **B3** | Secrets management | S | `lucky secret set KEY=value` — encrypted at rest (age/NaCl secretbox). Injected into context as `secret.X` references. Never logged, never in audit trails. |
| **B4** | LTP mTLS | M | Mutual TLS for LTP connections. The embedding SDK (A1) should support optional mTLS. Platform trusts Lucky; Lucky trusts platform. |
| **B5** | Path traversal protection | S | `Filesystem.read` and `Filesystem.write` enforce a root directory. `../../etc/passwd` is denied even if permissions allow `filesystem.read`. Canonical path check. |

#### B1 — Docker Sandbox Architecture

```rust
pub struct DockerSandbox {
    container_id: String,
    workspace_dir: PathBuf,      // mounted host path
    allowed_domains: Vec<String>,// e.g. ["api.openai.com", "api.deepseek.com"]
}

impl SandboxProvider for DockerSandbox {
    fn create_sandbox(config: SandboxConfig) -> Self {
        // docker run --rm -d \
        //   --network isolated \
        //   --read-only \
        //   --memory 512m \
        //   --cpus 2 \
        //   --mount type=bind,source=...,target=/workspace \
        //   lucky-sandbox:latest
    }
    fn execute_tool(&self, tool: ToolInvocation) -> ToolResult {
        // docker exec <container> <tool>
        // stdout/stderr captured, returned as result
        // deadline enforced with docker stop --time
    }
    fn destroy_sandbox(&self) {
        // docker rm -f <container>
    }
}
```

The sandbox image is minimal (based on `alpine:latest` with Python, Node, Git, curl, and jq). Tools are injected at runtime.

---

### C) Standard Library Runtime (15% of effort)

The spec defines a rich standard library. Today none of it exists at runtime — only the type declarations are defined. Platforms need these methods to actually do useful work.

| # | Feature | Effort | Description |
|---|---|---|---|
| **C1** | Core type methods | M | `Bool::not/and/or`, `Int::abs/pow/clamp`, `Float::round/sqrt/sin`, `String::len/split/replace/find`, `Bytes::to_hex/to_base64`. Implemented as native Rust functions. |
| **C2** | Collection methods | M | `List::map/filter/reduce/sort`, `Map::get/insert/keys/values`, `Set::union/intersect/difference`. Operate on `RuntimeValue`. |
| **C3** | `ai` package | L | `ai.ask`, `ai.summarize`, `ai.translate`, `ai.embed`, `ai.extract_keywords`, `ai.classify`, `ai.sentiment`. Each triggers an LLM call through the backend router. `ai.rag` — vector search from `knowledge` declarations. |
| **C4** | `http` package | M | `http.get`, `http.post`, `http.put`, `http.delete`. Response: `.status`, `.json()`, `.text()`, `.headers`. Retry, timeout, backoff params. Backed by `reqwest`. |
| **C5** | `json`, `time`, `math`, `crypto` packages | M | `json.parse`, `json.stringify`. `time.now`, `time.format`, `time.parse`, `time.sleep`. `math.abs/max/min/sqrt/sin/cos/random`. `crypto.hash/sha256/aes_encrypt/aes_decrypt`. |
| **C6** | Std lib documentation | S | Per-function docs generated from Rust doc comments. Published to docs.lucky-lang.org. |

#### C3 — ai Package Runtime

```rust
pub fn ai_ask(args: Vec<RuntimeValue>, runtime: &Runtime) -> Result<RuntimeValue> {
    let question = args[0].as_string()?;
    let model = args.get(1).and_then(|a| a.as_string());
    let min_confidence = args.get(2).and_then(|a| a.as_float());

    let backend = runtime.backend_router.select(model)?;
    let response = backend.complete(&[Message::user(question)])?;

    if let Some(min_conf) = min_confidence {
        if response.confidence < min_conf {
            return Err("Low confidence".into());
        }
    }
    Ok(RuntimeValue::Probabilistic {
        value: Box::new(RuntimeValue::String(response.text)),
        confidence: response.confidence,
    })
}
```

---

### D) Language Completeness (15% of effort)

Only the features that platforms **actually need** for real workflows. Defer anything speculative.

| # | Feature | Effort | Spec Reference | Why Platforms Need It |
|---|---|---|---|---|
| **D1** | `reason` mode | S | LRM Ch. 41 | Platforms expose reasoning: DeepSeek R1, Claude Sonnet with thinking, OpenAI o-series. Lucky should control this. |
| **D2** | `deploy` declaration | M | LRM Ch. 48 | Platforms need to know where output goes. `deploy Docker`, `deploy local`, `deploy cloud`. |
| **D3** | `when` / reactive events | L | LRM Ch. 68-69 | CI/CD workflows are event-driven. "When PR opened → run security audit." This is what platforms do. |
| **D4** | `pub` visibility | S | LRM Ch. 15 | Package reusability across projects. Platforms import community packages. |
| **D5** | Extended pattern matching | M | LRM Ch. 30-33 | Destructuring on AI outputs. Common in agent workflows. |
| **D6** | `transaction` blocks | M | LRM Ch. 62 | Platforms need rollback safety. Failed deploy → auto-rollback. |
| **D7** | Custom type declarations | M | LRM Ch. 12-14 | `type ReviewResult = Approved { notes } | Rejected { reasons }`. Sum types for structured AI outputs. |

**Deferred from language (to v0.4):**
- `confidence` expressions — interesting but platforms don't surface this today
- Stream types — complex, no platform demand yet
- `knowledge` declarations — needs vector DB integration, defer to v0.4
- `ask human` interactive queries — platforms handle this differently

---

### E) Observability & Telemetry (10% of effort)

Platforms need to show users what Lucky is doing. Telemetry is the window.

| # | Feature | Effort | Description |
|---|---|---|---|
| **E1** | Structured events via SDK | M | The C SDK (A1) already fires events. These events carry structured JSON payloads. Document the event schema. |
| **E2** | Platform-friendly event format | S | Events are designed for the platform to render: `NodeStarted { label, kind, estimated_cost }`, `ApprovalRequired { message, gate_id }`, `CostUpdated { total, remaining_budget }`. |
| **E3** | Cost & token tracking in events | S | Every `NodeCompleted` event carries `tokens_prompt`, `tokens_completion`, `cost_usd`. Platforms can show live cost in their UI. |
| **E4** | OpenTelemetry export | M | Optional: pipe events to OTel. For platforms that already use OTel, Lucky events become spans. |
| **E5** | `lucky observe` CLI | S | Standalone TUI showing live workflow progress. Useful for debugging and demos. |

---

### F) Distributed Runtime (10% of effort)

Distributed execution is useful, but not the priority. We build just enough for the common case: a coordinator that fans out to worker processes on the **same machine** (multi-core) or a small cluster (2-3 machines).

| # | Feature | Effort | Description |
|---|---|---|---|
| **F1** | Simple TCP coordinator | M | No NATS — just TCP + JSON messages. Coordinator and workers are subprocesses. Good for 2-10 workers. |
| **F2** | `lucky run --workers N` | S | Fan out to N local worker processes. Each worker gets its own core/thread. Useful for `swarm 50` on a single machine. |
| **F3** | Remote worker | M | `lucky worker --connect host:port`. Register with a remote coordinator. Run on another machine. |
| **F4** | Basic affinity | S | "This worker has GPU" / "This worker has local filesystem access." Match nodes to workers by capability. |
| **F5** | Distributed checkpoint (local FS) | M | Checkpoint to a shared NFS or SMB mount. Coordinator writes DAG state; workers write memory snapshots. |

**What we're NOT building:**
- NATS message bus (overkill for v0.3)
- Full distributed scheduler with dynamic worker pools
- Distributed tracing across machines
- K8s operator (deferred)

Simple TCP + subprocess workers covers the 90% use case: `swarm 50` on a beefy dev machine, or a staging server with 2 nodes.

---

### G) Deferred to v0.4 (5%)

These features are designed in the spec, not implemented in v0.2, and explicitly deferred because they don't help platform adoption.

| Feature | Rationale |
|---|---|
| **GVN / LICM / Inlining** | MIR optimizer is already good enough for v0.3. Most workflows are < 50 nodes; optimization doesn't matter. |
| **Low-level IR (LIR)** | MIR → JSON → Runtime is fine for now. LIR is needed for native codegen, which nobody has asked for. |
| **Binary IR (FlatBuffers)** | JSON IR is already small (< 100KB for most workflows). Premature optimization. |
| **Kubernetes operator** | Nobody is running Lucky in production at K8s scale yet. When platforms embed it, this becomes relevant. |
| **Package registry server** | File-system packages and `lucky pkg install ./path` work fine for v0.3. A central registry is a v0.4 service. |
| **AI-specific optimizer** | LLM call fusion, prompt caching — clever but unproven. Let platforms do their own caching. |
| **Firecracker sandbox** | Docker is enough for v0.3. Firecracker adds complexity (VM images, kernel config, Linux-only). |

---

## 4. Timeline & Milestones

Total estimated effort: **16-20 weeks** (4-5 months) for 1-2 engineers.
(Faster than the original plan because we dropped 5 complex features.)

| Milestone | Weeks | Content | Dependencies |
|---|---|---|---|
| **M1 — Embeddable Runtime** | 1-4 | A1 (C SDK), A2 (language bindings), A7 (MCP bridge) | v0.2 runtime |
| **M2 — Platform Proof** | 5-7 | A3 (adapter CI), A4 (WorkBuddy), A5 (Windsurf/Cline), A6 (integration guide) | M1 |
| **M3 — Security Foundation** | 8-10 | B1 (Docker sandbox), B2 (runtime audit), B3 (secrets), B5 (path traversal) | v0.2 runtime |
| **M4 — Standard Library** | 11-13 | C1 (core types), C2 (collections), C3 (ai package), C4 (http), C5 (json/time/math/crypto), C6 (docs) | v0.2 runtime |
| **M5 — Language + Observability** | 14-16 | D1 (reason), D2 (deploy), D3 (reactive), D4 (pub), D5-D7, E1-E4, E5 (`observe`) | M1 |
| **M6 — Distributed + Release** | 17-20 | F1-F5 (simple distributed), E5 (`observe`), polish, docs, changelog | M3, M5 |

### Dependency Graph

```
M1 ──→ M2 ──────────────────────┐
                                 │
M3 ──────────────────────────┐   │
                             │   │
M4 ───────────────────────┐  │   │
                          │  │   │
M5 ──────────────────────►M6◄───┘
```

M1, M3, and M4 are **fully parallel** — they touch different parts of the codebase (C SDK, runtime sandbox, compiler lib) and can be built simultaneously.

---

## 5. Design Decisions

### D1: C SDK over FFI Generators

**Decision:** Hand-write a C99 header and implement it in Rust with `extern "C"`. Not wasm, not gRPC, not FFI generators.

**Rationale:**
- A single `lucky.h` + `liblucky.a` is the lowest-friction integration possible. Any language can call C.
- Wasm would require a wasm runtime in the embedding platform — additional dependency.
- gRPC would require protoc + a server process running alongside — additional complexity.
- The C API surface is tiny (~10 functions). Hand-writing it is safer and more portable than FFI generators.
- Rust's `extern "C"` + `#[no_mangle]` produces clean, debuggable symbols.

### D2: Simple TCP over NATS for Distributed

**Decision:** Use plain TCP + JSON for distributed communication, not NATS.

**Rationale:**
- NATS requires a running server. That's another dependency to install, configure, and monitor.
- For 2-10 workers (the v0.3 target), raw TCP + JSON is simpler and has lower latency.
- If NATS is needed in the future (100+ workers, multi-datacenter), the coordinator/worker trait makes it swappable.
- JSON over TCP means workers are debuggable with `telnet` or `nc`.

### D3: Docker over Firecracker for Sandbox

**Decision:** Docker is the only sandbox provider in v0.3.

**Rationale:**
- Docker is omnipresent. Every CI runner, every dev machine, every staging server has it.
- Firecracker is Linux-only, requires VM image setup, and has no Windows support.
- Docker provides adequate isolation for agent tool execution (filesystem, network, resource limits).
- If VM-level isolation is needed, add Firecracker in v0.4.

### D4: MIR Level Kept at v0.2

**Decision:** No new optimizer passes in v0.3. Ship as-is.

**Rationale:**
- DCE, constant folding, CSE, and copy propagation are already implemented — enough for all practical Lucky programs.
- Most Lucky workflows are 5-50 nodes. Optimization doesn't matter at this scale.
- GVN, LICM, and inlining are each 2-4 week projects. That time is better spent on platform integration.
- If a performance bottleneck appears, optimize then.

### D5: File-System Packages over Central Registry

**Decision:** `lucky pkg install ./path` in v0.3. Central registry in v0.4.

**Rationale:**
- A registry is a service, not a compiler feature. It needs a server, authentication, signing verification, uptime monitoring.
- File-system packages work today: `import ./packages/reviewer.lk` or `import ~/.lucky/packages/*`.
- Package resolution via Git URLs: `lucky pkg install github.com/jfxia/lucky-reviewer`.
- Central registry is the right thing, but it's a v0.4 project with its own release cycle.

---

## 6. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| **Platforms don't adopt the C SDK** | Medium | **Critical** | Prove it works with 3 platforms (Claude Code MCP, WorkBuddy, Windsurf) before declaring done. If adoption is slow, invest in more complete MCP bridge. |
| **C SDK thread safety bugs** | Medium | High | The SDK is single-threaded by design (`lucky_session_poll` is the only entry point; no concurrent access). Add AddressSanitizer to CI. |
| **Docker sandbox breaks on Windows** | Medium | Medium | Docker Desktop on Windows supports bind mounts and resource limits. Test on Windows CI. Fallback: no sandbox (warn and continue). |
| **Standard library scope creep** | Medium | Medium | Ship core types + `ai` package in M4. Defer `http`, `crypto`, `time` to M5 if behind. All are independent. |
| **No new platform partnerships** | Low | **Critical** | If no platform wants Lucky, v0.3 fails regardless of technical quality. Mitigation: build the MCP bridge first (A7) — it makes Lucky work with every MCP client without needing a platform deal. |
| **Over-engineering the C SDK** | Low | Medium | Keep it 10 functions, 200 LOC of C header, 500 LOC of Rust glue. If it grows beyond that, trim. |

### The MCP Hedge

The biggest risk (no platform adoption) is mitigated by the MCP bridge (A7). MCP is becoming the universal protocol for agent-tool communication. If Lucky speaks MCP natively:

- **Claude Desktop** — can run Lucky workflows today, no integration work
- **Windsurf** — supports MCP tools
- **Cline** — supports MCP tools  
- **Continue** — supports MCP tools
- **Cursor** — has experimental MCP support

This means A7 (MCP bridge) should be the **first** thing built in M1, not the last. It de-risks the entire plan.

---

## 7. Success Criteria

### Must-Have (v0.3.0 Release)

- [ ] `lucky.h` C SDK compiles on Linux, macOS, Windows. `lucky_session_poll` delivers correct events for a 5-node workflow.
- [ ] Python binding: `pip install lucky-sdk && python run_workflow.py` works.
- [ ] MCP bridge: `lucky serve --mcp` is connectable from Claude Desktop. User can say "run my Lucky workflow" and see progress.
- [ ] 3 adapters have CI pipelines that pass: Claude Code, WorkBuddy, Windsurf.
- [ ] "Lucky in 5 Minutes" guide published. A platform that follows it can run a Lucky workflow within 5 minutes.
- [ ] Docker sandbox: `lucky run --sandbox docker` runs tool calls in isolated containers. Basic filesystem + network + resource limits work.
- [ ] Permission audit: runtime logs every allow/deny decision with source location.
- [ ] Standard library: `String`, `List`, `Map`, `Int`, `Float`, `Bool` methods + `ai.ask` + `http.get` all work at runtime.
- [ ] `reason deep/fast/none` controls reasoning mode on DeepSeek and OpenAI backends.
- [ ] `deploy Docker` generates a working Dockerfile and builds an image.
- [ ] `when file changes run workflow` triggers on file system events.
- [ ] `lucky run --workers 4` fans out tasks to 4 local worker processes.
- [ ] v0.2 programs compile without changes (backward compatibility).
- [ ] All spec documents updated to v0.3.

### Nice-to-Have (v0.3.1+)

- [ ] Node.js binding: `npm install lucky-sdk`
- [ ] Go binding
- [ ] Cursor adapter CI pipeline
- [ ] Codex CLI adapter CI pipeline
- [ ] Dify adapter CI pipeline
- [ ] Remote worker (`lucky worker --connect host:port`)
- [ ] OTel export
- [ ] `lucky observe` TUI
- [ ] Secrets management CLI

### Metrics to Track

| Metric | Target |
|---|---|
| Time for a new platform to integrate Lucky | < 5 minutes (following the guide) |
| C SDK binary size | < 5MB static, < 1MB shared |
| Adapter CI pipelines passing | 100% on main branch |
| Matched platforms supporting Lucky | ≥ 5 (current adapters + new ones) |
| Std lib method coverage vs. spec | 100% for core types, 80% for `ai`+`http` |
| Docker sandbox isolation | Tool can't read host files outside workspace |
| `lucky run --workers 4` speedup vs. single-process | ≥ 3x for 20+ independent tasks |

---

*Last updated: July 2026 — v0.3 Design Plan (revised with platform-first focus)*
