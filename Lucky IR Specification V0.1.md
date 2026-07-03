# Lucky IR Specification

**Version:** 0.1 Draft
**Status:** Technical Specification
**Target:** Compiler authors, backend adapter developers, IR tooling developers

---

# Table of Contents

```
Part I      IR Architecture

Chapter 1   IR Design Overview
Chapter 2   Three-Level IR Hierarchy
Chapter 3   IR Module Structure
Chapter 4   Type System in IR

----------------------------------------

Part II     High-Level IR (LIR-H)

Chapter 5   Execution DAG
Chapter 6   HIR Nodes
Chapter 7   HIR Edges
Chapter 8   HIR Regions
Chapter 9   HIR Attributes & Metadata

----------------------------------------

Part III    Mid-Level IR (LIR-M) -- SSA Form

Chapter 10  Basic Blocks & Control Flow
Chapter 11  SSA Values & Def-Use Chains
Chapter 12  Instructions
Chapter 13  Regions & Structural CFG
Chapter 14  Phi Nodes
Chapter 15  Dominance & Loop Analysis

----------------------------------------

Part IV     Low-Level IR (LIR-L)

Chapter 16  Linear IR
Chapter 17  Virtual Register Allocation
Chapter 18  Lowering from MIR to LIR

----------------------------------------

Part V      Optimization Passes

Chapter 19  Pass Manager
Chapter 20  Dead Code Elimination
Chapter 21  Constant Folding & Propagation
Chapter 22  Inlining
Chapter 23  Common Subexpression Elimination
Chapter 24  Loop Optimizations
Chapter 25  AI-Specific Optimizations
Chapter 26  Graph-Level Optimizations

----------------------------------------

Part VI     Serialization

Chapter 27  JSON Serialization Format
Chapter 28  Binary Serialization Format (LKR)
Chapter 29  Textual IR Format (LIRT)
Chapter 30  Versioning & Compatibility

----------------------------------------

Part VII    Backend Interoperability

Chapter 31  Backend Adapter Interface
Chapter 32  LLM Backend Mapping
Chapter 33  Tool Backend Mapping
Chapter 34  Local Executor Backend
Chapter 35  Multi-Backend Orchestration
Chapter 36  Custom Backend Development

----------------------------------------

Part VIII   Analysis & Verification

Chapter 37  IR Verifier
Chapter 38  Type Checking in IR
Chapter 39  Use-Def Analysis
Chapter 40  Alias Analysis
Chapter 41  Cost Estimation
Chapter 42  Critical Path Analysis

----------------------------------------

Appendix A  IR Instruction Set Reference
Appendix B  Optimization Pass Catalog
Appendix C  LIR-H Node Type Reference
Appendix D  Serialization Schemas (JSON Schema)
Appendix E  Binary Format Specification
Appendix F  Textual IR Grammar
```

---

# Part I -- IR Architecture

---

## Chapter 1 -- IR Design Overview

### 1.1 Purpose

The Lucky Intermediate Representation (LIR) is the central abstraction between the Lucky frontend (parser, semantic analyzer) and the diverse execution backends (Claude API, Codex CLI, OpenCode, local executors). Its design follows these principles:

1. **Multi-level**: Three distinct IR levels support progressive lowering from high-level agent orchestration down to concrete backend calls.
2. **SSA-based**: The mid-level IR uses Static Single Assignment form, enabling standard compiler optimizations.
3. **Graph-structured**: Both the execution DAG and the intra-procedural CFG are first-class graph structures.
4. **Serializable**: Every IR level has a lossless serialization format (JSON for tooling, binary for performance, textual for debugging).
5. **Verifiable**: The IR carries enough type and structural information to be independently verified.
6. **Backend-neutral**: No IR construct presupposes a specific LLM provider or execution environment.

### 1.2 Relationship to LLVM and MLIR

Lucky IR is *not* built on LLVM or MLIR, but draws inspiration from both:

| Concept | LLVM IR | MLIR | Lucky IR |
|---|---|---|---|
| Basic unit | Function | Operation | Task / Region |
| SSA | Registers + phi | Block arguments | Registers + phi |
| Nesting | None | Regions | Regions (parallel, loop, try) |
| Types | First-class | First-class + dialects | First-class + AI types |
| Serialization | Bitcode | Bytecode + text | JSON + LKR binary + LIRT text |
| Dialects | None (target intrinsic) | Core dialect system | Model dialect, Tool dialect |
| Optimization | Pass pipeline | Pass pipeline | Pass pipeline + graph passes |

Lucky IR is deliberately simpler than MLIR: it does not support user-defined dialects. Instead, it has a fixed set of AI-oriented operations that cover all Lucky language constructs.

### 1.3 Design Invariants

1. **SSA throughout**: Every value in the mid-level IR is defined exactly once. No mutable variables survive beyond the frontend.
2. **Structured control flow**: There is no `goto`. Control flow uses structured constructs (if/else, loop, parallel, try/recover) which lower to basic blocks with explicit terminators.
3. **Explicit effects**: Operations with side effects (LLM calls, tool invocations, I/O) are marked as `effectful`. Pure operations can be freely reordered and eliminated.
4. **Typed edges**: Every data dependency carries an explicit type. Control dependencies are separate from data dependencies.
5. **Provenance tracking**: Every value carries source-location and transformation-history metadata.

---

## Chapter 2 -- Three-Level IR Hierarchy

### 2.1 Level Overview

```
┌─────────────────────────────────────────────────┐
│                Lucky Source (.lk)                │
└────────────────────┬────────────────────────────┘
                     │ Parser + Semantic Analyzer
                     ▼
┌─────────────────────────────────────────────────┐
│   LIR-H (High-Level IR)                          │
│   - Execution DAG nodes (Goal, Workflow, Agent)  │
│   - Coarse-grained: agent/task granularity       │
│   - Edges express orchestration dependencies     │
│   - JSON serialization (.lir)                    │
└────────────────────┬────────────────────────────┘
                     │ HIR-to-MIR Lowering
                     ▼
┌─────────────────────────────────────────────────┐
│   LIR-M (Mid-Level IR)                           │
│   - SSA-based CFG within each task               │
│   - Basic blocks, phi nodes, regions             │
│   - Fine-grained: individual operations          │
│   - Optimization target                          │
│   - Binary serialization (.lkr)                  │
└────────────────────┬────────────────────────────┘
                     │ MIR-to-LIR Lowering
                     ▼
┌─────────────────────────────────────────────────┐
│   LIR-L (Low-Level IR)                           │
│   - Linear instruction sequence                  │
│   - Virtual registers (pre-colored for backends)  │
│   - Backend-specific lowering                    │
│   - Ready for JIT or interpretation              │
└────────────────────┬────────────────────────────┘
                     │ Backend Adapter
                     ▼
┌─────────────────────────────────────────────────┐
│   Backend Execution                              │
│   Claude | GPT | Codex | OpenCode | Local        │
└─────────────────────────────────────────────────┘
```

### 2.2 When Each Level Is Used

| Scenario | IR Level | Rationale |
|---|---|---|
| Save compiled program | HIR | Portable across backends, human-readable JSON |
| Load and execute | HIR → MIR → LIR | Progressive lowering at runtime |
| Apply optimizations | MIR | SSA form enables standard passes |
| JIT execute deterministic tasks | LIR | Linear form, minimal overhead |
| Debug / inspect | HIR (JSON) or MIR (text) | Human-readable |
| Ship to another runtime | HIR | Backend-neutral |
| Cache optimized form | MIR (binary) | Compact, fast to deserialize |

### 2.3 Conversion Guarantees

- **HIR → MIR**: Lossless. Every HIR construct maps to an MIR subgraph. The reverse is not always possible (MIR may be more fine-grained than any HIR source).
- **MIR → LIR**: Lossless for computational semantics. Debug metadata may be stripped.
- **LIR → Execution**: Operationally equivalent. Different backends may produce observably different outputs from the same LIR (because LLM outputs are non-deterministic), but the *control flow* is identical.

---

## Chapter 3 -- IR Module Structure

### 3.1 Module

The top-level container is a **Module**. A module corresponds to a compiled project.

```
Module
├── meta: ModuleMeta          # version, source info, checksum
├── symbols: SymbolTable       # all named entities
├── types: TypePool            # deduplicated type definitions
├── constants: ConstantPool    # deduplicated constant values
├── functions: List<Function>  # task bodies, lambdas
├── agents: List<AgentDef>     # agent metadata
├── graph: Graph               # execution DAG (HIR)
└── attributes: AttributeSet   # module-level attributes
```

### 3.2 Symbol Table

The symbol table maps fully-qualified names to definitions. Every named entity (agent, task, workflow, goal, variable, type) has a unique symbol ID.

```
SymbolTable
├── scopes: Tree<Scope>
│   ├── project scope
│   │   ├── agent "Researcher"
│   │   │   ├── task "Investigate"
│   │   │   └── memory "ResearchMemory"
│   │   └── workflow "Build"
│   │       └── context entry "repo"
│   └── ...
└── by_id: HashMap<SymbolId, SymbolDef>
```

### 3.3 Type Pool

Types are interned in a deduplicated pool. Each type has a unique `TypeId`.

```
TypePool
├── primitives: [Bool, Int, Float, Decimal, String, Bytes, ...]
├── composites: [List<Int>, Map<String, Agent>, ...]
├── ai_types: [Agent<Researcher>, Task<Review>, ...]
├── functions: [fn(Int, String) -> Bool, ...]
└── by_id: HashMap<TypeId, TypeDef>
```

### 3.4 Constant Pool

Constants appearing in the IR are interned in a pool. This enables fast equality checks and reduces serialization size.

```
ConstantPool
├── ints: HashMap<Int, ConstId>
├── floats: HashMap<FloatBits, ConstId>  # raw bits for NaN-safe hashing
├── strings: HashMap<String, ConstId>
├── booleans: [ConstId, ConstId]         # true, false
├── null_like: [ConstId]                 # null
├── unknown_like: [ConstId]              # unknown
└── composites: HashMap<CompositeKey, ConstId>
```

---

## Chapter 4 -- Type System in IR

### 4.1 IR Type Hierarchy

The IR type system is a superset of the Lucky source type system. It adds IR-internal types for control flow and memory.

```
IRType
├── PrimitiveType
│   ├── I1  (Bool)
│   ├── I64 (Int)
│   ├── F64 (Float)
│   ├── D128 (Decimal)
│   ├── StringType, BytesType, CharType
│   └── TimeType, DurationType, UUIDType, URIType, VersionType
├── CompositeType
│   ├── ListType { element: IRType }
│   ├── SetType { element: IRType }
│   ├── MapType { key: IRType, value: IRType }
│   ├── QueueType, StackType
│   ├── TupleType { elements: List<IRType> }
│   └── GraphType { node: IRType, edge: IRType }
├── FunctionType { params: List<IRType>, results: List<IRType> }
├── AIType
│   ├── AgentType { name: SymbolId }
│   ├── TaskType { name: SymbolId }
│   ├── ModelType, PromptType, MemoryType, KnowledgeType
│   ├── ProbabilisticType { inner: IRType }
│   ├── ConfidenceType { inner: IRType, threshold: Float }
│   └── ArtifactType { inner: IRType }
├── ReferenceType
│   ├── NullableType { inner: IRType }
│   └── OptionalType { inner: IRType }
├── UnionType { variants: List<IRType> }
├── ControlType
│   ├── TokenType       # control-flow token (no data)
│   ├── NeverType       # uninhabited (diverging computation)
│   └── RegionType      # region handle
└── BackendType
    ├── LLMCallType { model: SymbolId, prompt: SymbolId }
    ├── ToolCallType { tool: SymbolId, method: String }
    └── ApprovalType { gate: String }
```

### 4.2 Type Serialization

Types serialize to compact descriptors:

```
Primitive:  "i1" | "i64" | "f64" | "d128" | "str" | "bytes" | "char"
            | "time" | "dur" | "uuid" | "uri" | "ver"
Composite:  "list<T>" | "set<T>" | "map<K,V>" | "tuple<T1,T2,...>"
Function:   "fn(P1,P2,...)->R1,R2,..."
AI:         "agent:Researcher" | "task:Review" | "model:Claude"
Nullable:   "T?"
Optional:   "T!"
Union:      "T1|T2|T3"
```

### 4.3 Type Equality and Subtyping

- **Structural equality** for primitives, composites, and functions.
- **Nominal equality** for AI types (match by symbol ID).
- **Subtyping** rules:
  - `T <: T?` (non-nullable to nullable)
  - `T <: T | U` (union introduction)
  - `Never <: T` for all T
  - `T <: Top` for all T (Top is the implicit supertype of all types)
  - Probabilistic subtyping: `Probabilistic<T> @ c1 <: Probabilistic<T> @ c2` if `c1 >= c2`

---

# Part II -- High-Level IR (LIR-H)

---

## Chapter 5 -- Execution DAG

### 5.1 Graph Structure

The HIR represents the entire program as a directed acyclic graph (DAG) of execution nodes. This is the level at which orchestration, scheduling, and dependency analysis occur.

```
Graph
├── nodes: List<Node>          # all nodes in the graph
├── edges: List<Edge>          # all edges
├── entry_points: List<NodeId>  # goal/workflow entry nodes
└── regions: List<Region>       # hierarchical regions
```

### 5.2 Graph Invariants

