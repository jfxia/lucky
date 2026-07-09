# Lucky Language Roadmap

---

## v0.1 — Foundation (Completed)

### Language Specifications

| Document | Status |
|---|---|
| Language Reference Manual (syntax, types, expressions, statements, AI model) | Done |
| Programming Language Specification (design philosophy, core concepts) | Done |
| Runtime Specification (scheduler, memory, concurrency, checkpoints, permissions, security) | Done |
| Standard Library Specification (built-in types, collections, AI primitives, tools, agents) | Done |
| IR Specification (SSA execution graph, optimization passes, serialization, backend API) | Done |
| Tool Protocol Specification (LTP — JSON-RPC for cross-platform execution) | Done |

### Compiler & Runtime

| Component | Status | Description |
|---|---|---|
| Lexer with INDENT/DEDENT | Done | Hand-written state machine, Python-style indentation |
| Parser (Pratt + recursive descent) | Done | Full EBNF grammar, error recovery |
| AST (60+ expression types, 20+ statements) | Done | Spans, patterns, types, all declarations |
| Semantic analysis | Done | Symbol table, scoped name resolution |
| HIR builder | Done | 18 node kinds, edge types, graph structure |
| MIR types | Done | SSA basic blocks, 30+ opcodes, terminators |
| MIR optimizer | Done | DCE, constant folding, CSE, copy propagation |
| IR JSON serialization | Done | HIR → JSON, MIR → JSON |

### Runtime Engine

| Component | Status | Description |
|---|---|---|
| DAG scheduler | Done | Priority queue, topological traversal, depth-weighted |
| Context manager | Done | Immutable layered context propagation |
| Memory manager | Done | Agent-scoped KV store + vector similarity search |
| Permission enforcer | Done | Capability-security, glob matching, allow/deny |
| Tool execution | Done | Filesystem, Shell, Git, HTTP adapters |
| Execution engine | Done | Event stream, summary reporting, step/run API |

### Ecosystem

| Component | Status | Description |
|---|---|---|
| CLI (14 commands) | Done | init, check, compile, fmt, ir, run, test, debug, pkg, serve, lsp |
| Source formatter | Done | Indent normalization, section reordering, comment preservation |
| Test framework | Done | *.test.lk discovery, assertion evaluation, test reports |
| Package manager | Done | Local registry, dependency resolution, lockfile, version constraints |
| LSP server | Done | Completions, hover, go-to-def, references, semantic tokens, diagnostics |
| DAP debugger | Done | Breakpoints, stepping, variable inspection, DAP JSON protocol |
| VS Code extension | Done | Syntax highlighting, 15 snippets, 7 commands, format/lint on save |

### Platform Adapters

| Platform | Status | Description |
|---|---|---|
| Claude Code | Done | MCP tool definitions, stdio server, settings |
| Codex CLI | Done | YAML agent config, Python tool executor |
| OpenCode | Done | Skill definition + Python run scripts |
| Cursor | Done | VS Code extension package |
| Dify | Done | Tool YAML + Python provider + workflow example |
| LTP client (Python) | Done | stdio + HTTP transports, full API coverage |

### Documentation

| Document | Status |
|---|---|
| Quickstart Guide | Done |
| Tutorial (15 chapters) | Done |
| Docs + spec organization | Done |

---

## v0.2 — Production Ready (Completed)

### A) Production-Ready Compiler (30%)

| Feature | Effort | Description | Status |
|---|---|---|---|
| Fix remaining parser edge cases | S | Workflow arrows, commas in tools lists, multi-line strings | **Done** |
| Complete HIR builder | M | Generate real nodes+edges from all declaration types | **Done** |
| Type checker pass | M | Validate type compatibility on data edges, detect undefined refs | **Done** |
| MIR lowering | M | Convert HIR task/workflow nodes to SSA basic blocks with proper CFG | **Done** |
| IR verifier | S | Validate graph acyclicity, reachability, type consistency before execution | **Done** |

### B) Real LLM Backend Integration (25%)

