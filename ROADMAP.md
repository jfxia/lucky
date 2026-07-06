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

## v0.3 — Ecosystem Maturity (Future)

| Area | Features |
|---|---|
| **Distributed execution** | Coordinator + worker architecture, NATS message bus, affinity scheduling |
| **Multi-language IR** | IR bindings for Python, JavaScript, Go runtimes |
| **Cloud deployment** | Docker image, Kubernetes operator, managed Lucky service |
| **Observability** | OpenTelemetry metrics, distributed tracing, Prometheus exporter |
| **Advanced optimizer** | GVN (global value numbering), LICM (loop invariant code motion), AI-specific passes (LLM call fusion, prompt caching) |
| **Testing** | Snapshot testing, property-based testing, coverage reports |
| **Security** | mTLS for LTP, OAuth2 integration, sandbox isolation (Docker/Firecracker) |
| **Package registry** | Central registry server, package signing, automated CI/CD for packages |

---

*Last updated: July 2026 — v0.2 complete*
