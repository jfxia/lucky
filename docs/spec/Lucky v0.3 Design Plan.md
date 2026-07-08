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
   - [A: Language Completeness](#a-language-completeness-35)
   - [B: Distributed Runtime](#b-distributed-runtime-20)
   - [C: Advanced Optimizer & IR](#c-advanced-optimizer--ir-15)
   - [D: Observability & Telemetry](#d-observability--telemetry-10)
   - [E: Security & Sandboxing](#e-security--sandboxing-10)
   - [F: Ecosystem & Tooling](#f-ecosystem--tooling-10)
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

### 1.2 v0.3 Goal

> **Transform Lucky from a single-node orchestration engine into a production-grade distributed platform with full observability, enterprise security, and a mature language surface.**

v0.3 closes the gap between what Lucky's specs *design* and what the compiler/runtime *implements*, then layers on distributed execution, observability, and ecosystem infrastructure.

### 1.3 Guiding Principles

1. **Spec-driven development.** Every feature must be designed in the spec before implementation begins. If the spec doesn't describe it, it doesn't exist.
2. **Backward compatibility.** v0.3 programs must compile on v0.2. Breaking changes require a migration path.
3. **Dogfood first.** The Lucky team uses Lucky to build v0.3 features (CI/CD, testing, documentation generation).
4. **Pluggable architecture.** Observability, storage, and sandbox backends are swappable via traits/interfaces.

---

## 2. Architecture Vision

### 2.1 v0.3 Component Map

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Lucky v0.3 System                            │
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │   Language       │  │   Compiler       │  │   CLI & Tooling     │  │
│  │   Features       │  │   Pipeline       │  │                     │  │
│  │                  │  │                  │  │  lucky run --dist   │  │
│  │  reason mode     │  │  Lexer → AST     │  │  lucky deploy       │  │
│  │  confidence      │  │  → Semantic      │  │  lucky observe      │  │
│  │  deploy          │  │  → HIR → MIR     │  │  lucky sandbox      │  │
│  │  when/reactive   │  │  → LIR (NEW!)    │  │  lucky registry     │  │
│  │  transaction     │  │  → Opt passes    │  │                     │  │
│  │  pub visibility  │  │  → JSON/Binary   │  └─────────────────────┘  │
│  │  streams         │  │                  │                          │
│  │  std lib runtime │  └─────────────────┘                          │
│  └────────┬────────┘                                                │
│           │ compiles to IR                                           │
│           ▼                                                          │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                   Runtime Engine                             │    │
│  │                                                              │    │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐  │    │
│  │  │  Local      │  │  Distributed  │  │  Event Bus        │  │    │
│  │  │  Executor   │  │  Coordinator  │  │  (pub/sub)        │  │    │
│  │  │  (v0.2)     │  │  + Workers    │  │  for reactive     │  │    │
│  │  └─────────────┘  └──────┬───────┘  │  workflows         │  │    │
│  │                          │           └────────────────────┘  │    │
│  │  ┌─────────────┐  ┌──────▼───────┐  ┌────────────────────┐  │    │
│  │  │ Sandbox     │  │  Message Bus  │  │  Telemetry         │  │    │
│  │  │ (Docker/    │  │  (NATS)       │  │  (OpenTelemetry)   │  │    │
│  │  │ Firecracker)│  └──────────────┘  └────────────────────┘  │    │
│  │  └─────────────┘                                            │    │
│  │                                                              │    │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐  │    │
│  │  │  Package    │  │  Std Library  │  │  LLM Cache         │  │    │
│  │  │  Registry   │  │  Runtime      │  │  (response cache)  │  │    │
│  │  └─────────────┘  └──────────────┘  └────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 Key New Interfaces

```rust
/// Distributed execution coordinator trait
trait DistributedCoordinator {
    fn submit_workflow(ir: IrGraph) -> RunId;
    fn poll_status(run_id: RunId) -> RunStatus;
    fn cancel_run(run_id: RunId) -> Result;
    fn list_workers() -> Vec<WorkerInfo>;
}

/// Sandbox provider trait
trait SandboxProvider {
    fn create_sandbox(config: SandboxConfig) -> SandboxId;
    fn execute_tool(sandbox: SandboxId, tool: ToolInvocation) -> ToolResult;
    fn destroy_sandbox(sandbox: SandboxId) -> Result;
    fn get_sandbox_metrics(sandbox: SandboxId) -> SandboxMetrics;
}

/// Telemetry exporter trait
trait TelemetryExporter: Send + Sync {
    fn export_metrics(metrics: Vec<MetricPoint>);
    fn export_traces(traces: Vec<Span>);
    fn export_logs(logs: Vec<LogEntry>);
}
```

---

## 3. Work Packages

### A) Language Completeness (35% of effort)

Close the gap between the Language Reference Manual and the compiler. Implement language features that are **designed in the spec but not yet in the compiler/runtime**.

| # | Feature | Effort | Spec Reference | Description |
|---|---|---|---|---|
| **A1** | `reason` mode | S | LRM Ch. 41 | Implement `reason deep` / `reason fast` / `reason none` as a first-class expression. HIR: `ReasonMode` node. MIR: `ReasonOp` opcode. Runtime: pass reasoning mode to backend. |
| **A2** | `confidence` expressions | M | LRM Ch. 42 | AST exists (`Expr::Confidence`). Implement HIR→MIR lowering for `expr confidence > threshold`. Runtime `Probabilistic` value handling. Decision branching on confidence. |
| **A3** | `deploy` declaration | M | LRM Ch. 48 | New `DeployDecl` in AST, parser, HIR node `DeployTarget`, MIR opcode. Targets: Docker, local, cloud. `lucky deploy` CLI command. |
| **A4** | `when` / reactive events | L | LRM Ch. 68-69 | `When` statement exists in AST. Need HIR `Event` node, event bus in runtime, file watchers (`Filesystem.watch`), git hooks, cron triggers. |
| **A5** | `transaction` blocks | M | LRM Ch. 62 | New AST node `Transaction{body}` with implicit checkpoint + rollback. HIR `TransactionNode` with `rollback` edge. Snapshot isolation in runtime. |
| **A6** | `pub` visibility | S | LRM Ch. 15 | Already partially parsed. Implement visibility enforcement in semantic analysis and package export filtering. |
| **A7** | Custom type declarations | M | LRM Ch. 12-14 | `type` declaration exists as `TypeDecl` in AST. Implement type alias resolution, sum types (enum), product types (struct), recursive types. Full type checker support. |
| **A8** | Stream types | L | Std Lib Ch. 8 | `Stream<T>` type. `Stream::from_iter`, `Stream::from_channel`, `.map`, `.filter`, `.take`, `.batch`. Runtime channel-based implementation. |
| **A9** | Extended pattern matching | M | LRM Ch. 30-33 | Destructuring on custom types, nested patterns, `@` bindings, range patterns, or-patterns. |
| **A10** | Knowledge declarations | S | LRM Ch. 51 | `knowledge` declaration for RAG. HIR `KnowledgeNode`. Runtime: vector store integration. |

#### A1 — reason mode (Small)

```lucky
// What should work after A1:
reason deep        # set deep reasoning for subsequent LLM calls
let answer = ask DeepSeek: analyze this
reason fast        # switch to fast/cheap mode
let summary = ask DeepSeek: summarize

// Or inline:
let result = ai.ask(question, reason = deep)
```

**Implementation:** AST `Reason` expression exists. Add `ReasonMode` field to `LlmCall` HIR node. MIR `ReasonOp` sets a flag in the LLM backend call. DeepSeek adapter: pass `reasoning_effort` parameter. OpenAI adapter: pass `reasoning_effort` parameter.

#### A4 — when / reactive events (Large)

```lucky
// What should work after A4:
when
    files/main.lk changes
    main branch updates
run
    auto_test -> deploy_staging

// Time-based:
when
    every 1h
run
    health_check
```

**Implementation:**
1. **AST:** `When` statement exists, but `conditions` are `Vec<Expr>`. Define typed event condition AST: `FileEvent { path, kind }`, `GitEvent { branch, event_type }`, `TimeEvent { cron_expr | interval }`, `CustomEvent { name }`.
2. **HIR:** `EventNode` with conditions, listener edge, trigger action edge.
3. **Runtime Event Bus:** In-memory pub/sub with `EventBus` struct. `EventEmitter` trait (file watcher, git hook, cron).
4. **Watchers:** Platform-specific file watcher (notify crate on Linux/macOS, native Windows API). Git hook installer. Cron scheduler (`tokio-cron-scheduler` or similar).
5. **`lucky watch` enhancement:** Currently polls `.lk` files. Extend to execute `when` block actions.

#### A5 — transaction blocks (Medium)

```lucky
// What should work after A5:
transaction
    Git.commit("migration")
    Database.migrate("v2")
# if Database.migrate fails, Git.commit is automatically rolled back

task DeployVersion
    input version: Version
    rollback:
        undeploy version
    steps:
        deploy version
```

**Implementation:**
1. **AST:** New `Transaction { body: Block }` statement. `TaskDecl.rollback` already exists in AST.
2. **HIR:** `TransactionNode { body, rollback }`. Checkpoint created at transaction start. On failure, reverse-execute rollback nodes.
3. **Runtime:** Transaction stack in scheduler. `begin_transaction()`, `commit_transaction()`, `rollback_transaction()`.

---

### B) Distributed Runtime (20% of effort)

Take the single-node runtime and make it distributed. This is the single biggest architectural change in v0.3.

| # | Feature | Effort | Spec Reference | Description |
|---|---|---|---|---|
| **B1** | Message bus integration | L | Runtime Ch. 15.5 | NATS-based message passing. IR node serialization over NATS subjects. Worker registration, heartbeats. |
| **B2** | Coordinator service | L | Runtime Ch. 15.3 | Stateless coordinator that owns DAG state, ready queue, and scheduling. Distributes node execution to workers. |
| **B3** | Worker agent | L | Runtime Ch. 15.4 | Worker that receives IR nodes, executes them via the local runtime, returns results. Runs LLM backends and tools. |
| **B4** | Affinity scheduling | M | Runtime Ch. 15.6 | Schedule nodes to workers based on: model affinity (GPU), tool affinity (filesystem locality), data locality (previous output). |
| **B5** | Distributed checkpoint | M | Runtime Ch. 15.7 | Distributed checkpoint store (NATS KV or etcd). Coordinator persists DAG state; workers persist agent memory. |
| **B6** | CLI: `lucky run --dist` | S | — | New flag for distributed execution. Starts coordinator locally, connects to registered workers via config. |

#### B2 — Coordinator Service Architecture

```
┌─────────────────────────────┐
│       Coordinator           │  Stateless, can be replicated
│                             │
│  Per-Run State:            │
│  ┌────────────────┐        │
│  │  DAG State     │        │  Tracks node statuses (Pending/Ready/Running/Done/Failed)
│  │  Ready Queue    │        │  Priority-sorted list of ready nodes
│  │  Worker Pool    │        │  Registered workers + their capabilities
│  │  Dependencies   │        │  Count outstanding deps per node
│  └────────────────┘        │
│                             │
│  Operations:               │
│  - SubmitIR(ir) → RunId    │
│  - PollReadyWorkers()      │
│  - AssignNode(worker, node)│
│  - ReportResult(node, val) │
│  - GetStatus(runId)        │
└─────────────────────────────┘
```

**Key design decision:** Coordinator is **stateless** w.r.t. data. It only tracks DAG metadata. All actual execution state (context, memory, tool results) is stored in a distributed KV store and referenced by handle.

---

### C) Advanced Optimizer & IR (15% of effort)

| # | Feature | Effort | Spec Reference | Description |
|---|---|---|---|---|
| **C1** | GVN pass | M | IR Ch. 22 | Global Value Numbering. Detects redundant computations across basic blocks. |
| **C2** | LICM pass | M | IR Ch. 24 | Loop Invariant Code Motion. Hoists loop-invariant computations. |
| **C3** | Inlining pass | M | IR Ch. 22 | Inline small task/function calls into callers. Heuristic: inline if < 10 nodes. |
| **C4** | AI-specific optimization | L | IR Ch. 25 | LLM call fusion (merge adjacent calls to same model), prompt caching hints, speculative execution of cheap checks before expensive LLM calls. |
| **C5** | Low-level IR (LIR) | L | IR Ch. 16-18 | Third IR level: linear instruction sequence, virtual register allocation, block layout. Bridges MIR to execution. |
| **C6** | Binary IR serialization | M | IR Ch. 28 | `.lkr` binary format: protobuf or flatbuffers. Faster load times, smaller size. |
| **C7** | Critical path analysis | S | IR Ch. 42 | Compute critical path length, identify bottleneck nodes, report at compile time. |

#### C4 — AI-specific Optimizations

```
// Before fusion:
let a = ask DeepSeek: summarize X
let b = ask DeepSeek: categorize X
// After fusion (single LLM call with multi-part prompt):
let {a, b} = ask DeepSeek: summarize X; then categorize the same text

// Speculative execution:
check test_suite_cached?  // cheap check
if not cached
    run_tests             // expensive LLM call
```

---

### D) Observability & Telemetry (10% of effort)

| # | Feature | Effort | Spec Reference | Description |
|---|---|---|---|---|
| **D1** | OpenTelemetry SDK integration | M | Runtime Ch. 14 | Integrate `opentelemetry` Rust SDK. Export metrics and traces via OTLP. |
| **D2** | Structured metrics | M | Runtime Ch. 14.2 | All metrics from spec: node counters, LLM call counters+latency, cost tracking, scheduler queue depth. Prometheus exposition format. |
| **D3** | Distributed tracing | M | Runtime Ch. 14.3 | Span per node execution, parent-child relationships for sub-tasks. Trace ID propagation across distributed workers via NATS headers. |
| **D4** | Structured logging | S | Runtime Ch. 14.4 | JSON-format structured logs. Replace ad-hoc `eprintln!` with structured logger. |
| **D5** | `lucky observe` command | S | — | Real-time dashboard: `lucky observe` opens a TUI or web dashboard showing run status, DAG visualization, cost breakdown, timeline. |

---

### E) Security & Sandboxing (10% of effort)

| # | Feature | Effort | Spec Reference | Description |
|---|---|---|---|---|
| **E1** | Docker sandbox provider | M | Runtime Ch. 10, LTP Ch. 33 | Execute tool calls in ephemeral Docker containers. Filesystem isolation, network policies, resource limits. |
| **E2** | LTP mTLS | M | LTP Ch. 32 | Mutual TLS for LTP connections. Certificate-based client and server authentication. |
| **E3** | OAuth2 token exchange | S | LTP Ch. 30 | OAuth2 device code flow for CLI auth. Token refresh. |
| **E4** | Permission inheritance audit | S | Runtime Ch. 9 | Validate that permission inheritance (lexical scoping, restrict-only semantics) is enforced at runtime, not just compile time. |
| **E5** | Secrets management | S | — | `lucky secret set KEY=value` — secrets encrypted at rest, injected into context as `secret.X` references. Never logged or exposed in audit trails. |

---

### F) Ecosystem & Tooling (10% of effort)

| # | Feature | Effort | Spec Reference | Description |
|---|---|---|---|---|
| **F1** | Package registry server | L | — | Central registry. `lucky pkg publish`, `lucky pkg search`, `lucky pkg install` from registry. Package signing with Ed25519. Version resolution with semver. |
| **F2** | Standard library runtime | L | Std Lib | Implement runtime for standard library: `Bool`, `Int`, `Float`, `String`, `List`, `Map`, `Bytes` methods. `ai` package (ask, summarize, translate, embed, rag). `http`, `time`, `math`, `json`, `crypto` packages. |
| **F3** | Docker deployment | M | — | `lucky deploy docker` — generates Dockerfile, builds image, runs Lucky runtime in container. |
| **F4** | Kubernetes operator | L | — | Custom Kubernetes controller. `lucky` CRD. Manages Lucky workflow runs as K8s Jobs. |
| **F5** | Snapshot testing | S | — | `lucky test --snapshot` — record IR output and compare on subsequent runs. Property-based testing for the type checker. |
| **F6** | Language server enhancements | M | LSP | Richer completions (argument hints, type-aware). Code actions (auto-fix, add imports). Inlay hints (types for inferred bindings). Call hierarchy. |

#### F2 — Standard Library Runtime Architecture

The standard library runtime is a significant new surface. Strategy:

1. **Phase 1 (A2):** Implement **built-in type methods** as native Rust functions. `Bool::not()` maps to `!`. `Int::abs()` maps to `i64::abs()`. Registered in a `StdLibRegistry` at runtime startup.
2. **Phase 2 (A4):** Implement **collection methods** (`List::map`, `List::filter`, `Map::get`, etc.). These operate on `RuntimeValue::List`/`RuntimeValue::Map`.
3. **Phase 3 (A2):** Implement **`ai` package** — the most complex. Each function (`ai.ask`, `ai.summarize`, `ai.rag`) triggers an LLM call through the backend router.
4. **Phase 4 (S):** Implement **`http`, `time`, `math`, `json`, `crypto`** packages via Rust crates (`reqwest`, `chrono`, `serde_json`, `ring`/`sha2`).

```rust
// StdLib runtime registration pattern
pub struct StdLibRegistry {
    packages: HashMap<String, Package>,
}

struct Package {
    functions: HashMap<String, NativeFn>,
    types: HashMap<String, NativeType>,
}

type NativeFn = fn(args: Vec<RuntimeValue>, context: &Context) -> Result<RuntimeValue>;
```

---

## 4. Timeline & Milestones

Total estimated effort: **20-24 weeks** (5-6 months) for 1-2 engineers.

| Milestone | Weeks | Content | Dependencies |
|---|---|---|---|
| **M1 — Language Core** | 1-4 | A1 (reason), A2 (confidence), A3 (deploy), A6 (pub), A9 (patterns) | v0.2 compiler |
| **M2 — Language Advanced** | 5-8 | A4 (when/reactive), A5 (transaction), A7 (types), A8 (streams), A10 (knowledge) | M1 |
| **M3 — Distributed Runtime** | 9-12 | B1 (NATS bus), B2 (coordinator), B3 (worker), B6 (CLI) | v0.2 runtime |
| **M4 — Distributed + Optimizer** | 13-15 | B4 (affinity), B5 (dist checkpoint), C1 (GVN), C2 (LICM), C3 (inlining) | M3 |
| **M5 — Observability + Security** | 16-18 | D1-D5 (telemetry), E1 (Docker sandbox), E2 (mTLS) | M3 |
| **M6 — Ecosystem** | 19-22 | F1 (registry), F2 (std lib runtime), F3 (Docker), F5 (snapshot tests), F6 (LSP) | M1, M2 |
| **M7 — Polish & Release** | 23-24 | C4 (AI opt), C5 (LIR), C6 (binary IR), C7 (critical path), E3-E5, F4 (K8s) + beta test, docs, changelog | All above |

### Dependency Graph

```
M1 ──→ M2 ──→ M6
                │
M3 ──→ M4 ──→ M6 ──→ M7
                │
M5 ────────────┘
```

M1 and M3 can start in parallel (language features don't depend on distributed runtime, and vice versa).

---

## 5. Design Decisions

### D1: NATS as the Message Bus

**Decision:** Use [NATS](https://nats.io) (via `async-nats` crate) for distributed communication, not Kafka or RabbitMQ.

**Rationale:**
- NATS is lightweight (6MB binary), simple (pub/sub + request/reply), and fast.
- JetStream provides persistence and exactly-once delivery for critical messages (checkpoints, audit events).
- NATS has clean Rust bindings (`async-nats`).
- Kafka is overkill for workflow orchestration (we don't need log compaction or massive message retention).

**Alternatives considered:** gRPC (too heavy for per-node messages), MQTT (not designed for request/reply), custom TCP (too much work).

### D2: Binary IR via FlatBuffers

**Decision:** Use [FlatBuffers](https://flatbuffers.dev/) for `.lkr` binary IR, not Protobuf or MessagePack.

**Rationale:**
- Zero-copy deserialization — critical for large IR graphs (1000+ nodes).
- Schema evolution via explicit `id` fields.
- Smaller than JSON (typically 60-70% reduction).
- Rust codegen is mature.

**Migration:** `lucky compile --format binary` / `lucky compile --format json`. JSON remains default for v0.3.

### D3: OpenTelemetry over OTLP

**Decision:** Use OpenTelemetry native OTLP export, not Prometheus pushgateway or custom metrics.

**Rationale:**
- OTLP carries metrics + traces + logs in one protocol.
- Standard industry adoption (Grafana, Datadog, New Relic all ingest OTLP).
- `opentelemetry-rust` is production-ready.

### D4: Docker as Primary Sandbox

**Decision:** Start with Docker for sandbox isolation. Firecracker is a stretch goal (F4).

**Rationale:**
- Docker is ubiquitous, well-documented, and has a mature Rust API (`bollard` crate).
- Firecracker requires VM image management, kernel setup, and is Linux-only.
- Docker provides acceptable isolation for agent tool execution (filesystem, network, resource limits).

### D5: Package Registry Over OCI

**Decision:** Package registry uses OCI-compatible storage (same as Docker registries) with Ed25519 signatures.

**Rationale:**
- OCI is the de-facto standard for artifact storage.
- Any OCI-compatible registry can host Lucky packages (GHCR, Docker Hub, self-hosted).
- No need to build and operate a separate storage infrastructure.

---

## 6. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| **Distributed runtime complexity** | High | High | Start with coordinator + 1 worker locally. Add NATS + multi-worker in second iteration. Test with `lucky run --dist localhost`. |
| **Standard library scope creep** | Medium | Medium | Ship foundational types (String, List, Map) and `ai` package in Phase 1. Defer `http`, `crypto`, `time` to Phase 2. Defer `database` to v0.4. |
| **OpenTelemetry Rust SDK maturity** | Medium | Low | Use `opentelemetry` v0.27+ which is stable. Export to OTLP collector for production. Fallback to stdout JSON in development. |
| **LIR design taking longer than expected** | Medium | Medium | LIR is a stretch goal (M7). If delayed, ship v0.3 without LIR. MIR → JSON → Runtime is sufficient. |
| **Package registry security** | Low | High | Audited signing implementation. Use `ed25519-dalek` (well-reviewed). Registry operations are opt-in; users can use local filesystem packages without the registry. |
| **Backward compatibility breakage** | Low | Medium | CI runs v0.2 test suite against v0.3. Snapshot tests compare IR output. Deprecation warnings in v0.3, removal in v0.4. |

---

## 7. Success Criteria

### Must-Have (v0.3.0 Release)

- [ ] All A1-A10 language features compile, produce correct IR, and execute correctly
- [ ] `lucky run --dist` works with at least 2 workers (local or remote)
- [ ] Distributed workflow with 50+ agents completes without data loss
- [ ] Checkpoint + resume works in distributed mode
- [ ] OpenTelemetry metrics and traces export correctly (verify with `otel-collector` + Jaeger)
- [ ] `lucky observe` shows live DAG visualization (TUI or web)
- [ ] Docker sandbox provider — tool calls execute in isolated containers
- [ ] LTP mTLS — client/server mutual authentication
- [ ] Standard library runtime covers: String, List, Map, Int, Float, Bool, Bytes, Char, `ai` package
- [ ] Package registry: publish, search, install, dependency resolution
- [ ] Snapshot testing: `lucky test --snapshot`
- [ ] GVN + LICM optimizer passes pass correctness tests
- [ ] v0.2 programs compile without changes (backward compatibility)
- [ ] All spec documents updated to v0.3

### Nice-to-Have (v0.3.1+)

- [ ] LIR (Low-level IR) + binary serialization
- [ ] Kubernetes operator
- [ ] K8s sandbox provider
- [ ] AI-specific optimizer (LLM call fusion, prompt caching hints)
- [ ] `http`, `time`, `math`, `json`, `crypto`, `database` stdlib packages
- [ ] Firecracker sandbox provider (Linux only)

### Metrics to Track

| Metric | Target |
|---|---|
| v0.3 programs compile success rate | 100% on valid programs |
| Distributed run vs. single-node latency overhead | < 15% for < 100 nodes |
| Checkpoint serialization time | < 500ms for 1000 nodes |
| Binary IR size vs. JSON | ≤ 40% of JSON size |
| Std lib runtime method coverage vs. spec | 100% for foundational types, 80% for stdlib packages |
| OTel trace throughput | 10,000 spans/sec without backpressure |

---

## Appendix: Updated ROADMAP.md

When v0.3 design is approved, the [ROADMAP.md](../ROADMAP.md) should be updated to include the detailed milestone tables from this document and link to this spec.

---

*Last updated: July 2026 — v0.3 Design Plan*