1. **Acyclicity**: The graph (excluding back-edges of bounded loops) must be acyclic.
2. **Single entry**: Every execution starts from exactly one entry node (the selected Goal or Workflow).
3. **Reachability**: Every node must be reachable from an entry point.
4. **Type consistency**: Every data edge must connect a producer of type `T` to a consumer expecting `T` (or a subtype).
5. **Resource consistency**: Nodes declaring exclusive resources must not overlap in time.
6. **Deterministic topology**: The graph structure, excluding runtime decisions (conditional branches, model routing), is fully determined at compile time.

### 5.3 Graph as a Value

The HIR graph can be stored, transmitted, versioned, and diffed. Two graphs are semantically equivalent if they produce the same observable behavior for all inputs (including non-deterministic LLM outputs bounded by confidence thresholds).

---

## Chapter 6 -- HIR Nodes

### 6.1 Node Envelope

Every HIR node has a common structure:

```json
{
  "id": "uuid",
  "kind": "NodeKind",
  "label": "human-readable label",
  "debug_loc": { "file": "string", "line": int, "column": int },

  "policy": "PolicyRef | inline PolicyDef | null",
  "permissions": "PermissionSetRef | null",

  "resource": {
    "cpu_millicores": 100,
    "memory_mb": 256,
    "timeout_ms": 300000,
    "exclusive": ["resource_name"]
  },

  "cost": {
    "estimated_usd": 0.001,
    "estimated_tokens": 500,
    "estimated_duration_ms": 2000
  },

  "attributes": { "key": "value" }
}
```

### 6.2 Complete Node Catalog

#### GoalNode
Entry point with success criteria.
```
kind: "goal"
goal_ref: SymbolId
success_criteria: [Criterion]
workflow_options: [SymbolId]     # candidate workflows
selected_workflow: SymbolId?     # resolved by policy
context_template: Map<String, Value>
```

#### WorkflowNode
Named subgraph.
```
kind: "workflow"
workflow_ref: SymbolId
body: Region                     # contains sub-nodes
context_template: Map<String, Value>
is_entry: Bool
```

#### TaskNode
Executes a task within an optional agent context.
```
kind: "task"
task_ref: SymbolId
agent_ref: SymbolId?
inputs: Map<String, Value>
expected_outputs: Map<String, TypeRef>
is_stateful: Bool
```

#### AgentNode
Invokes an agent, possibly with model override.
```
kind: "agent_invoke"
agent_ref: SymbolId
method: String
arguments: Map<String, Value>
model_override: SymbolId?
```

#### ToolNode
Invokes a tool method.
```
kind: "tool"
tool_ref: SymbolId
method: String
arguments: Map<String, Value>
```

#### LLMCallNode
Direct LLM invocation (for `ask` and inline prompts).
```
kind: "llm_call"
model_ref: SymbolId
prompt_ref: SymbolId?
messages: List<Message>          # either prompt_ref or inline messages
options: CompleteOptions
```

#### DecisionNode
Conditional branch.
```
kind: "decision"
condition: Value                 # must be Bool
true_branch: Region
false_branch: Region?
```

#### MatchNode
Multi-way branch.
```
kind: "match"
scrutinee: Value
arms: [ { pattern: Pattern, body: Region } ]
default_arm: Region?
```

#### ParallelNode
Concurrent execution with optional barrier.
```
kind: "parallel"
branches: [Region]
strategy: "all" | "any" | "race"
join: Bool                       # true if wait barrier
```

#### LoopNode
Bounded iteration.
```
kind: "loop"
body: Region
max_iterations: Int?
induction_variable: String?
```

#### ForNode
Iteration over a collection or range.
```
kind: "for"
iterator: Value
body: Region
induction_variable: String
```

#### PipelineNode
Data-flow pipeline.
```
kind: "pipeline"
stages: [PipelineStage]
```

```
PipelineStage
├── operation: String
├── arguments: [Value]
└── is_effectful: Bool
```

#### AttemptNode
Error handling block.
```
kind: "attempt"
body: Region
recovery: [RecoveryAction]
```

```
RecoveryAction
├── kind: "retry" | "fallback" | "human" | "abort" | "skip"
├── max_retries: Int?
├── backoff: BackoffStrategy?
├── fallback_region: Region?
├── human_message: String?
└── human_timeout_ms: Int?
```

#### ApprovalNode
Human-in-the-loop gate.
```
kind: "approval"
gate: String                     # "before deploy", "before delete", etc.
message: String
timeout_ms: Int?
escalation: EscalationPolicy?
required_roles: [String]
```

#### LetNode
Immutable binding.
```
kind: "let"
name: String
value: Value
```

#### ReturnNode
Task return.
```
kind: "return"
value: Value?
```

#### NoOpNode
Placeholder, e.g., for empty else-branches.
```
kind: "noop"
```

### 6.3 Pattern Definition

```
Pattern
├── kind: "wildcard" | "variable" | "literal" | "constructor" | "list" | "map"
├── name: String?                # variable binding name
├── value: Value?                # for literal patterns
├── constructor: SymbolId?       # for constructor patterns
├── fields: [Pattern]?           # for constructor patterns
├── elements: [Pattern]?         # for list patterns
└── entries: [(Pattern, Pattern)]?  # for map patterns
```

---

## Chapter 7 -- HIR Edges

### 7.1 Edge Structure

```json
{
  "from": "NodeId",
  "to": "NodeId",
  "kind": "EdgeKind",
  "port": "string?",
  "condition": "string?",
  "metadata": {}
}
```

### 7.2 Edge Kinds

| Kind | Semantics | Example |
|---|---|---|
| `control` | Ordering: `to` executes after `from` completes | `A -> B` workflow arrow |
| `data` | Data flow: `to` reads `from`'s output | `task.output.data → next.input.source` |
| `resource` | Resource dependency: `to` needs a resource held by `from` | Database lock chain |
| `condition` | Conditional activation: `to` only activates if condition holds | Decision branch |
| `context` | Context propagation: `to` inherits context from `from` | Workflow → task |
| `approval` | Approval dependency: `to` awaits human decision | Approval gate → guarded node |
| `error` | Error propagation: `to` is a recovery handler for `from` | Attempt → recovery |
| `cost` | Budget dependency: `to` constrained by `from`'s consumed budget | Parent → child cost limit |

### 7.3 Edge Constraints

- `control` edges must form a DAG (no cycles).
- `data` edges must be type-compatible.
- `resource` edges serialize execution of the connected nodes.
- `condition` edges carry a boolean guard expression.
- `approval` edges suspend `to` until the human approves.
- `error` edges are only traversed when `from` fails.
- `cost` edges propagate budget consumption upward.

---

## Chapter 8 -- HIR Regions

### 8.1 Region Definition

A Region is a container for a subgraph with its own scope. Regions enable hierarchical nesting (parallel branches, loop bodies, recovery blocks).

```
Region
├── id: RegionId
├── kind: RegionKind
├── nodes: [Node]
├── edges: [Edge]                # edges with both endpoints inside the region
├── entry: NodeId                # first node to execute within the region
├── exit: NodeId?                # last node (for sequential regions)
├── context: Map<String, Value>  # region-local context
└── policy: PolicyDef?           # region-level policy override
```

### 8.2 Region Kinds

| Kind | Description |
|---|---|
| `sequential` | Nodes execute in dependency order |
| `parallel_all` | All branches execute concurrently; join after all complete |
| `parallel_any` | Branches execute concurrently; join after first completes |
| `parallel_race` | Branches execute concurrently; join after first finishes |
| `loop_body` | Body of a loop (may have back-edges for induction) |
| `recovery` | Error recovery handler |
| `branch` | One arm of a conditional or match |
| `transaction` | Atomic region (all-or-nothing) |

### 8.3 Region Scoping

Regions create nested scopes for:
- Variable bindings (let nodes within a region are scoped to it)
- Context entries (region context shadows outer context)
- Permissions (regions may further restrict but not expand permissions)
- Cost budgets (regions may have sub-budgets)

---

## Chapter 9 -- HIR Attributes & Metadata

### 9.1 Attribute Model

Attributes are key-value pairs attached to any IR entity (module, function, node, edge, region). They carry metadata that does not affect execution semantics but is used by tooling, optimization, and debugging.

```
Attribute
├── key: String                  # dotted namespace: "lucky.opt.level"
├── value: AttributeValue
└── source: "user" | "compiler" | "optimizer" | "backend"
```

### 9.2 Standard Attribute Namespaces

| Namespace | Purpose |
|---|---|
| `lucky.debug.*` | Source locations, variable names |
| `lucky.opt.*` | Optimization hints (inline, nounroll, etc.) |
| `lucky.cost.*` | Cost model overrides |
| `lucky.policy.*` | Policy annotations |
| `lucky.backend.*` | Backend-specific hints |
| `lucky.verify.*` | Verification annotations |
| `lucky.doc.*` | Documentation strings |

### 9.3 Attribute Value Types

```
AttributeValue = String | Int | Float | Bool | List<AttributeValue>
               | Map<String, AttributeValue> | SymbolId | TypeId
```

---

# Part III -- Mid-Level IR (LIR-M) -- SSA Form

---

## Chapter 10 -- Basic Blocks & Control Flow

### 10.1 Function Structure

In the MIR, each task body and lambda becomes a `Function`. A function is a control-flow graph (CFG) of basic blocks.

```
Function
├── id: FuncId
├── name: String
├── signature: FunctionType
├── entry_block: BlockId
├── blocks: List<BasicBlock>
├── regions: List<Region>        # nested structural regions
└── attributes: AttributeSet
```

### 10.2 Basic Block

A basic block is a linear sequence of instructions with no internal control flow. It has:
- Zero or more **block arguments** (for SSA phi-like semantics)
- A list of **instructions**
- Exactly one **terminator instruction**

```
BasicBlock
├── id: BlockId
├── arguments: [(String, IRType)]  # block parameters (SSA)
├── instructions: [Instruction]    # body instructions
├── terminator: TerminatorInst     # exactly one
└── predecessors: [BlockId]        # computed, not stored
```

### 10.3 Terminator Instructions

Every basic block ends with exactly one terminator:

| Terminator | Description |
|---|---|
| `br succ` | Unconditional branch |
| `cond_br cond, true_succ, false_succ` | Conditional branch |
| `switch val, default, [(case, succ)]` | Multi-way branch |
| `ret [value]` | Return from function |
| `invoke_region region, succ, err_succ` | Enter a region |
| `yield value` | Exit a region with a value |
| `unreachable` | Marks unreachable code |
| `abort` | Terminate execution |

### 10.4 CFG Properties

- The entry block has no predecessors.
- Every block is reachable from the entry block (unreachable blocks are removed by a cleanup pass).
- The CFG is reducible (all loops have a single entry point, the loop header).
- Critical edges (edges from a block with multiple successors to a block with multiple predecessors) are split by the frontend.

### 10.5 Block Arguments vs Phi Nodes

Lucky MIR uses **block arguments** instead of explicit phi instructions. This is the MLIR approach:

```
# Instead of:
#   bb2:
#     %x = phi [%a, bb0], [%b, bb1]

# Lucky MIR uses:
#   bb2(%x: I64):
#     ...

# Callers pass arguments:
#   br bb2(%a)       from bb0
#   br bb2(%b)       from bb1
```

This representation is cleaner, avoids the need for a separate phi instruction type, and simplifies SSA destruction.

---

## Chapter 11 -- SSA Values & Def-Use Chains

### 11.1 Value Identity

In MIR, every value is identified by a tuple of `(BlockId, index)` where `index` is either:
- An instruction index within the block (for instruction results), or
- A block argument index (for block parameters), or
- A special index for the block terminator's result (if any)

This is called the **value handle**.

```
ValueHandle = (BlockId, ValueIndex)
```

### 11.2 Def-Use Chain Construction

The IR maintains explicit def-use chains for fast analysis:

```
DefUseChain
├── definitions: HashMap<ValueHandle, Instruction>     # def → defining inst
├── uses: HashMap<ValueHandle, List<(BlockId, InstIndex, OperandIndex)>>  # def → use sites
└── block_arg_users: HashMap<(BlockId, ArgIndex), List<(BlockId, InstIndex, OperandIndex)>>
```

Def-use chains are:
- **Computed once** after IR construction (and incrementally maintained by transformations).
- **Invalidated** by any pass that modifies the IR. The pass manager re-computes them as needed.
- **Available** to all analysis passes via the `AnalysisContext`.

### 11.3 SSA Verification

The verifier checks:
1. Every use refers to a value that dominates the use site.
2. No value is defined more than once.
3. Block arguments are only referenced within their block.
4. All uses are within the same function (no cross-function references).
5. Every defined value has at least one use (after DCE), or is marked `has_side_effect`.

### 11.4 Value Naming

For debugging and textual IR, values can have optional names:

```
%42 = add %x, %y          # unnamed: %42
%result = add %x, %y      # named: %result
```

Names are not preserved through optimization (they are debug metadata). Value identity is always via `(BlockId, index)`.

---

## Chapter 12 -- Instructions

### 12.1 Instruction Structure

```
Instruction
├── opcode: Opcode
├── operands: [(ValueHandle | ConstId), ...]
├── result_type: IRType
├── attributes: AttributeSet
├── debug_loc: DebugLoc?
└── flags: InstructionFlags
```

```
InstructionFlags
├── has_side_effect: Bool    # true for LLM calls, tool invocations, I/O
├── is_terminator: Bool      # true for terminator instructions
├── is_throw: Bool           # true for operations that may fail
├── is_volatile: Bool        # true for operations that must not be reordered
├── is_cached: Bool          # true if result is cacheable
├── is_idempotent: Bool      # true if repeated calls produce same result
└── is_speculative: Bool     # true if safe to execute speculatively
```