| Feature | Effort | Description | Status |
|---|---|---|---|
| Model adapter trait | S | Abstract interface for `complete()`, `complete_stream()`, health check | **Done** |
| DeepSeek adapter | M | DeepSeek API via custom TLS 1.2 + manual HTTP/1.1 over TcpStream | **Done** |
| OpenAI adapter | M | GPT-4o via HTTP, chat completions | **Done** |
| Ollama adapter | S | Local models via plain HTTP API | **Done** |
| Model routing config | S | `lucky.toml` [models] section with API keys, rate limits, defaults | **Done** |
| Response streaming | M | Stream LLM tokens via `complete_stream()` + `--stream` flag | **Done** |

### C) Developer Experience (25%)

| Feature | Effort | Description | Status |
|---|---|---|---|
| Working LSP completions | M | Keyword completion, agent/task name, type-aware, tools, model, context | **Done** |
| Working LSP diagnostics | M | Real-time errors as you type, 300ms debounce on changes | **Done** |
| Watch mode | S | `lucky watch` polls .lk files, rechecks on change | **Done** |
| Rich error messages | M | ANSI colors, source context with underlines, fix suggestions (Levenshtein) | **Done** |
| `lucky doc` | M | Generate Markdown docs from .lk files (agents, tasks, workflows, tables) | **Done** |
| `lucky config` | S | Show resolved configuration from lucky.toml + environment | **Done** |

### D) Production Runtime (20%)

| Feature | Effort | Description | Status |
|---|---|---|---|
| Checkpoint system | M | Snapshot DAG state + context + memory to JSON disk, `--resume` | **Done** |
| CLI-based human approval | M | `lucky run` pauses for approve/reject/modify, `--auto-approve` | **Done** |
| Cost budget enforcement | S | `--budget USD` tracks and enforces cost limits per LLM call | **Done** |
| Execution audit trail | S | `--audit PATH` JSONL log with timestamps, events, costs, errors | **Done** |
| Retry with actual backoff | S | Exponential backoff + jitter, circuit breaker (5 failures/60s) | **Done** |

### Proposed Timeline

| Milestone | Weeks | Content | Status |
|---|---|---|---|
| **M1** | 1-2 | Compiler fixes + HIR builder complete + type checker | **Done** |
| **M2** | 3-4 | LLM backends (DeepSeek + OpenAI + Ollama) + custom TLS + routing | **Done** |
| **M3** | 5-6 | LSP completions + diagnostics + watch mode | **Done** |
| **M4** | 7-8 | Checkpoint + approval + audit + rich error messages | **Done** |
| **M5** | 9-10 | MIR lowering + streaming + Ollama adapter + `lucky doc` + config | **Done** |

---

## v0.3 — Ecosystem Maturity (In Design)

Full design: [Lucky v0.3 Design Plan](docs/spec/Lucky%20v0.3%20Design%20Plan.md)

### A) Platform Integration — 30% (Top Priority)

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| A1 | LTP Embedding C SDK | M | `lucky.h` + `liblucky.so` — 100KB static lib, pure C99, compile-once link-anywhere | Planned |
| A2 | Python/Node/Rust bindings | M | Auto-generated language bindings: `pip install lucky-sdk`, `npm install lucky-sdk` | Planned |
| A3 | Adapter CI pipelines | M | GitHub Actions for every adapter — compile, submit IR, assert events | Planned |
| A4 | WorkBuddy integration | M | New adapter — plugin or MCP-based for multi-agent code review | Planned |
| A5 | Windsurf / Cline integration | M | New MCP-based adapter. LTP as MCP server | Planned |
| A6 | Integration guide | S | "Add Lucky to your agent tool in 5 minutes" | Planned |
| A7 | LTP MCP bridge | M | Package LTP as MCP server. Works with Claude Desktop, Windsurf, Cline, Continue | Planned |
| A8 | Adapter health dashboard | S | `lucky adapter check` — smoke test each platform adapter | Planned |

### B) Security & Sandboxing — 15%

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| B1 | Docker sandbox | M | Ephemeral containers for tool execution. Filesystem, network, resource isolation | Planned |
| B2 | Runtime permission audit | S | Enforce lexical restrict-only permissions at runtime. Log every allow/deny | Planned |
| B3 | Secrets management | S | `lucky secret set KEY=value`. Encrypted at rest, injected as `secret.X` | Planned |
| B4 | LTP mTLS | M | Optional mutual TLS for LTP connections | Planned |
| B5 | Path traversal protection | S | Filesystem root enforcement — `../../etc/passwd` always denied | Planned |

