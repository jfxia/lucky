## Lucky Compiler V0.1 — Full Pipeline Architecture


```text
Source (.lk)
    │
    ▼
┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
│  Lexer   │ → │  Parser  │ → │ Semantic │ → │   HIR    │ → │   MIR    │
│ (std)    │   │ (Pratt)  │   │ Analysis │   │ Builder  │   │ Lowering │
└──────────┘   └──────────┘   └──────────┘   └──────────┘   └──────────┘
                                                                    │
                                                              ┌─────▼─────┐
                                                              │ Optimizer │
                                                              │ DCE/Fold  │
                                                              │ CSE/Copy  │
                                                              └─────┬─────┘
                                                                    │
                                                              ┌─────▼─────┐
                                                              │   JSON    │
                                                              │  Output   │
                                                              └───────────┘
``` 




## Modules Built

| Module          | Files | Description                                                                 |
| --------------- | ----- | --------------------------------------------------------------------------- |
| ast/            | 7     | Full AST with spans, 60+ expr variants, all declarations                    |
| lexer/          | 3     | Hand-written lexer with INDENT/DEDENT processor                             |
| parser/         | 4     | Recursive descent + Pratt expression parser, full error recovery            |
| diagnostics/    | 2     | Diagnostic types, labels, suggestions, human-readable reporter              |
| semantic/       | 2     | Scoped symbol table with 12 symbol kinds, name resolution                   |
| hir/            | 2     | 18 HIR node types, edge kinds, graph structure, builder                     |
| mir/            | 3     | SSA basic blocks, 30+ opcodes, terminator instructions, lowering, optimizer |
| ir_serialize.rs | 1     | HIR → JSON, MIR → JSON serializer                                            |
| lsp.rs          | 1     | LSP-compatible API surface                                                  |
| main.rs         | 1     | CLI: compile, check, tokenize, ir, lsp                                      |

## Key Features
- Zero external dependencies — builds with only Rust stdlib
- Full compilation pipeline: lucky ir file.lk produces HIR + MIR JSON
- 5 optimization passes: Dead code elimination, constant folding, local CSE, copy propagation, LICM (stub)
- SSA-based MIR with basic blocks, block arguments (MLIR-style), structured control flow
- 18 HIR node kinds matching the Lucky IR Specification
- Error recovery — parser continues after errors, produces partial AST + HIR