### 12.2 Instruction Set

#### Arithmetic / Logical

| Opcode | Operands | Result | Description |
|---|---|---|---|
| `add` | a: T, b: T | T | Addition (Int/Float/Decimal) |
| `sub` | a: T, b: T | T | Subtraction |
| `mul` | a: T, b: T | T | Multiplication |
| `div` | a: T, b: T | T | Division |
| `rem` | a: Int, b: Int | Int | Remainder |
| `neg` | a: T | T | Negation |
| `abs` | a: T | T | Absolute value |
| `and` | a: Bool, b: Bool | Bool | Logical AND |
| `or` | a: Bool, b: Bool | Bool | Logical OR |
| `not` | a: Bool | Bool | Logical NOT |
| `xor` | a: Int, b: Int | Int | Bitwise XOR |
| `shl` | a: Int, b: Int | Int | Shift left |
| `shr` | a: Int, b: Int | Int | Arithmetic shift right |
| `and_bits` | a: Int, b: Int | Int | Bitwise AND |
| `or_bits` | a: Int, b: Int | Int | Bitwise OR |

#### Comparison

| Opcode | Operands | Result | Description |
|---|---|---|---|
| `eq` | a: T, b: T | Bool | Equality |
| `neq` | a: T, b: T | Bool | Inequality |
| `lt` | a: T, b: T | Bool | Less than |
| `le` | a: T, b: T | Bool | Less or equal |
| `gt` | a: T, b: T | Bool | Greater than |
| `ge` | a: T, b: T | Bool | Greater or equal |

#### Memory / Allocation

| Opcode | Operands | Result | Description |
|---|---|---|---|
| `alloca` | type: IRType | Ref<T> | Stack allocation |
| `load` | ptr: Ref<T> | T | Load from memory |
| `store` | ptr: Ref<T>, val: T | () | Store to memory |
| `gep` | ptr: Ref<T>, indices: [Int] | Ref<U> | Get element pointer |
| `memcpy` | dst, src, size | () | Memory copy |
| `memzero` | ptr, size | () | Zero memory |

#### Collection Operations

| Opcode | Operands | Result | Description |
|---|---|---|---|
| `list_new` | elements: [T] | List<T> | Create list |
| `list_get` | list: List<T>, idx: Int | T? | Element access |
| `list_set` | list: List<T>, idx: Int, val: T | List<T> | Functional update |
| `list_len` | list: List<T> | Int | Length |
| `list_concat` | a: List<T>, b: List<T> | List<T> | Concatenation |
| `map_new` | entries: [(K,V)] | Map<K,V> | Create map |
| `map_get` | map: Map<K,V>, key: K | V? | Lookup |
| `map_insert` | map: Map<K,V>, k: K, v: V | Map<K,V> | Functional insert |
| `set_new` | elements: [T] | Set<T> | Create set |
| `set_contains` | set: Set<T>, elem: T | Bool | Membership |
| `set_insert` | set: Set<T>, elem: T | Set<T> | Functional insert |

#### String / Bytes

| Opcode | Operands | Result | Description |
|---|---|---|---|
| `str_concat` | a: String, b: String | String | Concatenation |
| `str_len` | s: String | Int | Byte length |
| `str_slice` | s: String, start: Int, end: Int | String | Substring |
| `str_find` | haystack: String, needle: String | Int? | Find |
| `str_replace` | s, from, to | String | Replace all |
| `str_split` | s, delim | List<String> | Split |
| `str_interpolate` | template: String, args: Map<String, Any> | String | Interpolation |
| `bytes_len` | b: Bytes | Int | Length |
| `bytes_slice` | b: Bytes, start, end | Bytes | Sub-slice |
| `bytes_concat` | a: Bytes, b: Bytes | Bytes | Concatenation |

#### Control Flow (Terminators)

| Opcode | Operands | Description |
|---|---|---|
| `br` | dest: BlockId, args: [Value] | Unconditional branch |
| `cond_br` | cond: Bool, true_dest, false_dest, true_args, false_args | Conditional branch |
| `switch` | val: Int, default: BlockId, cases: [(Int, BlockId)], args | Multi-way |
| `ret` | val: Value? | Return |
| `invoke_region` | region: RegionId, success_dest, error_dest, args | Enter region |
| `yield` | val: Value | Exit region with value |
| `unreachable` | -- | Unreachable |
| `abort` | reason: String | Abort execution |

#### AI Operations (Effectful)

| Opcode | Operands | Result | Description |
|---|---|---|---|
| `llm_complete` | model: SymId, messages: List<Message>, opts: CompleteOpts | Probabilistic<String> | LLM completion |
| `llm_chat` | model: SymId, messages: List<Message>, opts | Message | Multi-turn |
| `llm_stream` | model: SymId, messages, opts | Stream<String> | Streaming |
| `prompt_render` | prompt: SymId, vars: Map<String,Any> | String | Render prompt |
| `tool_invoke` | tool: SymId, method: String, args: Map<String,Any> | Result<Any> | Tool call |
| `agent_invoke` | agent: SymId, task: String, args: Map<String,Any> | Result<Any> | Agent task |
| `memory_remember` | mem: SymId, key: String, val: Any, emb: Embedding? | () | Store |
| `memory_recall` | mem: SymId, key: String | Any? | Retrieve |
| `memory_similar` | mem: SymId, emb: Embedding, limit: Int | List<(String,Any,Float)> | K-NN |
| `knowledge_search` | kb: SymId, query: String, top_k: Int | List<Chunk> | RAG search |
| `knowledge_ask` | kb: SymId, query: String, model, top_k | Answer | RAG query |
| `approval_request` | gate: String, msg: String, timeout: Duration | Approval | Human gate |
| `approval_wait` | approval: Approval | Bool | Wait for decision |
| `embed_generate` | model: SymId, text: String | Embedding | Generate embedding |

#### Pipeline / Stream

| Opcode | Operands | Result | Description |
|---|---|---|---|
| `pipe_next` | stream: Stream<T> | T? | Pull next element |
| `pipe_map` | stream: Stream<T>, fn: FuncId | Stream<U> | Map |
| `pipe_filter` | stream: Stream<T>, fn: FuncId | Stream<T> | Filter |
| `pipe_take` | stream: Stream<T>, n: Int | Stream<T> | Take N |
| `pipe_collect` | stream: Stream<T> | List<T> | Collect to list |
| `pipe_for_each` | stream: Stream<T>, fn: FuncId | () | Consume |
| `chan_send` | chan: Channel<T>, val: T | () | Send |
| `chan_recv` | chan: Channel<T> | T? | Receive |

#### Type Conversion

| Opcode | Operands | Result | Description |
|---|---|---|---|
| `cast` | val: T, target: IRType | U | Type cast |
| `int_to_float` | val: Int | Float | Int → Float |
| `float_to_int` | val: Float | Int | Float → Int (trunc) |
| `to_string` | val: T | String | Stringify |
| `parse_int` | s: String | Int? | Parse |
| `parse_float` | s: String | Float? | Parse |
| `unwrap` | result: Result<T,E> | T | Unwrap (may trap) |
| `wrap_ok` | val: T | Result<T,E> | Wrap success |
| `wrap_err` | err: E | Result<T,E> | Wrap error |
| `is_ok` | result: Result<T,E> | Bool | Check success |
| `unwrap_prob` | prob: Probabilistic<T>, threshold: Float | T? | Threshold unwrap |

#### Reflection / Runtime

| Opcode | Operands | Result | Description |
|---|---|---|---|
| `type_of` | val: Any | IRType | Get runtime type |
| `symbol_lookup` | name: String | SymId? | Dynamic symbol lookup |
| `checkpoint` | tag: String | () | Trigger checkpoint |
| `cost_query` | -- | CostReport | Query current cost |
| `log` | level: Int, msg: String, data: Map | () | Emit log |
| `sleep` | dur: Duration | () | Sleep |

### 12.3 Instruction Operands

Operands can be:
- **SSA values**: `(BlockId, ValueIndex)` referencing an instruction result or block argument
- **Constants**: `ConstId` referencing the constant pool
- **Symbols**: `SymbolId` referencing the symbol table
- **Types**: `TypeId` referencing the type pool
- **Functions**: `FuncId` referencing another function
- **Regions**: `RegionId` referencing a region
- **Immediates**: Small integers encoded directly in the instruction

---

## Chapter 13 -- Regions & Structural CFG

### 13.1 Region Model

In MIR, structured control flow constructs (parallel, loop, attempt/recover) are represented as **regions** -- nested subgraphs with their own entry/exit blocks.

```
Region
├── id: RegionId
├── kind: RegionKind
├── blocks: List<BlockId>       # blocks belonging to this region
├── entry: BlockId              # entry block
├── exit: BlockId?              # exit block (for yield)
├── attached_to: InstIndex      # instruction that owns this region
└── parent_region: RegionId?
```

### 13.2 Region-Bearing Instructions

These instructions carry attached regions:

| Instruction | Regions | Semantics |
|---|---|---|
| `parallel` | 1 region per branch | Execute regions concurrently |
| `loop` | 1 region (body) | Repeat while condition holds |
| `for_each` | 1 region (body) | Iterate over collection |
| `attempt` | 2 regions (body, recovery) | Try body; on failure, run recovery |
| `if` | up to 2 regions (then, else) | Conditional execution |
| `match` | 1 region per arm + default | Pattern-match dispatch |

### 13.3 Region Entry and Exit

- A region is entered via `invoke_region` in the parent block.
- Within the region, execution begins at the entry block.
- A region yields a value via `yield` in an exit block.
- The parent resumes at the `invoke_region`'s success successor.

### 13.4 Example: Attempt/Recover

```
# Lucky source:
# attempt
#     risky_op()
# recover
#     fallback_op()

bb0:
    %0 = invoke_region @attempt_body, success=bb3, error=bb2
bb2:  # error path
    %1 = invoke_region @recovery_body, success=bb3, error=bb4
bb3:  # success (from either body or recovery)
    ret
bb4:  # recovery also failed
    abort "unrecoverable error"

@attempt_body:
    bb5:
        %2 = call @risky_op()
        yield %2

@recovery_body:
    bb6:
        %3 = call @fallback_op()
        yield %3
```

---

## Chapter 14 -- Phi Nodes

### 14.1 Block Arguments as Phi

Lucky MIR uses block arguments for SSA merging, eliminating the need for explicit phi instructions.

```
# C-like:
#   if (cond) { x = 1; } else { x = 2; }
#   use(x);

bb0:
    cond_br %cond, bb1, bb2

bb1:
    br bb3(1)    # pass value 1 as block argument

bb2:
    br bb3(2)    # pass value 2 as block argument

bb3(%x: I64):    # %x is the merged value
    call @use(%x)
    ret
```

### 14.2 Multiple Block Arguments

A block can accept multiple arguments, which is common after merging from parallel regions:

```
bb_join(%result_a: String, %result_b: Int, %result_c: Bool):
    call @combine(%result_a, %result_b, %result_c)
    ret
```

### 14.3 Phi-Less Design Benefits

1. **Single definition site**: Block arguments are defined at the block header, not scattered through predecessor blocks.
2. **Simpler CFG manipulation**: Adding/removing predecessors doesn't require updating phi operand lists.
3. **Cleaner SSA destruction**: Block arguments become regular register assignments during out-of-SSA lowering.
4. **Easier verification**: Dominance of block arguments is checked at block boundaries.

---

## Chapter 15 -- Dominance & Loop Analysis

### 15.1 Dominator Tree

The IR framework computes and maintains the dominator tree:

```
DominatorTree
├── idom: HashMap<BlockId, BlockId>        # immediate dominators
├── children: HashMap<BlockId, List<BlockId>>  # tree structure
├── dominates(a: BlockId, b: BlockId) -> Bool
├── strictly_dominates(a, b) -> Bool
└── nearest_common_dominator(a, b) -> BlockId
```

Computed using the standard Lengauer-Tarjan algorithm. Re-computed after any CFG modification.

### 15.2 Dominance Frontier

The dominance frontier is used for SSA construction and is computed on demand:

```
DominanceFrontier
├── frontier: HashMap<BlockId, Set<BlockId>>
└── iterated_frontier(blocks: Set<BlockId>) -> Set<BlockId>
```

### 15.3 Loop Analysis

The loop nesting forest is computed from the dominator tree:

```
LoopInfo
├── loops: List<Loop>
└── Loop
    ├── header: BlockId
    ├── blocks: List<BlockId>
    ├── depth: Int
    ├── parent: Loop?
    ├── is_innermost: Bool
    ├── back_edges: List<(BlockId, BlockId)>
    └── induction_variables: List<InductionVar>
```

```
InductionVar
├── phi_value: ValueHandle
├── start: ConstId
├── step: ConstId
├── end: ConstId?
└── is_canonical: Bool    # true for simple i = start; i < end; i += step
```

### 15.4 Post-Dominance

Post-dominance is used for control-dependence analysis:

```
PostDominatorTree
├── ipdom: HashMap<BlockId, BlockId>
├── control_dependence: HashMap<BlockId, Set<BlockId>>
└── control_equivalent(a, b) -> Bool
```

---

# Part IV -- Low-Level IR (LIR-L)

---

## Chapter 16 -- Linear IR

### 16.1 Design