### C) Standard Library Runtime — 15%

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| C1 | Core type methods | M | Bool, Int, Float, String, Bytes, Char runtime methods | Planned |
| C2 | Collection methods | M | List::map/filter/reduce/sort, Map::get/insert/keys/values | Planned |
| C3 | `ai` package | L | ai.ask, ai.summarize, ai.translate, ai.embed, ai.rag | Planned |
| C4 | `http` package | M | http.get/post/put/delete with retry, timeout, backoff | Planned |
| C5 | `json`, `time`, `math`, `crypto` packages | M | Parse/stringify, temporal ops, math functions, hashing/encryption | Planned |
| C6 | Std library docs | S | Per-function docs published to docs.lucky-lang.org | Planned |

### D) Dynamic Sub-Agent System — 15%

Based on analysis of Solon AI Harness sub-agent patterns: Lucky's static compilation model needs controlled dynamism for sub-agent registration, isolation, and plugin-style loading.

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| D1 | `register agent` statement | M | Dynamic agent registration at runtime. `register agent Foo { prompt "..."; tools ... }`. Compile-time verified, runtime instantiated. | Planned |
| D2 | Sub-session isolation (`isolate`) | M | Explicit context scoping with `isolate`. Each sub-agent gets independent session, inherited context (opt-in), own memory scope. Replaces automatic propagation. | Planned |
| D3 | External agent definitions (mount) | M | `mount agents from "./agents/"` — load agent definitions from YAML/JSON/MD files. Plugin-style extensibility. | Planned |
| D4 | Agent registry runtime API | S | `agents()` built-in to query registered agents. `register agent from "./custom.yaml"`. C SDK binding. | Planned |

### E) Language Completeness — 10%

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| E1 | `reason` mode | S | `reason deep` / `reason fast` / `reason none` for LLM reasoning control | Planned |
| E2 | `deploy` declaration | M | `deploy Docker` / `deploy local`. `lucky deploy` CLI | Planned |
| E3 | `when` / reactive events | L | Event bus, file watchers, git hooks, cron. `when X changes run Y` | Planned |
| E4 | `pub` visibility | S | Visibility enforcement in semantic analysis + package export | Planned |
| E5 | Extended pattern matching | M | Destructuring, nested patterns, `@` bindings, or-patterns | Planned |
| E6 | `transaction` blocks | M | `Transaction{body}` with auto-rollback on failure | Planned |
| E7 | Custom type declarations | M | `type` aliases, sum types (enum), product types (struct) | Planned |

### F) Observability & Telemetry — 10%

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| F1 | Structured SDK events | M | Events carry JSON payloads with labels, costs, errors | Planned |
| F2 | Platform-friendly event format | S | NodeStarted, ApprovalRequired, CostUpdated — designed for platform UI | Planned |
| F3 | Cost tracking in events | S | tokens_prompt, tokens_completion, cost_usd per NodeCompleted | Planned |
| F4 | OpenTelemetry export | M | Optional OTLP export for platforms already using OTel | Planned |
| F5 | `lucky observe` CLI | S | Standalone TUI showing live workflow progress | Planned |

### G) Distributed Runtime — 10%

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| G1 | Simple TCP coordinator | M | No NATS — just TCP + JSON. Good for 2-10 workers | Planned |
| G2 | `lucky run --workers N` | S | Fan out to N local worker processes | Planned |
| G3 | Remote worker | M | `lucky worker --connect host:port` | Planned |
| G4 | Basic affinity | S | Match nodes to workers by capability (GPU, filesystem) | Planned |
| G5 | Distributed checkpoint (local FS) | M | Checkpoint to shared NFS/SMB mount | Planned |

### Proposed Timeline

| Milestone | Weeks | Content | Status |
|---|---|---|---|
| **M1** | 1-4 | Embeddable Runtime: C SDK, language bindings, MCP bridge | Planned |
| **M2** | 5-7 | Platform Proof: adapter CI, WorkBuddy, Windsurf, integration guide | Planned |
| **M3** | 8-10 | Security Foundation: Docker sandbox, audit, secrets, path protection | Planned |
| **M4** | 11-13 | Standard Library: core types, collections, ai/http/json/time/math/crypto | Planned |
| **M5** | 14-16 | Dynamic Sub-Agents: register, isolate, mount, agent registry + Language: reason, deploy, reactive + Observability | Planned |
| **M6** | 17-20 | Distributed + Release: TCP coordinator, workers, affinity, polish | Planned |

