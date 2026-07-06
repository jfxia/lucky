## Lucky Compiler v0.2 — Full Pipeline Architecture

```text
Source (.lk)
    │
    ▼
┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
│  Lexer   │ → │  Parser  │ → │ Semantic │ → │   HIR    │ → │   MIR    │ → │ Optimizer│
│ (std)    │   │ (Pratt)  │   │ Analysis │   │ Builder  │   │ Lowering │   │ DCE/Fold │
└──────────┘   └──────────┘   └──────────┘   └──────────┘   └──────────┘   └─────┬────┘
                                        │                          │                │
                                  ┌─────▼─────┐             ┌─────▼─────┐    ┌─────▼─────┐
                                  │   Type    │             │    IR     │    │   JSON    │
                                  │  Checker  │             │ Verifier  │    │  Output   │
                                  └───────────┘             └───────────┘    └───────────┘

Runtime Engine
┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│  Scheduler   │ → │   Executor   │ → │  Checkpoint  │   │    Audit     │
│ (DAG/priority)│  │ (event stream)│  │  (save/load) │   │ (JSONL log)  │
└──────────────┘   └──────────────┘   └──────────────┘   └──────────────┘
        │                   │                   │                   │
  ┌─────▼─────┐      ┌─────▼─────┐      ┌─────▼─────┐      ┌─────▼─────┐
  │  Context  │      │  Memory   │      │  Budget   │      │  Backends │
  │  Manager  │      │  Manager  │      │  Tracker  │      │  Router   │
  └───────────┘      └───────────┘      └───────────┘      └─────┬─────┘
                                                           ┌─────▼─────┐
                                                           │ DeepSeek  │
                                                           │ OpenAI    │
                                                           │ Ollama    │
                                                           └───────────┘
```

## Modules

| Module | Files | Description |
|---|---|---|
| ast/ | 7 | Full AST with spans, 60+ expr variants, all declarations |
| lexer/ | 3 | Hand-written lexer with INDENT/DEDENT processor |
| parser/ | 4 | Recursive descent + Pratt expression parser, full error recovery |
| diagnostics/ | 2 | Diagnostic types, labels, suggestions, ANSI-colored reporter with fix hints |
| semantic/ | 3 | Scoped symbol table, name resolution, type checker |
| hir/ | 2 | 18 HIR node types, edge kinds, graph structure, builder (real nodes, not stubs) |
| mir/ | 3 | SSA basic blocks, 30+ opcodes, terminators, lowering from HIR, optimizer |
| backends/ | 5 | Backend trait, DeepSeek/OpenAI/Ollama adapters, custom TLS 1.2, model routing |
| runtime/ | 9 | DAG scheduler, executor, context, memory, permissions, tools, checkpoint, audit |
| ir_serialize.rs | 1 | HIR → JSON, MIR → JSON serializer |
| ir_verify.rs | 1 | Graph validator: acyclicity, reachability, edge integrity, node references |
| lsp.rs + lsp/ | 2 | LSP server: context-aware completions, real-time diagnostics, hover, go-to-def |
| main.rs | 1 | CLI: 16 commands (init, check, compile, fmt, ir, run, test, debug, pkg, serve, lsp, watch, doc, config) |

## Key Features

- **Zero external dependencies** — builds with only Rust stdlib
- **Full compilation pipeline** — `lucky ir file.lk` produces HIR + MIR JSON with real SSA basic blocks
- **5 optimization passes** — DCE, constant folding, local CSE, copy propagation, LICM (stub)
- **Real LLM backends** — DeepSeek, OpenAI, and Ollama via raw TcpStream with manual HTTP/1.1 and custom TLS 1.2
- **Type checker** — validates type compatibility, agent/task references, tool availability
- **IR verifier** — checks DAG acyclicity, reachability, edge/node integrity before execution
- **Production runtime** — checkpoint/restore, CLI human approval, cost budget enforcement, JSONL audit trail, exponential backoff retry with jitter and circuit breaker
- **LSP server** — context-aware completions (tools, models, task methods, types), real-time diagnostics with 300ms debounce, hover info, go-to-definition, document symbols, semantic tokens
- **Developer experience** — ANSI rich errors with source context and fix suggestions, `lucky watch`, `lucky doc`, `lucky config`