The low-level IR (LIR-L) is a flat, linear sequence of instructions with virtual registers. It eliminates:
- Basic blocks (control flow becomes explicit jumps with labels)
- Block arguments (replaced by register assignments at join points)
- Regions (inlined or outlined)

LIR-L is the final IR before backend-specific code generation.

### 16.2 Linear Function

```
LinearFunction
├── id: FuncId
├── name: String
├── num_regs: Int                    # v0..vN (virtual registers)
├── instructions: List<LInst>        # linear instruction list
├── labels: HashMap<String, Int>     # label → instruction index
├── constants: [ConstId]             # referenced constants
└── debug_info: List<DebugLoc?>      # per-instruction debug info
```

### 16.3 Linear Instruction

```
LInst
├── opcode: LOpcode
├── result: RegId?                   # destination virtual register
├── operands: [LValue]               # source values
├── flags: LInstFlags
└── debug_loc: DebugLoc?
```

```
LValue = Reg(RegId) | Const(ConstId) | Label(String) | Func(FuncId) | Global(String)
```

### 16.4 Linear Opcodes

| Opcode | Description |
|---|---|
| `mov dst, src` | Register copy |
| `const dst, ConstId` | Load constant |
| `add dst, a, b` | Integer addition |
| `fadd dst, a, b` | Float addition |
| `dadd dst, a, b` | Decimal addition |
| `sub`, `mul`, `div`, `fsub`, `fmul`, `fdiv` | Arithmetic |
| `cmp_eq dst, a, b` | Compare equal |
| `cmp_lt dst, a, b` | Compare less |
| `br label` | Unconditional jump |
| `br_cond cond, tlabel, flabel` | Conditional jump |
| `call dst, func, args...` | Call function |
| `ret [val]` | Return |
| `llm_call dst, model, messages, opts` | LLM call (black-box) |
| `tool_call dst, tool, method, args` | Tool call (black-box) |
| `mem_read dst, ptr, offset` | Memory read |
| `mem_write ptr, offset, val` | Memory write |
| `list_get dst, list, idx` | List access |
| `map_get dst, map, key` | Map access |
| `yield val` | Yield from region |
| `enter_region region_id` | Enter nested region |
| `leave_region` | Leave nested region |
| `checkpoint tag` | Trigger checkpoint |
| `log level, msg` | Emit log |
| `nop` | No operation |

### 16.5 Virtual Register Conventions

- `v0`..`vN` are virtual registers (unlimited, SSA within the linear IR).
- Register allocation (mapping virtual to physical/machine registers) is performed by the backend adapter, not by the IR framework.
- The IR provides liveness information (def-use intervals) to assist the backend.

---

## Chapter 17 -- Virtual Register Allocation

### 17.1 Liveness Analysis

The IR framework computes live intervals for all virtual registers:

```
LiveInterval
├── reg: RegId
├── def_point: Int           # instruction index where defined
├── use_points: [Int]         # instruction indices where used
├── live_range: (Int, Int)    # [first_use_or_def, last_use]
├── is_spillable: Bool
└── preferred_color: Int?     # from backend hints
```

### 17.2 Spilling Hints

The IR annotates registers with spill preferences:

- `prefer_reg`: This value is used frequently; prefer a machine register.
- `prefer_spill`: This value is used infrequently; spilling is acceptable.
- `forbidden_spill`: This value must not be spilled (e.g., it's used inside a tight loop or is an LLM response being streamed).

### 17.3 Backend Register Mapping

Backend adapters declare their register constraints:

```
BackendRegConfig
├── num_gp_regs: Int     # general-purpose registers
├── num_fp_regs: Int     # floating-point registers
├── reserved: [Int]       # reserved by the backend
├── callee_saved: [Int]   # preserved across calls
└── special: HashMap<String, Int>  # special-purpose registers
```

The backend adapter is responsible for final register allocation using this information.

---

## Chapter 18 -- Lowering from MIR to LIR

### 18.1 Lowering Phases

```
MIR Function
    │
    ▼
[1. Region Inlining]
    │  Inline parallel/loop/attempt regions into the parent CFG
    ▼
[2. Block Argument Lowering]
    │  Replace block arguments with explicit register moves at join points
    ▼
[3. CFG Linearization]
    │  Order blocks, insert explicit jumps, create label table
    ▼
[4. SSA Destruction]
    │  Replace phi-equivalents with register copies; coalesce copies
    ▼
[5. Instruction Selection] (backend-specific)
    │  Map generic MIR opcodes to LIR-L opcodes
    ▼
[6. Peephole Optimization]
    │  Local pattern-based improvements
    ▼
LIR-L Function
```

### 18.2 Region Inlining

Each structured region becomes a sub-CFG with explicit enter/exit:

```
# Before: invoke_region @parallel_body
# After:
#   fork bb_parallel_entry
#   (control splits to parent and parallel body)
#   ...
#   join bb_after_parallel
```

### 18.3 Block Argument Lowering

```
# Before:
#   bb1: br bb3(%val)
#   bb3(%x: I64): use(%x)

# After:
#   bb1: mov %merge_tmp, %val; br bb3
#   bb3: %x = mov %merge_tmp; use(%x)
```

A register coalescing pass eliminates the redundant moves.

---

# Part V -- Optimization Passes

---

## Chapter 19 -- Pass Manager

### 19.1 Pass Infrastructure

The pass manager orchestrates the execution of optimization and analysis passes.

```
PassManager
├── passes: List<Pass>
├── analysis_context: AnalysisContext
├── run(module: Module) -> Module
└── run_function(func: Function) -> Function
```

### 19.2 Pass Types

| Pass Type | Scope | Description |
|---|---|---|
| `ModulePass` | Whole module | Inter-procedural optimizations |
| `FunctionPass` | Single function | Intra-procedural optimizations |
| `RegionPass` | Single region | Nested-structure optimizations |
| `GraphPass` | Execution DAG | Orchestration-level optimizations |
| `AnalysisPass` | Any | Compute analysis results for other passes |

### 19.3 Pass Registration

```python
pass_manager = PassManager()

# Canonicalization (always run first)
pass_manager.add(CanonicalizePass())

# SSA optimization pipeline
pass_manager.add(ConstantFoldingPass())
pass_manager.add(SCCPPass())              # Sparse Conditional Constant Propagation
pass_manager.add(DeadCodeEliminationPass())
pass_manager.add(CommonSubexpressionEliminationPass())
pass_manager.add(LoopInvariantCodeMotionPass())
pass_manager.add(StrengthReductionPass())

# AI-specific
pass_manager.add(LLMCallFusionPass())
pass_manager.add(PromptCachingPass())
pass_manager.add(SpeculativeExecutionPass())

# Graph-level
pass_manager.add(TaskFusionPass())
pass_manager.add(ParallelismDiscoveryPass())
```

### 19.4 Pass Ordering

Passes declare dependencies:

```
Pass
├── name: String
├── required_analyses: [AnalysisId]    # analyses this pass needs
├── invalidated_analyses: [AnalysisId] # analyses this pass destroys
├── preserves_cfg: Bool
└── run(module | function | region) -> bool  # returns true if IR changed
```

The pass manager automatically schedules required analyses (lazily, on first use) and invalidates them when a pass modifies the IR.

### 19.5 Analysis Cache

Analyses are computed on demand and cached:

```
AnalysisContext
├── get<DominatorTree>(func: FuncId) -> &DominatorTree
├── get<LoopInfo>(func: FuncId) -> &LoopInfo
├── get<DefUseChain>(func: FuncId) -> &DefUseChain
├── invalidate(func: FuncId)              # drop all cached analyses for function
└── invalidate_all()                      # drop everything
```

---

## Chapter 20 -- Dead Code Elimination

### 20.1 Dead Instruction Elimination (DIE)

Removes instructions whose results are never used and that have no side effects.

```
Algorithm:
1. Initialize worklist with all instructions that have side effects.
2. Mark each as "live".
3. For each live instruction, mark its operands as live.
4. Any unmarked instruction with no side effects is dead → remove.
```

### 20.2 Dead Block Elimination (DBE)

Removes basic blocks that are not reachable from the entry block.

```
Algorithm:
1. Traverse CFG from entry block using DFS/BFS.
2. Mark all visited blocks as reachable.
3. Remove unreachable blocks.
4. Remove edges from reachable blocks to removed blocks.
```

### 20.3 Dead Region Elimination

Removes regions that are never entered:

```
An invoke_region instruction whose result is dead and whose region
has no side-effect instructions can be eliminated along with its region.
```

### 20.4 Aggressive DCE

Iterates DIE and DBE to a fixed point, also removing:
- Dead block arguments (no branch passes them)
- Dead region yields (no invoke_region consumes them)
- Dead branches in conditional branches (when the condition is constant)

---

## Chapter 21 -- Constant Folding & Propagation

### 21.1 Constant Folding

Evaluates instructions with all-constant operands at compile time:

```
# Before:
#   %1 = const 2
#   %2 = const 3
#   %3 = add %1, %2

# After:
#   %3 = const 5
```

Supports: arithmetic, comparison, logical, string concatenation, list/map construction, and pure collection operations.

### 21.2 Sparse Conditional Constant Propagation (SCCP)

A more powerful algorithm that propagates constants through the CFG, including through branches:

```
Algorithm (Wegman-Zadeck):
1. Initialize all values to "unknown" (top).
2. Mark entry block as executable.
3. Worklist of executable edges:
   a. For each instruction in an executable block, evaluate with known constant operands.
   b. If result is constant, propagate to uses.
   c. For conditional branches with constant conditions, mark only the taken edge as executable.
4. Values that remain "unknown" become "variable" (bottom).
5. Replace constant-valued instructions with const instructions.
6. Convert unreachable blocks to unreachable-terminated blocks.
```

### 21.3 AI-Aware Folding

Special folding rules for AI types:

- `unwrap(wrap_ok(X))` → `X`
- `is_ok(wrap_ok(X))` → `true`
- `is_ok(wrap_err(X))` → `false`
- `unwrap_prob(Probabilistic{value: X, confidence: 1.0}, t)` → `X` (for any t ≤ 1.0)
- `prompt_render(p, {})` where prompt has no template variables → the raw prompt text (if known)
- `memory_recall(m, k)` after `memory_remember(m, k, v)` → `v` (intra-procedural only)

---

## Chapter 22 -- Inlining

### 22.1 Function Inlining

Replaces a call instruction with the body of the called function.

```
Heuristics:
- Inline if callee_size <= 20 instructions OR callee_size <= 50 AND call_count == 1
- Never inline recursive functions
- Never inline functions with regions (parallel, attempt) in the default pipeline
  (these can be inlined by a region-aware inliner in the graph-level passes)
- Prefer inlining when caller passes constant arguments (enables further folding)
- Respect `@inline` and `@noinline` attributes
```

### 22.2 Inlining Algorithm

```
1. Clone the callee's blocks and instructions.
2. Map callee block arguments to the caller's argument values.
3. Replace `ret` in the callee with `br` to the after-call block, passing the return value.
4. Update def-use chains.
```

### 22.3 Region Inlining

Regions may also be inlined into their parent:

```
invoke_region @body, success=bb_succ, error=bb_err
```

Becomes: the body blocks are merged into the parent function's CFG, with the body's `yield` becoming a `br` to `bb_succ`, and the body's error exits connecting to `bb_err`.

---

## Chapter 23 -- Common Subexpression Elimination

### 23.1 Local CSE (within a block)

Within a single basic block, if two instructions have the same opcode and the same operands, and neither has side effects, the second can be replaced by the first's result.

```
# Before:
#   %1 = add %a, %b
#   ...
#   %2 = add %a, %b

# After:
#   %1 = add %a, %b
#   ...
#   (use %1 instead of %2)
```

### 23.2 Global CSE (GVN)

Global Value Numbering assigns a "value number" to each expression. Expressions with the same value number compute the same value and can be unified.

```
Algorithm:
1. Walk the dominator tree in pre-order.
2. For each instruction:
   a. Compute hash from (opcode, value_numbers_of_operands).
   b. If hash exists in the value table and the defining instruction dominates this use:
      → replace with the existing value.
   c. Otherwise, assign a new value number and add to the table.
3. Scoped hash table: entries are removed when leaving a scope (for block-dependent values).
```

### 23.3 AI-Specific CSE

LLM calls are *not* CSE'd by default (they are non-deterministic). However, CSE is applied to:

- `prompt_render(p, args)`: pure function, CSE when args are identical.
- `embed_generate(model, text)`: typically deterministic for the same model+text; CSE when model is the same.
- `type_of`, `symbol_lookup`: pure reflection, CSE always.
- `cost_query`: effectful; NOT CSE'd.

---

## Chapter 24 -- Loop Optimizations

### 24.1 Loop Invariant Code Motion (LICM)

Moves instructions that produce the same value on every iteration out of the loop.

```
Algorithm:
1. Identify loop-invariant instructions:
   - All operands are constants OR defined outside the loop OR are themselves loop-invariant.
   - The instruction has no side effects.
   - The instruction is in a block that dominates all loop exits.
2. Hoist these instructions to the loop pre-header.
```

### 24.2 Induction Variable Optimization

Identifies and simplifies induction variables:

```
# Before:
#   loop i = 0; i < 100; i = i + 1:
#       addr = base + i * 4
#       use(addr)

# After (strength reduction):
#   loop i = 0, addr = base; i < 100; i = i + 1, addr = addr + 4:
#       use(addr)
```

### 24.3 Loop Unrolling

Duplicates the loop body to reduce branch overhead and enable further optimization:

```
Unroll factors:
- Small loops (≤ 10 instructions): unroll factor 4
- Medium loops (≤ 50 instructions): unroll factor 2
- Large loops: no unrolling
- Loops with unknown trip count: no unrolling
- Respect @unroll(N) and @nounroll attributes
```

### 24.4 Loop Fusion

Merges adjacent loops with the same iteration space:

```
# Before:
#   for i in range:
#       a[i] = f(i)
#   for i in range:
#       b[i] = g(a[i])

# After:
#   for i in range:
#       a[i] = f(i)
#       b[i] = g(a[i])
```

Reduces loop overhead and improves cache locality.

---

## Chapter 25 -- AI-Specific Optimizations

### 25.1 LLM Call Fusion

Combines independent LLM calls that can be batched into a single multi-prompt request:

```
# Before:
#   %1 = llm_complete @Claude, [msg("summarize A")]
#   %2 = llm_complete @Claude, [msg("summarize B")]

# After (if backend supports batching):
#   [%1, %2] = llm_batch @Claude, [[msg("summarize A")], [msg("summarize B")]]
```

Fusion conditions:
- Same model and parameters (temperature, max_tokens).
- No data dependency between calls.
- Both calls are in the same block or in blocks that can be merged.
- Backend supports batching.

### 25.2 Response Caching

Inserts cache check/insert operations around idempotent LLM calls:

```
# Before:
#   %1 = llm_complete @Claude, messages, opts

# After (with caching):
#   %key = hash(messages, opts)
#   %cached = cache_lookup(%key)
#   cond_br %cached, bb_cached, bb_fetch
# bb_fetch:
#   %1 = llm_complete @Claude, messages, opts
#   cache_insert(%key, %1)
#   br bb_use
# bb_cached:
#   br bb_use(%cached)
# bb_use(%result):
#   use(%result)
```

Cache eligibility:
- `temperature == 0` (deterministic outputs)
- No tool-use (function calling invalidates caching)
- `max_tokens` is explicit
- Messages are known at compile time (or vary only by values that the cache key captures)
- Cache TTL configured by policy

### 25.3 Prompt Specialization

Generates specialized prompt variants for known arguments:

```
# Original: prompt with variable {language}
# If call site always passes language="Python":
#   → Generate specialized version without the variable, saving rendering cost.
```

### 25.4 Speculative Tool Execution

When a task conditionally calls a tool based on an LLM response, and the tool call is cheap (e.g., `filesystem.exists`), the tool call can be executed speculatively in parallel with the LLM call:

```
# Before:
#   %1 = llm_complete(...)      # may ask to check if file exists
#   %2 = tool_invoke @Filesystem, "exists", [%path]

# After:
#   parallel:
#       %1 = llm_complete(...)
#       %2_spec = tool_invoke @Filesystem, "exists", [%path]
#   # If LLM needed %2_spec, it's already available
```

### 25.5 Confidence Pruning

If a `Probabilistic<T>` value is used only through `is_certain(0.9)` or similar gates, and the confidence is provably below the threshold at compile time, the guarded code path can be eliminated:

```
# Before:
#   %1 = llm_complete(...)     # estimated confidence: 0.6
#   %2 = unwrap_prob(%1, 0.9)  # always null

# After DCE:
#   %1 = llm_complete(...)     # still needed for side effects? If not, dead.
#   %2 = null
```

---

## Chapter 26 -- Graph-Level Optimizations

These optimizations operate on the HIR execution DAG.

### 26.1 Task Fusion

Merges adjacent tasks that run in the same agent context with no data dependencies between them into a single compound task:

```
# Before:
#   [Task A: read file] → [Task B: analyze file]
#
# After:
#   [Task AB: read and analyze file]
```

Benefits: eliminates context-switching overhead, reduces scheduling latency.

### 26.2 Parallelism Discovery

Analyzes the DAG to identify opportunities for parallel execution:

```
Algorithm:
1. Build dependency graph.
2. Compute the transitive reduction (remove redundant edges).
3. Nodes at the same topological depth that share no transitive dependencies are parallel candidates.
4. Insert ParallelNode wrapper around candidate sets.
5. Estimate parallelism benefit vs. overhead (swarm cost, context duplication).
```

### 26.3 Critical Path Reduction

Identifies the critical path through the DAG and applies optimizations:

- **Promote to faster model**: Use GPT-4o-mini instead of GPT-4o for non-critical-path tasks.
- **Cache pre-computed results**: If a task on the critical path has been executed before with identical inputs, reuse the cached output.
- **Speculative execution**: Start downstream tasks with estimated inputs before upstream tasks complete.

### 26.4 Subgraph Deduplication

Identifies identical subgraphs (same node types, same edges, same parameters) and deduplicates:

```
Before: Two identical "Check Security" task chains in different workflows.
After:  Single "Check Security" subgraph, referenced from both workflows.
```

### 26.5 Cost-Aware Node Reordering

When the DAG allows topological flexibility (nodes at the same depth with no mutual dependencies), reorder to optimize cost:

- Schedule cheap nodes first (fast feedback).
- Defer expensive speculative nodes until their results are needed.
- Batch model calls to the same provider (reduces connection overhead).

---

# Part VI -- Serialization

---

## Chapter 27 -- JSON Serialization Format

### 27.1 Purpose

JSON serialization (`.lir` files) is the primary interchange format. It is:
- **Human-readable** for debugging and inspection.
- **Tool-friendly** for scripting, diffing, and version control.
- **Self-describing** with embedded schema references.

### 27.2 Top-Level Schema

```json
{
  "$schema": "https://lucky-lang.dev/ir/v0.1/schema.json",
  "version": "0.1",
  "meta": {
    "source_file": "main.lk",
    "compiled_at": "2026-07-02T10:30:00Z",
    "compiler_version": "0.1.0",
    "checksum": "sha256:abcdef...",
    "ir_level": "high"
  },
  "module": { ... },
  "graph": { ... }
}
```

### 27.3 Compact vs Pretty

Two JSON encodings are supported:

- **Pretty**: Indented, with debug names, source locations, and comments. Used for development and debugging. File extension: `.lir.json`.
- **Compact**: No whitespace, symbol references by numeric ID, no debug info. Used for production deployment. File extension: `.lir`.

### 27.4 Symbol References

In pretty mode, symbols are referenced by name:
```json
{ "model_ref": "Claude" }
```

In compact mode, symbols are referenced by index:
```json
{ "model_ref": 12 }
```

The symbol table in the module header maps indices to fully-qualified names.

### 27.5 Type References

Types use the compact descriptor format (see 4.2) or numeric indices:
```json
{ "type": "list<str>" }         # pretty
{ "type": 47 }                   # compact (index into type pool)
```

### 27.6 Value Encoding

Values are tagged unions:
```json
{ "kind": "bool", "value": true }
{ "kind": "int", "value": 42 }
{ "kind": "float", "value": 3.14 }
{ "kind": "string", "value": "hello" }
{ "kind": "null" }
{ "kind": "unknown" }
{ "kind": "list", "items": [ ... ] }
{ "kind": "map", "entries": { "key": { "kind": "int", "value": 1 } } }
{ "kind": "const_ref", "const_id": 5 }
{ "kind": "symbol_ref", "symbol": "Researcher" }
```

### 27.7 Complete Example (Pretty)

```json
{
  "version": "0.1",
  "meta": { "source_file": "main.lk", "ir_level": "high" },
  "module": {
    "symbols": {
      "Researcher": { "kind": "agent", "model": "Claude" },
      "BuildAndDeploy": { "kind": "workflow" }
    },
    "types": {
      "URI": { "kind": "primitive", "name": "uri" },
      "String": { "kind": "primitive", "name": "str" }
    }
  },
  "graph": {
    "nodes": [
      {
        "id": "n1",
        "kind": "workflow",
        "label": "BuildAndDeploy",
        "workflow_ref": "BuildAndDeploy",
        "body": {
          "nodes": ["n2", "n3", "n4"],
          "edges": [
            { "from": "n2", "to": "n3", "kind": "control" },
            { "from": "n3", "to": "n4", "kind": "control" }
          ]
        }
      }
    ]
  }
}
```

---

## Chapter 28 -- Binary Serialization Format (LKR)

### 28.1 Design

The binary format (`.lkr`) is optimized for:
- **Fast deserialization**: Memory-mappable, zero-copy where possible.
- **Compact size**: Variable-length integer encoding, string interning.
- **Random access**: Index tables allow seeking to specific sections without full parse.

### 28.2 File Layout

```
┌──────────────────────────────────────┐
│ Magic: 4 bytes  "LKR\0"              │ 0x00
│ Version: 2 bytes (major, minor)      │ 0x04
│ Flags: 2 bytes                       │ 0x06
│ Header offset: 4 bytes               │ 0x08
├──────────────────────────────────────┤
│ Section Index                        │
│  count: u16                          │
│  [section_tag: u8, offset: u32, len: u32] × count
├──────────────────────────────────────┤
│ Sections (aligned to 8 bytes)        │
│  [Section data] × count              │
├──────────────────────────────────────┤
│ String Table                         │
│  count: u32                          │
│  [len: u16, data: [u8]] × count      │
├──────────────────────────────────────┤
│ Constant Pool                        │
│  count: u32                          │
│  [tag: u8, data: variable] × count   │
├──────────────────────────────────────┤
│ Type Pool                            │
│  count: u32                          │
│  [type encoding] × count             │
└──────────────────────────────────────┘
```

### 28.3 Section Tags

| Tag | Section | Description |
|---|---|---|
| 0x01 | `META` | Module metadata (version, checksum, source info) |
| 0x02 | `SYMBOLS` | Symbol table |
| 0x03 | `TYPES` | Type pool (if not in global pool) |
| 0x04 | `AGENTS` | Agent definitions |
| 0x05 | `TASKS` | Task definitions |
| 0x06 | `WORKFLOWS` | Workflow definitions |
| 0x07 | `GRAPH_NODES` | HIR graph nodes |
| 0x08 | `GRAPH_EDGES` | HIR graph edges |
| 0x10 | `MIR_FUNCTIONS` | MIR function definitions |
| 0x11 | `MIR_BLOCKS` | MIR basic blocks |
| 0x12 | `MIR_INSTRUCTIONS` | MIR instruction stream |
| 0x13 | `MIR_REGIONS` | MIR region definitions |
| 0x20 | `LIR_FUNCTIONS` | LIR-L functions |
| 0x30 | `ATTRIBUTES` | Attribute sets |
| 0x31 | `DEBUG_INFO` | Debug location table |
| 0x32 | `POLICIES` | Policy definitions |
| 0x33 | `PERMISSIONS` | Permission sets |

### 28.4 Variable-Length Integer Encoding

Integers use a LEB128-like encoding:

```
Byte 0: [continuation: 1 bit] [data: 7 bits]
...
Byte N: [continuation: 0 bit] [data: 7 bits]

Unsigned: 7 bits per byte, little-endian.
Signed: zigzag encoding + unsigned LEB128.
```

### 28.5 Instruction Encoding

Each MIR instruction is encoded as:

```
[opcode: u16] [flags: u8] [num_operands: u8] [operand: u32] × num_operands [type_ref: u32] [attributes_len: u16] [attributes: variable]
```

Operands are encoded as:
- SSA value: `(block_id: u32, value_index: u16)` packed into u32
- Constant: `(0x8000_0000 | const_id: u31)`
- Symbol: `(0xC000_0000 | symbol_id: u30)`
- Type: `(0xE000_0000 | type_id: u28)`
- Immediate int: `(0xF000_0000 | value: u28)` -- signed 28-bit immediate

### 28.6 Memory Mapping

The LKR format is designed for `mmap`:

1. The string table and constant pool are at known offsets (from the header).
2. Pointers within the file are file-relative offsets.
3. All data structures use aligned layouts (8-byte alignment for bulk data).
4. A loader can mmap the file and access most structures without copying.

---

## Chapter 29 -- Textual IR Format (LIRT)

### 29.1 Purpose

The textual IR format (`.lirt`) is a human-readable, line-oriented representation for debugging, testing, and compiler development. It is *not* a stable interchange format -- it may change between compiler versions.

### 29.2 MIR Textual Syntax

```
module "my_project" {
  // Symbols
  agent @Researcher { model: @Claude }
  task @Investigate { input: (topic: str, depth: str), output: (report: doc) }

  // Types
  type %str = prim "str"
  type %uri = prim "uri"

  // Function
  func @Investigate.body(%topic: str, %depth: str) -> %doc {
  bb0:
    %0 = const 42
    %1 = add %0, %topic.length
    cond_br %depth eq "shallow", bb1, bb2

  bb1:
    %2 = call @quick_search(%topic)
    br bb3(%2)

  bb2:
    %3 = call @deep_search(%topic)
    br bb3(%3)

  bb3(%result: %doc):
    ret %result
  }
}
```

### 29.3 HIR Textual Syntax

```
graph {
  node n1: workflow "BuildAndDeploy" {
    context { repo: %uri }
  }

  node n2: task @Research {
    inputs { query: "how to build" }
  }

  node n3: agent_invoke @Coder.generate {
    arguments { spec: n2.report }
  }

  edge n1 -> n2: control
  edge n2 -> n3: data { port: "report" }
}
```

### 29.4 Round-Trip Guarantees

- `JSON → Text → JSON`: Not guaranteed identical (whitespace, ordering), but semantically equivalent.
- `Binary → Text → Binary`: Byte-for-byte identical (Text acts as a pretty-printer for binary).
- `Text → Binary → Text`: Semantically equivalent but formatting may differ.

---

## Chapter 30 -- Versioning & Compatibility

### 30.1 Version Scheme

IR version follows `major.minor`:
- **Major**: Incremented for breaking changes (old consumers cannot read new IR).
- **Minor**: Incremented for additive changes (old consumers can read new IR but may ignore new fields).

### 30.2 Compatibility Rules

| Rule | Description |
|---|---|
| **Forward compatibility** | A runtime v0.1 MUST reject IR v0.2. A runtime v0.1 MAY accept IR v0.1.5 (same major). |
| **Backward compatibility** | A runtime v0.2 SHOULD accept IR v0.1 (same major). |
| **Feature flags** | New IR features are guarded by feature flags in the meta section. A consumer that doesn't understand a feature flag MUST reject the IR. |
| **Deprecation** | Deprecated fields are marked with a `deprecated` attribute and remain for one major version before removal. |

### 30.3 Feature Flagging

```json
{
  "meta": {
    "features": ["streaming_tasks", "swarm_v2", "structured_outputs"]
  }
}
```

A consumer checks the features list against its supported set. Unsupported features cause rejection with a clear error message.

### 30.4 Migration

The compiler toolchain includes an IR migration tool:

```
lucky ir migrate --from 0.1 --to 0.2 input.lir -o output.lir
```

Migration handles:
- Renamed fields and types
- Restructured node kinds
- Changed encoding formats
- Deprecated features (removes them)
- New required fields (fills with defaults)

---

# Part VII -- Backend Interoperability

---

## Chapter 31 -- Backend Adapter Interface

### 31.1 Adapter Architecture

Backend adapters consume the IR and translate it into backend-specific API calls:

```
┌──────────────────────────────────────────┐
│              Backend Adapter              │
│                                           │
│  ┌─────────────────────────────────────┐ │
│  │         IR Consumer                  │ │
│  │  - Parse LIR-H / LIR-M / LIR-L      │ │
│  │  - Walk execution DAG               │ │
│  │  - Lower MIR instructions           │ │
│  └──────────────┬──────────────────────┘ │
│                 │                         │
│  ┌──────────────▼──────────────────────┐ │
│  │         Backend Translator           │ │
│  │  - Map IR ops to backend API calls  │ │
│  │  - Handle model-specific prompts    │ │
│  │  - Adapt tool calling conventions   │ │
│  └──────────────┬──────────────────────┘ │
│                 │                         │
│  ┌──────────────▼──────────────────────┐ │
│  │         Transport Layer             │ │
│  │  - HTTP/gRPC/stdio transport        │ │
│  │  - Authentication                   │ │
│  │  - Rate limiting & retry            │ │
│  └─────────────────────────────────────┘ │
└──────────────────────────────────────────┘
```

### 31.2 Adapter Trait (Abstract Interface)

```
trait BackendAdapter {
    /// Unique identifier for this backend
    fn name() -> &str;

    /// Which IR levels does this backend consume?
    fn supported_ir_levels() -> Vec<IRLevel>;

    /// Initialize with configuration
    fn initialize(config: BackendConfig) -> Result<Self>;

    /// Execute a HIR node
    fn execute_node(&self, node: &HIRNode, context: &Context) -> Result<Value>;

    /// Execute a MIR function (for local execution)
    fn execute_mir(&self, func: &MIRFunction, args: &[Value]) -> Result<Value>;

    /// Execute LIR-L linear instructions
    fn execute_lir(&self, lfunc: &LinearFunction, args: &[Value]) -> Result<Value>;

    /// Health check
    fn health_check(&self) -> Result<()>;

    /// Get capabilities
    fn capabilities(&self) -> BackendCapabilities;
}

struct BackendCapabilities {
    supported_models: Vec<String>,
    supported_tools: Vec<String>,
    supports_streaming: bool,
    supports_batching: bool,
    supports_function_calling: bool,
    supports_vision: bool,
    max_context_tokens: u64,
    max_output_tokens: u32,
}
```

### 31.3 IR Level Selection

Backends declare which IR level they prefer:

| IR Level | Best For | Example Backends |
|---|---|---|
| HIR | Orchestration, scheduling, multi-agent workflows | Claude Code, Codex CLI, OpenCode |
| MIR | Optimized single-task execution | Lucky standalone runtime |
| LIR-L | Deterministic computation, no LLM calls | Local executor, WASM backend |

The runtime selects the appropriate level based on backend capabilities and the task's requirements.

---

## Chapter 32 -- LLM Backend Mapping

### 32.1 Mapping LLM Call Instructions

The `llm_complete`, `llm_chat`, and `llm_stream` MIR instructions map to provider-specific API calls:

```
Claude Adapter:
    llm_complete(model=@Claude, messages, opts)
    → POST https://api.anthropic.com/v1/messages
      {
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 4096,
        "temperature": 0.7,
        "system": messages[0].content,   // if role == "system"
        "messages": messages[1..]         // user/assistant alternating
      }

GPT Adapter:
    llm_complete(model=@GPT, messages, opts)
    → POST https://api.openai.com/v1/chat/completions
      {
        "model": "gpt-4o",
        "messages": [{"role": m.role, "content": m.content} for m in messages],
        "max_tokens": 4096,
        "temperature": 0.7
      }
```

### 32.2 Tool-Use Mapping

When the IR contains tool definitions, the adapter includes them in the LLM request:

```
llm_complete(model=@Claude, messages=[
    {role: "user", content: "List files"}
], opts={
    tools: [@Filesystem]
})

→ Anthropic Messages API with "tools" field:
  {
    "tools": [{
      "name": "Filesystem.list",
      "description": "List files in a directory",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": {"type": "string", "description": "Directory path"}
        },
        "required": ["path"]
      }
    }]
  }
```

### 32.3 Streaming Mapping

```
llm_stream → SSE (Server-Sent Events) stream

Adapter wraps the SSE stream into Lucky's Stream<T> type,
emitting tokens as they arrive. Each token is a String value.
```

### 32.4 Model-Specific Prompt Assembly

Different models have different system prompt and message formatting conventions. The adapter handles:

- Claude: System prompt as top-level `system` field.
- GPT: System prompt as a message with `role: "system"`.
- Gemini: System prompt as `system_instruction` field.
- Local (Ollama): System prompt as `system` field in the request.

The IR is neutral; the adapter performs the mapping.

---

## Chapter 33 -- Tool Backend Mapping

### 33.1 Tool Call Flow

```
IR: tool_invoke(tool=@Git, method="commit", args={message: "fix", files: ["main.lk"]})

→ Backend Adapter:
   1. Look up tool adapter for @Git
   2. Check permissions: git.commit allowed?
   3. Validate arguments against method schema
   4. Invoke: GitAdapter.commit("fix", ["main.lk"])
   5. Return: Success("abc123") or Failure(error)
```

### 33.2 Built-in Tool Adapters

| Tool | Adapter | Implementation |
|---|---|---|
| `Filesystem` | `FilesystemAdapter` | Direct OS calls with path sandboxing |
| `Git` | `GitAdapter` | libgit2 or git CLI subprocess |
| `Browser` | `BrowserAdapter` | Playwright/Puppeteer subprocess |
| `Shell` | `ShellAdapter` | Subprocess with command allow-listing |
| `HTTP` | `HTTPAdapter` | HTTP client library |
| `Database` | `DatabaseAdapter` | SQL driver with connection pooling |
| `Search` | `SearchAdapter` | Web search API (Tavily, Brave, Serper) |
| `Memory` | `MemoryAdapter` | Internal memory manager |
| `Knowledge` | `KnowledgeAdapter` | Internal RAG pipeline |

### 33.3 Custom Tool Registration

Custom tools defined as Lucky tasks are registered with the adapter:

```
1. Compile custom tool task to MIR function.
2. Register function with the tool adapter under the tool's name.
3. When `tool_invoke` references the custom tool, the adapter calls the MIR function.
```

---

## Chapter 34 -- Local Executor Backend

### 34.1 Design

The local executor runs deterministic MIR/LIR-L functions directly in-process, without any LLM or external API calls. This is used for:

- Pure computation tasks (math, data transformation)
- Non-AI tool calls (filesystem reads when no LLM is needed)
- Test execution
- Debugging

### 34.2 Local Execution Model

```
LocalExecutor
├── execute_function(func: &MIRFunction, args: &[Value]) -> Result<Value>
├── registers: Vec<Value>        # virtual register file
├── stack: Vec<StackFrame>       # call stack
├── memory: Vec<u8>              # heap memory for alloca/store/load
└── pc: InstIndex                # program counter
```

### 34.3 Instruction Dispatch

```
fn execute_function(func, args):
    block = func.entry_block
    registers = args

    loop:
        for inst in block.instructions:
            match inst.opcode:
                Add => { dst = pop_op(); a = pop_op(); b = pop_op(); regs[dst] = a + b }
                Const => { dst = pop_op(); regs[dst] = load_constant(inst.operand) }
                Call => { push_frame(); jump to callee; }
                Ret => { pop_frame(); if stack empty: return regs[inst.operand]; }
                CondBr => {
                    if regs[inst.operands[0]]:
                        block = inst.true_dest; break
                    else:
                        block = inst.false_dest; break
                }
                Br => { block = inst.dest; break }
                LlmComplete => {
                    return Err("LLM calls not supported in local executor")
                }
                ...
```

### 34.4 Limitations

The local executor:
- Cannot execute LLM calls (raises an error).
- Cannot execute tool calls that require external processes (raises an error, unless a tool adapter is registered).
- Does not perform checkpointing (checkpoints are a scheduler-level concern).
- Does not enforce permissions (permissions are a scheduler-level concern).

---

## Chapter 35 -- Multi-Backend Orchestration

### 35.1 Heterogeneous Execution

A single Lucky program can execute across multiple backends:

```
Workflow "BuildAndDeploy":
  ├── Research          → Claude API (Anthropic)
  ├── Design            → GPT API (OpenAI)
  ├── Implement         → Claude Code (local CLI)
  ├── UnitTest          → Local Executor (no LLM)
  └── Deploy            → Shell tool (local subprocess)
```

### 35.2 Backend Selection

Backend selection follows policy resolution:

```
fn select_backend(node, context):
    1. Check node-level model override (agent uses @GPT)
    2. Check policy-based routing:
       - cost_limit → prefer cheapest capable backend
       - latency_sla → prefer fastest backend
       - privacy → prefer local backend
    3. Check backend capabilities (does it support the required operations?)
    4. Check backend availability (health check)
    5. Fall back to default backend
```

### 35.3 State Transfer Between Backends

When a value produced by one backend is consumed by another:

1. The producing backend returns a `Value`.
2. The runtime stores the value in the context.
3. The consuming backend reads from context.

All backends use the same `Value` representation (see IR value encoding), so no conversion is needed for primitive and collection types. AI-specific types (Probabilistic, Artifact) use a canonical in-memory representation shared by all backends.

### 35.4 Cross-Backend Checkpointing

Checkpoints are backend-agnostic. When execution spans multiple backends, a checkpoint captures:
- The HIR DAG state (which nodes completed, which are active).
- All context values (canonical representation).
- Per-backend opaque state (each backend serializes its internal state as a blob).

---

## Chapter 36 -- Custom Backend Development

### 36.1 Backend SDK

The Lucky toolchain provides a backend SDK for developing custom backends:

```
lucky-backend-sdk (Rust crate)
├── ir::loader          # Parse .lir / .lkr / .lirt files
├── ir::walker          # Walk IR nodes, blocks, instructions
├── ir::value           # Value type and conversions
├── adapter::traits     # BackendAdapter trait
├── adapter::testing    # Test harness for backends
└── codegen::helpers    # Code generation utilities
```

### 36.2 Minimal Backend Example

```rust
use lucky_backend_sdk::*;

struct MyBackend;

impl BackendAdapter for MyBackend {
    fn name() -> &str { "my_backend" }

    fn supported_ir_levels() -> Vec<IRLevel> {
        vec![IRLevel::High]
    }

    fn initialize(config: BackendConfig) -> Result<Self> {
        // Read API keys, set up connections
        Ok(MyBackend)
    }

    fn execute_node(&self, node: &HIRNode, ctx: &Context) -> Result<Value> {
        match node.kind {
            NodeKind::Task => {
                // Execute task node
                let task = node.as_task()?;
                // ... translate to backend-specific API calls ...
                Ok(Value::null())
            }
            NodeKind::LLMCall => {
                // Translate to model API call
                let llm = node.as_llm_call()?;
                let response = self.call_model(llm.model, llm.messages, llm.options)?;
                Ok(Value::string(response))
            }
            _ => Err(Error::unsupported_node(node.kind))
        }
    }

    fn health_check(&self) -> Result<()> {
        // Verify backend is reachable
        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supported_models: vec!["my-model".into()],
            supports_streaming: false,
            supports_batching: false,
            // ...
        }
    }
}
```

### 36.3 Backend Testing

The SDK includes a conformance test suite:

```
Test suite:
├── ir_parsing          # Can the backend parse all IR levels?
├── value_roundtrip     # Does the backend handle all Value types correctly?
├── node_execution      # Can the backend execute each node kind?
├── error_handling      # Does the backend properly propagate errors?
├── tool_calling        # Can the backend invoke standard tools?
├── streaming           # Does streaming work correctly?
└── concurrency         # Is the backend thread-safe?
```

### 36.4 Backend Registration

Backends register with the Lucky runtime via configuration:

```toml
[runtime.backends.my_backend]
adapter = "my_backend_adapter.so"   # dynamically loaded
# or
adapter = "my_lucky_backend"         # crate name (statically linked)
config = { api_key = "${MY_API_KEY}", endpoint = "https://..." }
```

---

# Part VIII -- Analysis & Verification

---

## Chapter 37 -- IR Verifier

### 37.1 Purpose

The IR verifier checks structural and semantic invariants of the IR. It runs:
- **During compilation**: After each optimization pass (debug mode).
- **At load time**: Before executing any IR.
- **On demand**: `lucky ir verify file.lir`

### 37.2 Verification Checks

#### Module-Level

- All symbol references resolve to valid definitions.
- All type references resolve to valid types.
- The constant pool has no dangling references.
- No duplicate symbol IDs or type IDs.
- Feature flags are consistent with the IR content.

#### Graph-Level (HIR)

- The graph is acyclic.
- Every node is reachable from an entry point.
- All edges have valid source and target node IDs.
- Data edges are type-compatible.
- Control edges form a valid partial order.
- Resource edges do not create contradictions.

#### Function-Level (MIR)

- The CFG is valid (entry block, terminators, no dead-ends except `unreachable`).
- Every use dominates its definition (SSA property).
- Block arguments are used only within their block.
- All instructions have valid operand types.
- `ret` instructions match the function's declared return type.
- Region-bearing instructions have the correct number of attached regions.
- `yield` instructions are only used within regions.

#### LIR-L Level

- Labels are uniquely defined.
- Jump targets exist.
- Register definitions dominate uses (within the linear instruction stream).
- Call arguments match callee signatures.
- Stack operations are balanced.

### 37.3 Verification Error Format

```
Error: [E001] Use-before-def in function @Investigate.body
  → Instruction %7 at bb2:4 uses value %3
  → Value %3 is defined at bb3:1
  → bb3 does not dominate bb2
  → Suggestion: Move the definition before the use, or add a block argument.
```

---

## Chapter 38 -- Type Checking in IR

### 38.1 IR Type Checker

The IR type checker validates that every instruction's operands are compatible with its opcode:

```
Instruction type rules (excerpt):
├── add(I64, I64)     → I64
├── add(F64, F64)     → F64
├── add(D128, D128)   → D128
├── eq(T, T)          → I1 (Bool)
├── cond_br(I1, _, _) → Token
├── load(Ref<T>)      → T
├── store(Ref<T>, T)  → Token
├── list_get(List<T>, I64) → T?
├── map_get(Map<K,V>, K)   → V?
├── llm_complete(Model, List<Message>, CompleteOpts) → Probabilistic<String>
├── tool_invoke(Tool, String, Map) → Result<Any>
└── ...
```

### 38.2 Type Inference in IR

The IR is fully typed -- types are explicit on every instruction. However, the verifier can *reconstruct* types for consistency checking:

```
fn check_instruction(inst, env):
    expected_op_types = opcode_type_schema[inst.opcode].param_types
    for (operand, expected) in zip(inst.operands, expected_op_types):
        actual = env.type_of(operand)
        if not is_subtype(actual, expected):
            return TypeError(inst, operand, expected, actual)
    return opcode_type_schema[inst.opcode].result_type
```

### 38.3 Polymorphic Instructions

Some instructions are polymorphic (work on multiple types):

```
add : (T, T) → T  where T ∈ {I64, F64, D128}
eq  : (T, T) → I1 where T is any equality-comparable type
```

The type checker resolves the concrete type from the operands.

---

## Chapter 39 -- Use-Def Analysis

### 39.1 Def-Use Chains

As described in Chapter 11, the IR maintains def-use chains for fast data-flow analysis.

### 39.2 Reaching Definitions

For optimization passes, reaching-definition analysis is available:

```
ReachingDefinitions
├── at_block_entry(block) -> Set<ValueHandle>
├── at_block_exit(block) -> Set<ValueHandle>
└── reaching_defs_at(block, inst_index) -> Set<ValueHandle>
```

Computed via standard data-flow equations (forward, may-analysis).

### 39.3 Liveness Analysis

```
Liveness
├── live_in(block) -> Set<ValueHandle>
├── live_out(block) -> Set<ValueHandle>
└── is_live_at(value, block, inst_index) -> Bool
```

Computed via backward data-flow analysis. Used by:
- Register allocation (spill decisions)
- Dead code elimination (removing unused definitions)
- Checkpoint optimization (which values to snapshot)

---

## Chapter 40 -- Alias Analysis

### 40.1 Memory Alias Analysis

Lucky's memory model is mostly alias-free (immutable values), but agent memory and heap allocations via `alloca` can alias:

```
AliasAnalysis
├── may_alias(a: Ref<T>, b: Ref<U>) -> Bool
├── must_alias(a: Ref<T>, b: Ref<U>) -> Bool
└── is_no_alias(a: Ref<T>, b: Ref<U>) -> Bool
```

Rules:
- Two `alloca` results are always distinct (no alias).
- `gep` results derived from the same `alloca` may alias.
- Agent memory field accesses through different agents never alias.
- Agent memory field accesses through the same agent may alias.

### 40.2 Purity Analysis

```
fn is_pure(inst):
    return not inst.flags.has_side_effect
        and not inst.flags.is_volatile
        and all operands are pure
        and (if inst accesses memory, it's read-only and all aliases are known)
```

Pure instructions can be:
- Eliminated if their result is unused.
- Hoisted out of loops.
- Common-subexpression eliminated.
- Reordered freely.

---

## Chapter 41 -- Cost Estimation

### 41.1 Node Cost Model

The IR framework estimates execution cost for every HIR node:

```
CostEstimate
├── tokens_prompt: u64          # estimated prompt tokens
├── tokens_completion: u64      # estimated completion tokens
├── latency_ms: u64             # estimated wall-clock time
├── cost_usd: f64               # estimated dollar cost
├── confidence: f64             # confidence in this estimate (0-1)
└── breakdown: HashMap<String, f64>  # cost by component
```

### 41.2 Estimation Heuristics

| Node Type | Estimation Method |
|---|---|
| LLM Call | tokens_prompt = sum(message lengths) / 4 (approx); cost = model.cost_per_1k * tokens / 1000 |
| Tool Call | Latency from tool adapter metadata; cost = 0 (local) or API pricing |
| Task | Sum of child node estimates |
| Parallel | max(child latencies); sum(child costs) |
| Decision | Weighted sum of branch costs by estimated branch probability |
| Loop | body_cost * estimated_iterations |
| Pipeline | sum(stage costs) |

### 41.3 Cost Budget Tracking

The cost model is used during scheduling (Chapter 5 of Runtime Spec) and for optimization decisions:

```
Optimizer uses cost estimates to:
- Decide whether to inline (cost of call vs cost of inlined body)
- Decide whether to unroll (loop overhead vs code size)
- Select model routing (GPT-4o-mini for cheap tasks, GPT-4o for complex tasks)
- Enable/disable caching (cache lookup cost vs LLM call cost)
```

---

## Chapter 42 -- Critical Path Analysis

### 42.1 Critical Path Computation

```
fn compute_critical_path(graph):
    # Forward pass: earliest start times
    for node in topological_order(graph):
        node.earliest_start = max(pred.earliest_finish for pred in node.predecessors)
        node.earliest_finish = node.earliest_start + node.estimated_duration

    # Backward pass: latest start times
    for node in reverse_topological_order(graph):
        node.latest_finish = min(succ.latest_start for succ in node.successors)
        node.latest_start = node.latest_finish - node.estimated_duration

    # Slack = latest_start - earliest_start
    for node in graph:
        node.slack = node.latest_start - node.earliest_start
        node.is_critical = (node.slack == 0)
```

### 42.2 Optimization Opportunities

Nodes on the critical path are prioritized for:
- Faster model selection
- Parallelization (if slack allows re-structuring)
- Caching and speculation

Nodes with high slack can be:
- Deferred to cheaper models
- Executed with lower priority
- Batched with other low-priority work

---

# Appendix A -- IR Instruction Set Reference

Complete catalog of MIR instructions with opcodes, operand types, result types, and flags.

```
Opcode             | Operands                                | Result            | Flags
───────────────────┼────────────────────────────────────────┼───────────────────┼──────────
add                | T, T                                    | T                 | C
sub                | T, T                                    | T                 | C
mul                | T, T                                    | T                 | C
div                | T, T                                    | T                 | C
rem                | I64, I64                                | I64               | C
neg                | T                                       | T                 | C
abs                | T                                       | T                 | C
and                | I1, I1                                  | I1                | C
or                 | I1, I1                                  | I1                | C
not                | I1                                      | I1                | C
xor                | I64, I64                                | I64               | C
shl                | I64, I64                                | I64               | C
shr                | I64, I64                                | I64               | C
eq                 | T, T                                    | I1                | C
neq                | T, T                                    | I1                | C
lt                 | T, T                                    | I1                | C
le                 | T, T                                    | I1                | C
gt                 | T, T                                    | I1                | C
ge                 | T, T                                    | I1                | C
───────────────────┼────────────────────────────────────────┼───────────────────┼──────────
alloca             | Type                                    | Ref<T>            | M
load               | Ref<T>                                  | T                 | MR
store              | Ref<T>, T                               | Token             | MW
gep                | Ref<T>, [I64]                           | Ref<U>            | C
───────────────────┼────────────────────────────────────────┼───────────────────┼──────────
list_new           | [T]                                     | List<T>           | C
list_get           | List<T>, I64                            | T?                | C
list_set           | List<T>, I64, T                         | List<T>           | C
list_len           | List<T>                                 | I64               | C
list_concat        | List<T>, List<T>                        | List<T>           | C
map_new            | [(K,V)]                                 | Map<K,V>          | C
map_get            | Map<K,V>, K                             | V?                | C
map_insert         | Map<K,V>, K, V                          | Map<K,V>          | C
set_new            | [T]                                     | Set<T>            | C
set_contains       | Set<T>, T                               | I1                | C
set_insert         | Set<T>, T                               | Set<T>            | C
───────────────────┼────────────────────────────────────────┼───────────────────┼──────────
str_concat         | String, String                          | String            | C
str_len            | String                                  | I64               | C
str_slice          | String, I64, I64                        | String            | C
str_find           | String, String                          | I64?              | C
str_replace        | String, String, String                  | String            | C
str_split          | String, String                          | List<String>      | C
───────────────────┼────────────────────────────────────────┼───────────────────┼──────────
br                 | BlockId, [Value]                        | Token             | T
cond_br            | I1, BlockId, BlockId, [Value], [Value]  | Token             | T
switch             | I64, BlockId, [(I64,BlockId)], [[Value]]| Token             | T
ret                | Value?                                  | Token             | T
invoke_region      | RegionId, BlockId, BlockId, [Value]     | Token             | T
yield              | Value                                   | Token             | T
unreachable        | --                                      | Never             | T
abort              | String                                  | Never             | TE
───────────────────┼────────────────────────────────────────┼───────────────────┼──────────
llm_complete       | SymId, [Message], CompleteOpts          | Probabilistic<S>  | EA
llm_chat           | SymId, [Message], CompleteOpts          | Message           | EA
llm_stream         | SymId, [Message], CompleteOpts          | Stream<String>    | EA
prompt_render      | SymId, Map<String,Any>                  | String            | C
tool_invoke        | SymId, String, Map<String,Any>          | Result<Any>       | EA
agent_invoke       | SymId, String, Map<String,Any>          | Result<Any>       | EA
memory_remember    | SymId, String, Any, Embedding?          | Token             | EAW
memory_recall      | SymId, String                           | Any?              | EAR
memory_similar     | SymId, Embedding, I64                   | List<...>         | EAR
knowledge_search   | SymId, String, I64                      | List<Chunk>       | EAR
knowledge_ask      | SymId, String, SymId, I64               | Answer            | EAR
approval_request   | String, String, Duration                | Approval          | EH
approval_wait      | Approval                                | I1                | EH
embed_generate     | SymId, String                           | Embedding         | EA
───────────────────┼────────────────────────────────────────┼───────────────────┼──────────
pipe_next          | Stream<T>                               | T?                | EA
pipe_map           | Stream<T>, FuncId                       | Stream<U>         | C
pipe_filter        | Stream<T>, FuncId                       | Stream<T>         | C
pipe_take          | Stream<T>, I64                          | Stream<T>         | C
pipe_collect       | Stream<T>                               | List<T>           | EA
pipe_for_each      | Stream<T>, FuncId                       | Token             | EA
chan_send          | Channel<T>, T                           | Token             | EAW
chan_recv          | Channel<T>                              | T?                | EAR
───────────────────┼────────────────────────────────────────┼───────────────────┼──────────
cast               | T, Type                                 | U                 | C
int_to_float       | I64                                     | F64               | C
float_to_int       | F64                                     | I64               | C
to_string          | T                                       | String            | C
parse_int          | String                                  | I64?              | C
parse_float        | String                                  | F64?              | C
unwrap             | Result<T,E>                             | T                 | C*
wrap_ok            | T                                       | Result<T,E>       | C
wrap_err           | E                                       | Result<T,E>       | C
is_ok              | Result<T,E>                             | I1                | C
unwrap_prob        | Probabilistic<T>, F64                   | T?                | C
───────────────────┼────────────────────────────────────────┼───────────────────┼──────────
type_of            | Any                                     | Type              | C
symbol_lookup      | String                                  | SymId?            | C
checkpoint         | String                                  | Token             | ES
cost_query         | --                                      | CostReport        | ES
log                | I64, String, Map                        | Token             | ES
sleep              | Duration                                | Token             | ES

Flags legend:
  C  = Constant-foldable (pure, no side effects)
  M  = Memory operation
  MR = Memory read
  MW = Memory write
  T  = Terminator
  E  = Effectful (has side effects)
  A  = AI operation (involves LLM or AI service)
  R  = Read effect (observable but idempotent)
  W  = Write effect (mutates state)
  H  = Human interaction
  S  = System/runtime operation
  *  = May trap/panic
```

---

# Appendix B -- Optimization Pass Catalog

```
Pass Name                 | Level  | Description                          | Enabled
──────────────────────────┼────────┼──────────────────────────────────────┼────────
canonicalize              | MIR    | Normalize IR to canonical form       | Always
die                       | MIR    | Dead instruction elimination           | Always
dbe                       | MIR    | Dead block elimination                 | Always
dre                       | MIR    | Dead region elimination                | Always
const_fold                | MIR    | Constant folding                       | Always
sccp                      | MIR    | Sparse conditional constant propagation| O1+
cse_local                 | MIR    | Local common subexpression elimination | O1+
gvn                       | MIR    | Global value numbering                 | O2+
licm                      | MIR    | Loop invariant code motion             | O2+
indvar_simplify           | MIR    | Induction variable optimization        | O2+
loop_unroll               | MIR    | Loop unrolling                         | O3
loop_fuse                 | MIR    | Loop fusion                            | O3
inline                    | MIR    | Function inlining                      | O1+
inline_region             | MIR    | Region inlining                        | O2+
strength_reduce           | MIR    | Strength reduction                      | O2+
mem2reg                   | MIR    | Promote memory to SSA registers        | Always
copy_prop                 | MIR    | Copy propagation                       | Always
branch_fold               | MIR    | Fold constant branches                  | Always
jump_thread               | MIR    | Jump threading                         | O2+
tail_call                 | MIR    | Tail call optimization                  | O2+
──────────────────────────┼────────┼──────────────────────────────────────┼────────
llm_fuse                  | MIR    | Fuse adjacent LLM calls into batches   | O1+
prompt_cache              | MIR    | Insert response caching                 | O1+
prompt_specialize         | MIR    | Specialize prompts for const args      | O2+
speculative_tool          | MIR    | Speculative tool execution              | O3
confidence_prune          | MIR    | Prune unreachable confidence paths     | O2+
──────────────────────────┼────────┼──────────────────────────────────────┼────────
task_fuse                 | HIR    | Merge adjacent tasks                   | O1+
discover_parallelism       | HIR    | Discover parallel execution opportunities| Always
critical_path_opt         | HIR    | Optimize critical path                  | O2+
subgraph_dedup            | HIR    | Deduplicate identical subgraphs        | O2+
cost_reorder              | HIR    | Cost-aware node reordering              | O2+
model_route_opt           | HIR    | Optimize model routing                  | O1+
cache_reuse               | HIR    | Reuse cached task outputs               | O1+
```

Optimization levels:
- **O0**: No optimizations (debug mode). Only canonicalization.
- **O1**: Basic optimizations (DCE, CSE, inlining, const folding). Fast compile.
- **O2**: Standard optimizations (GVN, LICM, AI-specific passes). Balanced.
- **O3**: Aggressive optimizations (loop unrolling, speculation). Maximum performance.

---

# Appendix C -- LIR-H Node Type Reference

```
Node Kind              | Has Body? | Has Regions? | Key Fields
───────────────────────┼───────────┼──────────────┼───────────────────────────
goal                   | yes       | yes          | goal_ref, success_criteria
workflow               | yes       | yes          | workflow_ref, context
task                   | yes       | no           | task_ref, agent_ref, inputs
agent_invoke           | no        | no           | agent_ref, method, args
tool                   | no        | no           | tool_ref, method, args
llm_call               | no        | no           | model_ref, messages, opts
decision               | yes       | yes (2)      | condition, true/false bodies
match                  | yes       | yes (N+1)    | scrutinee, arms, default
parallel               | no        | yes (N)      | branches, strategy, join
join                   | no        | no           | source_parallel, mode
loop                   | no        | yes (1)      | max_iterations, induction
for_each               | no        | yes (1)      | iterator, induction
pipeline               | no        | no           | stages
attempt                | yes       | yes (1+N)    | body, recovery actions
approval               | no        | no           | gate, message, timeout
let                    | no        | no           | name, value
return                 | no        | no           | value
noop                   | no        | no           | --
```

---

# Appendix D -- Serialization Schemas (JSON Schema)

### D.1 HIR Graph Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://lucky-lang.dev/ir/v0.1/hir-graph.schema.json",
  "type": "object",
  "required": ["version", "meta", "graph"],
  "properties": {
    "version": { "const": "0.1" },
    "meta": {
      "type": "object",
      "required": ["source_file", "ir_level"],
      "properties": {
        "source_file": { "type": "string" },
        "compiled_at": { "type": "string", "format": "date-time" },
        "compiler_version": { "type": "string" },
        "checksum": { "type": "string" },
        "ir_level": { "enum": ["high", "mid", "low"] },
        "features": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    },
    "graph": {
      "type": "object",
      "required": ["nodes", "edges"],
      "properties": {
        "nodes": {
          "type": "array",
          "items": { "$ref": "#/$defs/Node" }
        },
        "edges": {
          "type": "array",
          "items": { "$ref": "#/$defs/Edge" }
        }
      }
    }
  },
  "$defs": {
    "Node": {
      "type": "object",
      "required": ["id", "kind"],
      "properties": {
        "id": { "type": "string", "format": "uuid" },
        "kind": { "$ref": "#/$defs/NodeKind" },
        "label": { "type": "string" },
        "debug_loc": { "$ref": "#/$defs/DebugLoc" },
        "policy": {},
        "permissions": { "type": "string" },
        "resource": { "$ref": "#/$defs/ResourceReq" },
        "cost": { "$ref": "#/$defs/CostEstimate" },
        "attributes": { "type": "object" }
      },
      "allOf": [
        { "if": { "properties": { "kind": { "const": "goal" } } },
          "then": { "required": ["goal_ref"] } }
      ]
    },
    "NodeKind": {
      "enum": [
        "goal", "workflow", "task", "agent_invoke", "tool",
        "llm_call", "decision", "match", "parallel", "loop",
        "for_each", "pipeline", "attempt", "approval",
        "let", "return", "noop"
      ]
    },
    "Edge": {
      "type": "object",
      "required": ["from", "to", "kind"],
      "properties": {
        "from": { "type": "string", "format": "uuid" },
        "to": { "type": "string", "format": "uuid" },
        "kind": { "$ref": "#/$defs/EdgeKind" },
        "port": { "type": "string" },
        "condition": { "type": "string" }
      }
    },
    "EdgeKind": {
      "enum": ["control", "data", "resource", "condition", "context", "approval", "error", "cost"]
    },
    "DebugLoc": {
      "type": "object",
      "properties": {
        "file": { "type": "string" },
        "line": { "type": "integer" },
        "column": { "type": "integer" }
      }
    },
    "ResourceReq": {
      "type": "object",
      "properties": {
        "cpu_millicores": { "type": "integer", "default": 100 },
        "memory_mb": { "type": "integer", "default": 256 },
        "timeout_ms": { "type": "integer", "default": 300000 },
        "exclusive": { "type": "array", "items": { "type": "string" } }
      }
    },
    "CostEstimate": {
      "type": "object",
      "properties": {
        "estimated_usd": { "type": "number" },
        "estimated_tokens": { "type": "integer" },
        "estimated_duration_ms": { "type": "integer" }
      }
    }
  }
}
```

---

# Appendix E -- Binary Format Specification

### E.1 Complete Byte Layout

```
Offset  Size   Field
──────────────────────────────────────
0x00    4      Magic: 'L' 'K' 'R' '\0'
0x04    2      Version major (u16 LE)
0x06    2      Version minor (u16 LE)
0x08    2      Flags (u16 LE)
              bit 0: compressed (zstd)
              bit 1: has MIR sections
              bit 2: has LIR sections
              bit 3: has debug info
              bits 4-15: reserved
0x0A    2      Reserved (zero)
0x0C    4      Section index offset (u32 LE)
0x10    --     Padding to 16-byte alignment
──────────────────────────────────────
        Section Index
        ├── count: u32
        └── entries: [SectionIndexEntry; count]
            SectionIndexEntry:
            ├── tag: u8
            ├── reserved: u8
            ├── reserved: u16
            ├── offset: u32
            └── length: u32
──────────────────────────────────────
        Sections (each 8-byte aligned)
──────────────────────────────────────
        String Table (last section)
        ├── count: u32
        └── entries: [StringEntry; count]
            StringEntry:
            ├── length: u16
            └── data: [u8; length]
──────────────────────────────────────
        End of file
```

### E.2 Value Encoding in Binary

```
Value (tagged union):
    tag: u8
        0x00 = null
        0x01 = bool (next byte: 0 or 1)
        0x02 = int (LEB128 signed)
        0x03 = float (8 bytes, IEEE 754 LE)
        0x04 = decimal (16 bytes, LE)
        0x05 = string (u32 string_table_index)
        0x06 = bytes (u32 length + [u8; length])
        0x07 = list (u32 length + [Value; length])
        0x08 = set (u32 length + [Value; length])
        0x09 = map (u32 length + [(Value, Value); length])
        0x0A = time (i64 nanoseconds since epoch)
        0x0B = duration (i64 nanoseconds)
        0x0C = uuid (16 bytes)
        0x0D = uri (u32 string_table_index)
        0x0E = version (5 × u16: major, minor, patch, pre_len, build_len + strings)
        0x0F = path (u32 string_table_index)
        0x10 = const_ref (u32 const_pool_index)
        0x11 = symbol_ref (u32 symbol_table_index)
        0x12 = type_ref (u32 type_pool_index)
        0x13 = node_ref (16 bytes uuid)
        0x14 = error (u32 code + u32 msg_string_index + bool recoverable)
        0x15 = unknown
        0x16 = embedding (u32 dimensions + u32 model_string_index + [f32; dimensions])
        0x17 = probabilistic (Value inner + f64 confidence)
        0x18 = result_ok (Value inner)
        0x19 = result_err (Value inner)
        0x1A-0xFF: reserved
```

---

# Appendix F -- Textual IR Grammar

### F.1 MIR Text Grammar (EBNF)

```ebnf
module      = "module" string_lit "{" { top_level } "}" ;

top_level   = agent_def | task_def | func_def | type_def | symbol_def ;

agent_def   = "agent" "@" identifier "{" agent_body "}" ;
agent_body  = { "model" ":" "@" identifier
              | "memory" ":" "@" identifier
              | "tools" ":" "[" [ "@" identifier { "," "@" identifier } ] "]" } ;

func_def    = "func" "@" identifier [ "." identifier ]
              "(" [ typed_param { "," typed_param } ] ")"
              "->" type_ref "{" { basic_block } "}" ;

typed_param = identifier ":" type_ref ;

basic_block = block_id [ "(" [ typed_param { "," typed_param } ] ")" ] ":"
              { instruction }
              terminator ;

block_id    = "bb" digit { digit } ;

instruction = [ "%" identifier "=" ] opcode operands
              [ "{" { attribute } "}" ] ";" ;

opcode      = identifier ;

operands    = { operand } ;
operand     = "%" identifier           -- SSA value
            | const_lit                -- constant
            | "@" identifier           -- symbol
            | block_id                 -- block reference
            | type_ref                 -- type reference
            | string_lit               -- string constant
            ;

terminator  = "br" block_id "(" [ operand { "," operand } ] ")" ";"
            | "cond_br" operand "," block_id "(" operands ")" ","
                         block_id "(" operands ")" ";"
            | "ret" [ operand ] ";"
            | "unreachable" ";"
            | "abort" string_lit ";"
            ;

type_ref    = "i1" | "i64" | "f64" | "d128" | "str" | "bytes"
            | "list" "<" type_ref ">"
            | "map" "<" type_ref "," type_ref ">"
            | "fn" "(" [ type_ref { "," type_ref } ] ")" "->" type_ref
            | "%" identifier            -- named type
            ;

attribute   = identifier ":" ( string_lit | int_lit | float_lit | "true" | "false" ) ;

const_lit   = int_lit | float_lit | string_lit | "true" | "false"
            | "null" | "unknown" | "inf" | "nan" ;
```

### F.2 Example MIR Text

```
module "example_project" {
  type %str = prim "str"
  type %uri = prim "uri"
  type %doc = prim "doc"

  agent @Researcher {
    model: @Claude
    tools: [@Browser, @Search]
  }

  func @Investigate.body(%topic: %str, %depth: %str) -> %doc {
  bb0:
    %0 = const 42
    %1 = str_len %topic
    %2 = add %0, %1
    %3 = eq %depth, "shallow"
    cond_br %3, bb1, bb2

  bb1:
    %4 = call @quick_search(%topic)
    br bb3(%4)

  bb2:
    %5 = call @deep_search(%topic)
    br bb3(%5)

  bb3(%result: %doc):
    %6 = call @validate(%result)
    ret %6
  }
}
```

---

*End of Lucky IR Specification, Version 0.1*