M1, M3, M4 run in parallel. M5 pulls in the new Harness-inspired features.

---

## v0.4 — Production Scale (Planned)

Full design: To be drafted based on [v0.3 outcomes and analysis](docs/analysis/harness-subagent-analysis.md)

### A) Advanced Orchestration Patterns

| # | Feature | Effort | Description |
|---|---|---|---|
| A1 | Multi-level delegation | L | Sub-agents can also use `task` tool. Runtime tracks delegation tree (not just flat DAG). `task` tool is composable. |
| A2 | Contract enforcement | M | Runtime validates agent I/O contracts. `output ResearchBrief` — if agent returns wrong shape or low confidence, orchestrator re-delegates. |
| A3 | Auto-rethink / adaptive workflows | M | `policy AdaptivePolicy { rethink on partial_failure; max_rethink 3; escalate_on_stuck }`. Agent can re-plan mid-workflow. |
| A4 | Parallel sub-agent patterns | M | Built-in patterns: `split` (divide input across agents), `aggregate` (merge outputs), `vote` (majority consensus), `refine` (iterative improvement). |

### B) Advanced Optimizer & IR

| # | Feature | Effort | Description |
|---|---|---|---|
| B1 | GVN pass | M | Global Value Numbering across basic blocks. |
| B2 | LICM pass | M | Loop Invariant Code Motion. |
| B3 | Inlining pass | M | Inline small task/function calls (heuristic: < 10 nodes). |
| B4 | Low-level IR (LIR) | L | Linear instruction sequence, virtual register allocation, block layout. Bridges MIR to execution. |
| B5 | Binary IR serialization | M | `.lkr` via FlatBuffers — zero-copy, 60% smaller than JSON. |
| B6 | AI-specific optimization | L | LLM call fusion (merge adjacent calls to same model), prompt caching hints, speculative execution. |
| B7 | Critical path analysis | S | Compute critical path length, identify bottleneck nodes at compile time. |

### C) Ecosystem & Platform

| # | Feature | Effort | Description |
|---|---|---|---|
| C1 | Kubernetes operator | L | `lucky` CRD. Workflow-as-K8s-Job. Native K8s scaling, secrets, networking. |
| C2 | Package registry server | L | Central OCI-based registry. `lucky pkg publish/search/install`. Ed25519 signing. Semver resolution. |
| C3 | Lucky Cloud service | L | Managed Lucky runtime as a service. REST API: `POST /run`, `GET /events`, `POST /approve`. |
| C4 | Confidence expressions | M | `expr confidence > threshold` → HIR/MIR → runtime Probabilistic value branching. |
| C5 | Stream types | L | `Stream<T>`. `from_iter`, `from_channel`, map/filter/take/batch. |
| C6 | Knowledge declarations | S | `knowledge` for RAG. Vector store integration at runtime. |
| C7 | More platform adapters | M | Cline, Continue.dev, JetBrains AI, GitHub Copilot Extensions. |

### D) Advanced Security

| # | Feature | Effort | Description |
|---|---|---|---|
| D1 | Firecracker sandbox | L | VM-level isolation for tool execution. Linux-only. Stronger isolation than Docker. |
| D2 | mTLS everywhere | M | Full mTLS for LTP, SDK, and inter-worker communication. Certificate management CLI. |
| D3 | Audit SIEM integration | M | Structured audit events shipped to SIEM (Splunk, Elastic, Datadog). OTel-compatible. |

### Proposed Timeline

| Milestone | Weeks | Content |
|---|---|---|
| **M1** | 1-6 | Advanced orchestration: delegation, contracts, auto-rethink, parallel patterns |
| **M2** | 7-12 | Optimizer + IR: GVN, LICM, inlining, LIR, binary IR, AI-specific pass |
| **M3** | 13-16 | Ecosystem: K8s operator, package registry, Cloud service |
| **M4** | 17-20 | Language + Security: confidence, streams, knowledge, Firecracker, mTLS, SIEM |
| **M5** | 21-24 | Polish + platform adapters: Cline, Continue, JetBrains, GitHub Copilot + release |

---

*Last updated: July 2026 — v0.2 complete, v0.3 in design (revised with sub-agent features from Harness analysis), v0.4 planned*
