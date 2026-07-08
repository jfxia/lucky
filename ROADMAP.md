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

### D) Language Completeness — 15%

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| D1 | `reason` mode | S | `reason deep` / `reason fast` / `reason none` for LLM reasoning control | Planned |
| D2 | `deploy` declaration | M | `deploy Docker` / `deploy local`. `lucky deploy` CLI | Planned |
| D3 | `when` / reactive events | L | Event bus, file watchers, git hooks, cron. `when X changes run Y` | Planned |
| D4 | `pub` visibility | S | Visibility enforcement in semantic analysis + package export | Planned |
| D5 | Extended pattern matching | M | Destructuring, nested patterns, `@` bindings, or-patterns | Planned |
| D6 | `transaction` blocks | M | `Transaction{body}` with auto-rollback on failure | Planned |
| D7 | Custom type declarations | M | `type` aliases, sum types (enum), product types (struct) | Planned |

### E) Observability & Telemetry — 10%

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| E1 | Structured SDK events | M | Events carry JSON payloads with labels, costs, errors | Planned |
| E2 | Platform-friendly event format | S | NodeStarted, ApprovalRequired, CostUpdated — designed for platform UI | Planned |
| E3 | Cost tracking in events | S | tokens_prompt, tokens_completion, cost_usd per NodeCompleted | Planned |
| E4 | OpenTelemetry export | M | Optional OTLP export for platforms already using OTel | Planned |
| E5 | `lucky observe` CLI | S | Standalone TUI showing live workflow progress | Planned |

### F) Distributed Runtime — 10%

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| F1 | Simple TCP coordinator | M | No NATS — just TCP + JSON. Good for 2-10 workers | Planned |
| F2 | `lucky run --workers N` | S | Fan out to N local worker processes | Planned |
| F3 | Remote worker | M | `lucky worker --connect host:port` | Planned |
| F4 | Basic affinity | S | Match nodes to workers by capability (GPU, filesystem) | Planned |
| F5 | Distributed checkpoint (local FS) | M | Checkpoint to shared NFS/SMB mount | Planned |

### Deferred to v0.4

GVN / LICM / Inlining, Low-level IR (LIR), Binary IR (FlatBuffers), Kubernetes operator, Package registry server, AI-specific optimizer, Firecracker sandbox, confidence expressions, stream types, knowledge declarations.

### Proposed Timeline

| Milestone | Weeks | Content | Status |
|---|---|---|---|
| **M1** | 1-4 | Embeddable Runtime: C SDK, language bindings, MCP bridge | Planned |
| **M2** | 5-7 | Platform Proof: adapter CI, WorkBuddy, Windsurf, integration guide | Planned |
| **M3** | 8-10 | Security Foundation: Docker sandbox, audit, secrets, path protection | Planned |
| **M4** | 11-13 | Standard Library: core types, collections, ai/http/json/time/math/crypto | Planned |
| **M5** | 14-16 | Language + Observability: reason, deploy, reactive, events, OTel, observe | Planned |
| **M6** | 17-20 | Distributed + Release: TCP coordinator, workers, affinity, polish | Planned |

M1, M3, M4 run in parallel.

---

*Last updated: July 2026 — v0.2 complete, v0.3 in design (revised: platform-first)*
