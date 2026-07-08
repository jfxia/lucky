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

### A) Language Completeness (35%)

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| A1 | `reason` mode | S | `reason deep` / `reason fast` / `reason none` as first-class expressions | Planned |
| A2 | `confidence` | M | Lower `Expr::Confidence` to HIR/MIR. Runtime `Probabilistic` decision branching | Planned |
| A3 | `deploy` declaration | M | `DeployDecl` in AST. `lucky deploy` CLI. Docker/local/cloud targets | Planned |
| A4 | `when` / reactive events | L | Event bus, file watchers, git hooks, cron. `when X changes run Y` | Planned |
| A5 | `transaction` blocks | M | `Transaction{body}` with checkpoint + automatic rollback | Planned |
| A6 | `pub` visibility | S | Visibility enforcement in semantic analysis + package export | Planned |
| A7 | Custom type declarations | M | `type` aliases, sum types (enum), product types (struct). Full type checker | Planned |
| A8 | Stream types | L | `Stream<T>`. `from_iter`, `from_channel`, map/filter/take/batch | Planned |
| A9 | Extended pattern matching | M | Destructuring, nested patterns, `@` bindings, or-patterns | Planned |
| A10 | Knowledge declarations | S | `knowledge` for RAG. Vector store integration at runtime | Planned |

### B) Distributed Runtime (20%)

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| B1 | NATS message bus | L | NATS pub/sub + request/reply for distributed coordination | Planned |
| B2 | Coordinator service | L | Stateless coordinator owns DAG state, ready queue, scheduling | Planned |
| B3 | Worker agent | L | Receives IR nodes, executes via local runtime, returns results | Planned |
| B4 | Affinity scheduling | M | Schedule nodes to workers by model, tool, or data locality | Planned |
| B5 | Distributed checkpoint | M | NATS KV / etcd-backed DAG + memory checkpointing | Planned |
| B6 | CLI integration | S | `lucky run --dist` flag for distributed execution | Planned |

### C) Advanced Optimizer & IR (15%)

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| C1 | GVN pass | M | Global Value Numbering across basic blocks | Planned |
| C2 | LICM pass | M | Loop Invariant Code Motion | Planned |
| C3 | Inlining pass | M | Inline small task/function calls | Planned |
| C4 | AI-specific optimization | L | LLM call fusion, prompt caching hints, speculative execution | Planned |
| C5 | Low-level IR (LIR) | L | Linear instruction sequence, virtual register allocation | Planned |
| C6 | Binary IR serialization | M | `.lkr` via FlatBuffers — zero-copy, 60% smaller than JSON | Planned |
| C7 | Critical path analysis | S | Compute critical path, identify bottleneck nodes | Planned |

### D) Observability & Telemetry (10%)

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| D1 | OpenTelemetry SDK | M | `opentelemetry-rust` integration, OTLP export | Planned |
| D2 | Structured metrics | M | All spec metrics: counters, histograms, gauges | Planned |
| D3 | Distributed tracing | M | Span per node, parent-child, trace ID across workers | Planned |
| D4 | Structured logging | S | JSON-format structured logs replacing ad-hoc `eprintln!` | Planned |
| D5 | `lucky observe` | S | Real-time TUI/web dashboard: DAG viz, cost, status | Planned |

### E) Security & Sandboxing (10%)

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| E1 | Docker sandbox | M | Tool execution in ephemeral Docker containers | Planned |
| E2 | LTP mTLS | M | Mutual TLS, certificate-based auth for LTP | Planned |
| E3 | OAuth2 CLI auth | S | Device code flow for CLI authentication | Planned |
| E4 | Permission inheritance audit | S | Runtime enforcement of lexical restrict-only permissions | Planned |
| E5 | Secrets management | S | `lucky secret set/get`. Encrypted at rest, injected as `secret.X` | Planned |

### F) Ecosystem & Tooling (10%)

| # | Feature | Effort | Description | Status |
|---|---|---|---|---|
| F1 | Package registry | L | Central OCI-based registry, Ed25519 signing, semver | Planned |
| F2 | Std library runtime | L | Runtime methods for Bool, Int, Float, String, List, Map, Bytes, `ai`, `http` | Planned |
| F3 | Docker deployment | M | `lucky deploy docker` — Dockerfile generation | Planned |
| F4 | Kubernetes operator | L | `lucky` CRD, workflow-as-K8s-Job | Stretch goal |
| F5 | Snapshot testing | S | `lucky test --snapshot`. Property-based type checker tests | Planned |
| F6 | LSP enhancements | M | Inlay hints, code actions, call hierarchy | Planned |

### Proposed Timeline

| Milestone | Weeks | Content | Status |
|---|---|---|---|
| **M1** | 1-4 | Language core: reason, confidence, deploy, pub, pattern matching | Planned |
| **M2** | 5-8 | Language advanced: reactive, transaction, types, streams, knowledge | Planned |
| **M3** | 9-12 | Distributed runtime: NATS bus, coordinator, worker, CLI | Planned |
| **M4** | 13-15 | Distributed checkpoint + affinity, GVN, LICM, inlining | Planned |
| **M5** | 16-18 | Observability (OTel, metrics, tracing, `observe`), Docker sandbox, mTLS | Planned |
| **M6** | 19-22 | Ecosystem: package registry, std lib runtime, Docker deploy, snapshot tests, LSP | Planned |
| **M7** | 23-24 | Polish: AI opt, LIR, binary IR, critical path + K8s operator + release | Planned |

---

*Last updated: July 2026 — v0.2 complete, v0.3 in design*
