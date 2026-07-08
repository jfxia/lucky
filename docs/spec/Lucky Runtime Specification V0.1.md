# Lucky Runtime Specification
<img src="../../logo/logo128.png" alt="Lucky logo" width="64" align="right" />


**Version:** 0.1 Draft  
**Status:** Technical Specification  
**Target:** Runtime implementors and backend adapter authors  

---

# Table of Contents

```
1.  Introduction
2.  Architecture Overview
3.  Lucky IR Specification
4.  Execution Engine
5.  Scheduler
6.  Memory Model
7.  Concurrency Model
8.  Checkpoint System
9.  Permission & Security System
10. Tool Execution & Sandboxing
11. Backend Adapter Interface
12. Recovery & Fault Tolerance
13. Cost & Resource Management
14. Observability & Telemetry
15. Distributed Execution
16. Runtime Configuration
```

---

## 1. Introduction

### 1.1 Purpose

This document specifies the Lucky Runtime &mdash; the execution engine that consumes Lucky IR (`.lir`) files and orchestrates the execution of goals, workflows, agents, and tasks across LLM backends and tool executors.

The runtime is designed with these invariants:

* **Deterministic orchestration atop probabilistic AI.** The DAG structure, scheduling decisions, and state transitions are deterministic and reproducible. Only leaf-level LLM invocations are non-deterministic.
* **Backend agnosticism.** The same IR executes on Claude Code, Codex CLI, OpenCode, or a standalone runtime without source changes.
* **Observability by default.** Every state transition, tool invocation, and LLM call is logged for audit, replay, and debugging.
* **Graceful degradation.** Failures are contained, retried, rolled back, or escalated according to explicit policy rather than crashing the process.

### 1.2 Document Conventions

* **MUST / SHOULD / MAY** are used as defined in RFC 2119.
* Byte sizes use IEC prefixes (KiB, MiB).
* Time durations are in milliseconds unless stated otherwise.
* JSON is the wire format for all runtime protocol messages.

### 1.3 Relationship to Other Specifications

```
Lucky Language Reference Manual    →  Syntax, types, semantics
    └── Lucky Runtime Specification  →  Execution, scheduling, memory, security
```

---

## 2. Architecture Overview

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Lucky Runtime                          │
│                                                             │
│  ┌──────────┐   ┌──────────┐   ┌───────────────────────┐  │
│  │  IR      │   │ DAG      │   │     Scheduler         │  │
│  │  Loader  │──→│ Builder  │──→│                        │  │
│  └──────────┘   └──────────┘   │  ┌───────┐ ┌────────┐ │  │
│                                │  │ Ready  │ │Priority│ │  │
│  ┌──────────┐                  │  │ Queue  │ │Resolver│ │  │
│  │ Context  │                  │  └───────┘ └────────┘ │  │
│  │ Manager  │                  └───────────┬───────────┘  │
│  └──────────┘                              │              │
│                                            ▼              │
│  ┌──────────┐   ┌──────────┐   ┌───────────────────────┐  │
│  │ Memory   │   │Checkpoint│   │   Execution Engine    │  │
│  │ Manager  │   │ Manager  │   │                        │  │
│  └──────────┘   └──────────┘   │  ┌─────────────────┐  │  │
│                                │  │ Backend Router  │  │  │
│  ┌──────────┐   ┌──────────┐   │  ├─────────────────┤  │  │
│  │Permission│   │  Cost    │   │  │ Local Executor  │  │  │
│  │ Enforcer │   │ Tracker  │   │  │ Adapter         │  │  │
│  └──────────┘   └──────────┘   │  └─────────────────┘  │  │
│                                └───────────┬───────────┘  │
│                                            │              │
│  ┌──────────┐   ┌──────────┐   ┌───────────▼───────────┐  │
│  │ Event    │   │Telemetry │   │   Backend Adapters    │  │
│  │ Bus      │   │ Exporter │   │                        │  │
│  └──────────┘   └──────────┘   │ Claude│Codex│OpenCode  │  │
│                                └───────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Component Responsibilities

| Component | Responsibility |
|---|---|
| **IR Loader** | Parse `.lir` JSON, validate schema, deserialize |
| **DAG Builder** | Construct in-memory execution graph, resolve references |
| **Scheduler** | Maintain ready queue, assign nodes to executors |
| **Execution Engine** | Dispatch nodes to backends, manage lifecycle |
| **Context Manager** | Propagate context through the DAG, handle scoping |
| **Memory Manager** | Provide vector/structured memory for agents |
| **Checkpoint Manager** | Snapshot and restore runtime state |
| **Permission Enforcer** | Gate operations by capability-security rules |
| **Cost Tracker** | Track token usage and cost, enforce budgets |
| **Event Bus** | Publish/subscribe for reactive workflows |
| **Telemetry Exporter** | Export metrics, traces, and logs |

### 2.3 Lifecycle

```
[Startup]
    │
    ▼
[Load IR] ──→ [Validate Schema] ──→ [Resolve Dependencies]
    │
    ▼
[Build DAG] ──→ [Validate DAG] ──→ [Initialize Context]
    │
    ▼
[Schedule Loop]
    │
    ├── [Pick Ready Nodes]
    ├── [Dispatch to Executors]
    ├── [Wait for Completion]
    ├── [Checkpoint (if policy dictates)]
    ├── [Handle Failures (if any)]
    │
    ▼
[Verify Success Criteria]
    │
    ├── [Pass] ──→ [Finalize] ──→ [Shutdown]
    └── [Fail] ──→ [Recovery Path]
```

### 2.4 Thread Model

The runtime uses a bounded thread pool:

* **Main thread**: Runs the scheduler loop (single-threaded, deterministic).
* **Worker pool**: `N` worker threads for local task execution (configurable, default = CPU cores).
* **I/O pool**: `M` threads for async I/O (LLM calls, HTTP, file operations), default = 4 &times; CPU cores.
* **Timer thread**: Single thread for timeout enforcement.

All shared state (DAG progress, agent memory, context) is protected by fine-grained locks or lock-free data structures. The scheduler loop itself is single-threaded to guarantee deterministic replay.

---

## 3. Lucky IR Specification

### 3.1 Schema Version

The version field is `"0.1"` for this specification. All implementations MUST reject IR with an unsupported version.

### 3.2 Top-Level Structure

```json
{
  "version": "0.1",
  "meta": {
    "source_file": "string",
    "compiled_at": "ISO8601",
    "compiler_version": "string",
    "checksum": "SHA256-hex"
  },
  "project": {
    "name": "string",
    "version": "string"
  },
  "modules": [
    {
      "name": "string",
      "path": "string",
      "exports": ["Identifier"]
    }
  ],
  "agents": [ AgentDef ],
  "tasks": [ TaskDef ],
  "workflows": [ WorkflowDef ],
  "goals": [ GoalDef ],
  "tools": [ ToolDef ],
  "models": [ ModelDef ],
  "prompts": [ PromptDef ],
  "memories": [ MemoryDef ],
  "knowledge": [ KnowledgeDef ],
  "policies": [ PolicyDef ],
  "permissions": [ PermissionSet ],
  "context": {
    "entries": { "name": TypeRef }
  },
  "graph": {
    "nodes": [ Node ],
    "edges": [ Edge ]
  },
  "symbols": {
    "name": { "kind": "string", "type": TypeRef, "location": "string" }
  }
}
```

### 3.3 Node Types

Every node in the execution graph has a common envelope:

```json
{
  "id": "string (UUID)",
  "kind": "NodeKind",
  "label": "string (human-readable)",
  "policy": "PolicyRef | inline PolicyDef | null",
  "resource_requirements": {
    "cpu": "int (millicores, default 100)",
    "memory_mb": "int (default 256)",
    "timeout_ms": "int (default 300000)",
    "exclusive_resources": ["string"]
  },
  "estimated_cost_usd": "float | null",
  "estimated_duration_ms": "int | null",
  "metadata": { "key": "value" }
}
```

#### 3.3.1 GoalNode

```json
{
  "kind": "goal",
  "goal_ref": "string (symbol reference)",
  "success_criteria": [
    {
      "predicate": "string (expression source)",
      "description": "string"
    }
  ],
  "subgraph": [ "NodeId" ]
}
```

A GoalNode is the root of the execution DAG. On completion, the runtime evaluates each success criterion against the final context state.

#### 3.3.2 TaskNode

```json
{
  "kind": "task",
  "task_ref": "string",
  "agent_ref": "string | null",
  "inputs": { "param_name": Value },
  "expected_outputs": { "name": TypeRef },
  "steps": [ StepDef ]
}
```

A TaskNode executes a task &mdash; either a built-in operation or a user-defined task. If `agent_ref` is set, the task runs within that agent's context (memory, tools, permissions).

#### 3.3.3 StepDef

```json
{
  "kind": "StepKind",
  "...": "step-specific fields"
}
```

StepKind values:

| Kind | Description |
|---|---|
| `let` | Immutable binding: `{ "name": "string", "value": Value }` |
| `call` | Operation/task call: `{ "target": "string", "args": [Value] }` |
| `ask` | LLM prompt: `{ "model": "string", "prompt_ref": "string", "template_args": {} }` |
| `tool` | Tool invocation: `{ "tool": "string", "method": "string", "args": [Value] }` |
| `if` | Conditional: `{ "condition": Value, "then": [StepDef], "else": [StepDef] }` |
| `match` | Pattern match: `{ "scrutinee": Value, "arms": [{ "pattern": Pattern, "body": [StepDef] }] }` |
| `for` | Loop: `{ "variable": "string", "iterable": Value, "body": [StepDef] }` |
| `parallel` | Parallel: `{ "branches": [[StepDef]] }` |
| `await` | Await async: `{ "expression": Value }` |
| `return` | Return: `{ "value": Value | null }` |

#### 3.3.4 AgentInvokeNode

```json
{
  "kind": "agent_invoke",
  "agent_ref": "string",
  "method": "string",
  "arguments": { "name": Value },
  "model_override": "string | null"
}
```

#### 3.3.5 ParallelNode / JoinNode

```json
{
  "kind": "parallel",
  "branches": [ [NodeId, ...] ],
  "strategy": "all | any | race"
}
```

```json
{
  "kind": "join",
  "source_parallel": "NodeId",
  "mode": "wait_all | wait_any"
}
```

#### 3.3.6 DecisionNode

```json
{
  "kind": "decision",
  "condition": "string (expression source)",
  "true_branch": "NodeId",
  "false_branch": "NodeId | null"
}
```

#### 3.3.7 ApprovalNode

```json
{
  "kind": "approval",
  "gate_description": "string",
  "timeout_ms": "int | null",
  "escalation": {
    "on_timeout": "abort | escalate",
    "escalate_to": "string | null"
  }
}
```

#### 3.3.8 AttemptNode (Error Handling)

```json
{
  "kind": "attempt",
  "body": "NodeId",
  "recovery": [
    {
      "action": "retry | fallback | human | abort | skip",
      "max_retries": "int | null",
      "backoff": "linear | exponential | null",
      "backoff_max_ms": "int | null",
      "fallback_node": "NodeId | null",
      "human_message": "string | null"
    }
  ]
}
```

#### 3.3.9 PipelineNode

```json
{
  "kind": "pipeline",
  "stages": [
    {
      "operation": "string",
      "arguments": [Value]
    }
  ]
}
```

#### 3.3.10 Value Representation

Values are serialized as tagged unions:

```json
{ "kind": "bool", "value": true }
{ "kind": "int", "value": 42 }
{ "kind": "float", "value": 3.14 }
{ "kind": "string", "value": "hello" }
{ "kind": "null" }
{ "kind": "unknown" }
{ "kind": "list", "items": [Value] }
{ "kind": "set", "items": [Value] }
{ "kind": "map", "entries": { "key": Value } }
{ "kind": "node_ref", "node_id": "string" }
{ "kind": "symbol_ref", "symbol": "string" }
{ "kind": "error", "code": "int", "message": "string", "recoverable": true }
```

#### 3.3.11 Edges

```json
{
  "from": "NodeId",
  "to": "NodeId",
  "kind": "data | control | resource",
  "port": "string (output port name, for data edges)",
  "condition": "string | null (for conditional control edges)"
}
```

Edge kinds:
* **data**: The destination node reads a value produced by the source node.
* **control**: The destination node must not start until the source node completes.
* **resource**: The destination node requires an exclusive resource held by the source node.

### 3.4 DAG Validation Rules

The runtime MUST validate these invariants before execution:

1. **Acyclicity**: The graph must contain no cycles (loops are represented as bounded LoopNodes, not graph cycles).
2. **Single root**: Exactly one GoalNode or root WorkflowNode exists.
3. **Reachability**: Every node must be reachable from the root.
4. **Type consistency**: Data edges must connect compatible types (structural subtype check).
5. **No dangling references**: All `node_id`, `agent_ref`, `task_ref` references must resolve.
6. **Unique node IDs**: No duplicate node IDs.

### 3.5 IR Optimizations

The runtime MAY apply these optimizations before execution:

| Optimization | Description | Safety |
|---|---|---|
| Constant folding | Evaluate compile-time-constant expressions | Always safe |
| Dead node elimination | Remove nodes whose outputs are never consumed | Safe if no side effects |
| Node fusion | Merge adjacent pipeline stages into a single node | Must not change observable behavior |
| Speculative pre-warming | Begin loading agent state for likely-upcoming nodes | Must not affect correctness |
| Subgraph deduplication | Collapse identical subgraphs | Must preserve separate checkpoints |

---

## 4. Execution Engine

### 4.1 Engine State Machine

```
                 ┌─────────┐
                 │ Created  │
                 └────┬─────┘
                      │ validate DAG
                      ▼
                 ┌─────────┐
                 │  Ready   │
                 └────┬─────┘
                      │ scheduler dispatches
                      ▼
                 ┌──────────┐
            ┌───→│ Running   │
            │    └─────┬─────┘
            │          │
            │    ┌─────┴──────┐
            │    │            │
            │    ▼            ▼
            │ ┌───────┐  ┌──────────┐
            │ │Paused  │  │Completed │
            │ └───┬───┘  └────┬─────┘
            │     │            │
            │     ▼            ▼
            │ ┌──────────┐  ┌──────────┐
            │ │Resumed    │  │Finalized │
            │ └──────────┘  └──────────┘
            │
            │    ┌──────────┐
            └────│  Failed   │────→ (recovery chain)
                 └──────────┘
                      │
                      ▼
                 ┌───────────┐
                 │ Cancelled  │
                 └───────────┘
```

### 4.2 Node State Machine

Every node tracks its state independently:

```
    ┌─────────┐
    │ Pending  │ (waiting on upstream dependencies)
    └────┬─────┘
         │ all inputs satisfied
         ▼
    ┌─────────┐
    │  Ready   │ (eligible for scheduling)
    └────┬─────┘
         │ scheduler allocates slot
         ▼
    ┌─────────┐     ┌─────────────┐
    │ Running  │────→│ Checkpointed │ (state snapshot taken)
    └────┬─────┘     └──────┬──────┘
         │                  │ resume
         │    ┌─────────┐   │
         ├───→│ Waiting  │───┘ (waiting on sub-task or approval)
         │    └─────────┘
         │
         ▼
    ┌──────────┐
    │ Completed │ (produces output artifacts)
    └──────────┘
         │
         ▼ (on error)
    ┌──────────┐
    │  Failed   │
    └────┬─────┘
         │ recovery policy
         ▼
    ┌───────────┐     ┌───────────┐
    │ Recovering │────→│ Completed │
    └───────────┘     └───────────┘
         │
         ▼ (exhausted recovery)
    ┌───────────┐
    │ Escalated  │ (awaiting human)
    └───────────┘
```

### 4.3 Node Execution Protocol

When the scheduler dispatches a node to the execution engine, the engine:

1. **Acquires context**: Builds the effective context by merging inherited context with node-local overrides.
2. **Checks permissions**: Verifies the node's operations are allowed by the effective permission set.
3. **Allocates resources**: Reserves memory, file handles, and exclusive resources as declared.
4. **Executes the node body** according to `kind`:
   - TaskNode: executes steps sequentially within the node.
   - AgentNode: dispatches to the LLM backend via the adapter.
   - DecisionNode: evaluates the condition and activates the appropriate branch.
   - ParallelNode: fans out branches to separate execution slots.
   - ApprovalNode: suspends and notifies the human approver.
5. **Captures output**: Collects output values and attaches them as artifacts.
6. **Updates DAG state**: Marks the node as completed and signals downstream nodes.
7. **Checkpoints** (if policy dictates): Snapshots the current state.

### 4.4 Execution Context Construction

The effective context for a node is computed as:

```
effective_context = project_context
                  + workflow_context
                  + agent_context (if agent_ref is set)
                  + task_context (if task_ref is set)
                  + node_context (inline context in node definition)
```

Shadowing rules: more specific scopes shadow less specific ones. A task-level `context` entry with the same name as a workflow-level entry replaces it for the duration of that task.

### 4.5 Output Propagation

When a node completes:

1. Its declared outputs are written into the execution context under `node_id.output_name`.
2. Downstream nodes that reference these outputs via data edges receive the values.
3. If a downstream node is of type `DecisionNode`, the condition is re-evaluated with the updated context.
4. Artifacts (documents, patches, etc.) are stored in the artifact store and referenced by URI.

---

## 5. Scheduler

### 5.1 Scheduling Algorithm

The scheduler implements a priority-based topological traversal of the execution DAG.

```
function schedule_loop(graph, runtime_config):
    ready_set = PriorityQueue()

    # Initialize: enqueue all nodes with indegree zero
    for node in graph.nodes:
        if graph.in_degree(node) == 0:
            ready_set.push(node, compute_priority(node))

    active_nodes = Set()
    completed_nodes = Set()

    while not ready_set.empty() or not active_nodes.empty():
        # Dispatch as many ready nodes as slots allow
        while not ready_set.empty() and active_nodes.size() < runtime_config.max_concurrency:
            node = ready_set.pop_highest()
            if not exceeds_resource_budget(node, runtime_config):
                slot = acquire_slot(node)
                dispatch(node, slot)
                active_nodes.add(node)

        # Wait for at least one node to complete
        completed = await_next_completion(active_nodes, timeout=runtime_config.poll_interval_ms)
        active_nodes.remove(completed)
        completed_nodes.add(completed)

        # Process completion
        if completed.status == Failed:
            recovery_node = apply_recovery_policy(completed)
            if recovery_node is not None:
                graph.add_node(recovery_node)
                ready_set.push(recovery_node, PRIORITY_CRITICAL)
                continue

        # Signal successors
        for successor in graph.successors(completed):
            successor.pending_inputs.remove(completed)
            if successor.pending_inputs.empty():
                ready_set.push(successor, compute_priority(successor))

        # Checkpoint if policy dictates
        if runtime_config.checkpoint_after_each_node:
            checkpoint_manager.snapshot()

    # Verify success criteria
    root_goal = graph.root()
    for criterion in root_goal.success_criteria:
        if not evaluate(criterion.predicate, context):
            return Failure(criterion.description)

    return Success
```

### 5.2 Priority Calculation

Priority is a 32-bit integer computed as:

```
PRIORITY_CRITICAL = 0x0000_0000  # recovery, human escalation
PRIORITY_HIGH     = 0x4000_0000  # nodes on critical path
PRIORITY_NORMAL   = 0x8000_0000  # default
PRIORITY_LOW      = 0xC000_0000  # speculative / optional nodes
PRIORITY_BATCH    = 0xF000_0000  # batch / background work

def compute_priority(node):
    base = PRIORITY_NORMAL

    # User-specified priority overrides base
    if node.policy.priority is not None:
        return node.policy.priority

    # Critical path depth: longest path from this node to any leaf
    depth = graph.critical_path_depth(node)
    if depth == graph.max_critical_path_depth():
        base = PRIORITY_HIGH

    # Cost-weighted: cheaper nodes get priority (favors fast feedback)
    if node.estimated_cost_usd is not None and node.estimated_cost_usd < 0.01:
        base -= 0x1000_0000

    # Time-weighted: longer nodes get priority (avoids tail latency)
    if node.estimated_duration_ms is not None and node.estimated_duration_ms > 60000:
        base -= 0x0800_0000

    return base
```

### 5.3 Resource-Aware Scheduling

Before dispatching a node, the scheduler checks resource constraints:

| Constraint | Check |
|---|---|
| **Concurrency cap** | active_nodes.size() < max_concurrency |
| **Per-model rate limit** | model_requests_last_minute[model] < model_rate_limit |
| **Cost budget** | total_cost_so_far + node.estimated_cost <= cost_budget |
| **Memory budget** | allocated_memory + node_memory_requirement <= memory_limit |
| **Exclusive resources** | No active node holds a resource this node requires exclusively |

### 5.4 Backpressure

If no ready nodes can be dispatched due to resource constraints, the scheduler enters a waiting state. It polls every `poll_interval_ms` (default: 100ms) for resource availability.

When a new node becomes ready but cannot be immediately dispatched, it is placed in a **pending-ready** queue. Items in this queue are re-evaluated each scheduling cycle.

### 5.5 Scheduling Hooks

The scheduler exposes extension points:

```
on_node_ready(node)       # called when a node enters Ready state
on_node_dispatched(node)  # called when a node is dispatched to a worker
on_node_completed(node)   # called when a node completes (success or failure)
on_node_checkpointed(node, checkpoint_id)
on_cost_threshold(threshold_name)  # e.g., 50%, 80%, 100% of budget
```

These hooks are used by the telemetry exporter and can be consumed by external monitoring systems.

### 5.6 Fairness

When multiple ready nodes have identical priority, the scheduler uses FIFO ordering (the node that became ready first is dispatched first). This prevents starvation of nodes that happen to share a priority band with a continuous stream of new ready nodes.

---

## 6. Memory Model

### 6.1 Memory Architecture

```
┌───────────────────────────────────────────────────────────┐
│                     Runtime Process                       │
│                                                           │
│  ┌─────────────────────────────────────────────────────┐ │
│  │                    Value Heap                        │ │
│  │  (immutable, ref-counted, shared across tasks)       │ │
│  │                                                     │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │ │
│  │  │  String  │  │   List   │  │   Map    │  ...     │ │
│  │  │  "hello" │  │  [1,2,3] │  │ {a:1,b:2}│          │ │
│  │  └──────────┘  └──────────┘  └──────────┘          │ │
│  └─────────────────────────────────────────────────────┘ │
│                                                           │
│  ┌───────────┐  ┌───────────┐  ┌─────────────────────┐  │
│  │  Agent 1  │  │  Agent 2  │  │   Workflow Context  │  │
│  │  Memory   │  │  Memory   │  │                      │  │
│  │ ┌───────┐ │  │ ┌───────┐ │  │  {user, repo, ...}  │  │
│  │ │Vector │ │  │ │Struct │ │  └─────────────────────┘  │
│  │ │ Store │ │  │ │ Store │ │                           │
│  │ └───────┘ │  │ └───────┘ │  ┌─────────────────────┐  │
│  └───────────┘  └───────────┘  │   Task 1  Local     │  │
│                                 │   {step, state}     │  │
│  ┌───────────────────────────┐  └─────────────────────┘  │
│  │    Knowledge Base (RAG)   │                           │
│  │  ┌─────────────────────┐  │  ┌─────────────────────┐  │
│  │  │ Embedding Index     │  │  │   Checkpoint Store  │  │
│  │  │ (in-memory / disk)  │  │  │   (JSON snapshots)  │  │
│  │  └─────────────────────┘  │  └─────────────────────┘  │
│  └───────────────────────────┘                           │
└───────────────────────────────────────────────────────────┘
```

### 6.2 Value Heap

#### Allocation Strategy

The value heap is a thread-safe, arena-based allocator:

* **Small objects** (&le; 256 bytes): allocated from per-thread bump-allocated arenas. Extremely fast allocation and no per-object free overhead. Arenas are freed in bulk when the owning scope (task or workflow) completes.
* **Medium objects** (257 bytes &ndash; 64 KiB): allocated from a central page allocator with per-size-class free lists.
* **Large objects** (&gt; 64 KiB): allocated directly from the OS via `mmap` / `VirtualAlloc`.

#### Reference Counting

Every heap-allocated value carries a 32-bit reference count:

```
struct HeapValue {
    uint32_t refcount;        // atomic reference count
    uint8_t  type_tag;        // discriminator for the value type
    uint8_t  flags;           // pinned, immortal, etc.
    uint16_t padding;
    // ... type-specific data follows
};
```

Reference counting operations:

* **Increment**: `atomic_fetch_add(&refcount, 1, memory_order_relaxed)`
* **Decrement**: `if atomic_fetch_sub(&refcount, 1, memory_order_release) == 1 { atomic_thread_fence(memory_order_acquire); free(this); }`

Thread-safe for shared values. Single-threaded tasks may elide atomic operations if the value is not shared.

#### Immortal Values

Values that live for the entire runtime lifetime (boolean constants, small integers, empty collections) are marked `IMMORTAL` and never freed. These are pre-allocated at startup.

#### Memory Pressure Handling

When the heap exceeds a configurable threshold (default: 80% of process memory limit), the runtime:

1. Triggers a checkpoint (to preserve progress).
2. Evicts the least-recently-used cached LLM responses.
3. Compacts per-task arenas (frees completed task arenas).
4. If pressure persists, suspends low-priority nodes and notifies via telemetry.

### 6.3 Agent Memory

Agent memory is a persistent key-value store with optional vector indexing.

#### Memory Schema

```
AgentMemory {
    agent_id: UUID,
    scope: local | session | project | organization | global,
    entries: BTreeMap<String, MemoryEntry>,
    vector_index: Option<VectorIndex>,
}

MemoryEntry {
    key: String,
    value: Value,
    created_at: Timestamp,
    updated_at: Timestamp,
    ttl: Option<Duration>,
    embedding: Option<Vec<Float32>>,
    tags: Set<String>,
}
```

#### Memory Operations

| Operation | Signature | Description |
|---|---|---|
| `remember` | (key, value, embedding?) → void | Store or update an entry |
| `recall` | (key) → value? | Retrieve by exact key |
| `similar` | (embedding, limit) → [(key, value, score)] | K-nearest neighbor search |
| `search` | (query, limit) → [(key, value, score)] | Full-text or hybrid search |
| `forget` | (key) → void | Remove an entry |
| `list` | (prefix?, tags?) → [key] | List matching entries |
| `clear` | () → void | Remove all entries (scope-dependent) |

#### Vector Index

The vector index supports approximate nearest neighbor (ANN) search. The default backend is an in-memory HNSW index with the following parameters:

* **Dimensions**: Per the memory declaration (e.g., 1536 for OpenAI embeddings, 768 for others).
* **M** (max connections per layer): 16
* **ef_construction**: 200
* **ef_search**: 100 (configurable at query time)
* **Metric**: Cosine similarity (default), Euclidean, or dot product (configurable).

Large-scale deployments MAY use an external vector database (Pinecone, Weaviate, Qdrant) via a backend adapter.

#### Memory Consistency

Agent memory operations within a single task are **sequentially consistent** (operations appear to execute in program order). Across concurrent tasks of the same agent, memory operations are **linearizable** (each operation appears to execute instantaneously at some point between its invocation and completion).

The implementation uses a per-agent read-write lock with operation-level locking:
* `remember`, `forget`, `clear`: acquire write lock.
* `recall`, `similar`, `search`, `list`: acquire read lock.

#### Memory TTL and Eviction

Entries with a TTL set are automatically removed after expiration. A background thread runs eviction every 60 seconds:

```
for entry in memory.entries:
    if entry.ttl is not None and now() - entry.updated_at > entry.ttl:
        memory.forget(entry.key)
```

### 6.4 Context Store

Context is an immutable, layered key-value map. Each scope (project, workflow, agent, task) contributes a layer.

#### Context Layering

```
struct ContextLayer {
    parent: Option<&ContextLayer>,
    entries: HashMap<String, Value>,
    scope: ScopeId,
}

fn context_get(layer: &ContextLayer, key: &str) -> Option<Value> {
    if let Some(value) = layer.entries.get(key) {
        return Some(value.clone());
    }
    if let Some(parent) = &layer.parent {
        return context_get(parent, key);
    }
    None
}
```

#### Immutability

Context layers are immutable once constructed. A node that needs to "modify" context produces a new layer that shadows entries in parent layers. This preserves the ability to checkpoint and replay deterministically.

### 6.5 Knowledge Base

#### Document Store

The knowledge base indexes documents for RAG queries:

```
KnowledgeBase {
    name: String,
    sources: Vec<Source>,
    chunker: ChunkerConfig,
    index: EmbeddingIndex,
}

Source {
    kind: file | directory | url,
    path: String,
    glob: Option<String>,
    recursive: bool,
}

ChunkerConfig {
    chunk_size: usize,       // characters (default: 1024)
    chunk_overlap: usize,    // characters (default: 128)
    separators: Vec<String>, // priority-ordered split points
}
```

#### Indexing Pipeline

1. **Crawl**: Walk sources and collect documents.
2. **Chunk**: Split documents into overlapping chunks using recursive character splitting.
3. **Embed**: Generate embeddings via the configured embedding model.
4. **Index**: Insert into the HNSW vector index.
5. **Update**: On file changes (for file sources), re-index affected chunks.

#### Query Pipeline

```
fn knowledge_query(kb, query, top_k):
    query_embedding = embed(query)
    candidates = kb.index.search(query_embedding, top_k * 2)  # oversample for reranking
    reranked = rerank(candidates, query)                       # cross-encoder rerank (optional)
    return reranked[:top_k]
```

### 6.6 Garbage Collection

The runtime uses **scope-based collection** rather than tracing GC:

1. **Task-local values**: Freed when the task completes (arena reset).
2. **Workflow-context values**: Freed when the workflow completes.
3. **Agent memory entries**: Explicitly freed via `forget()` or TTL eviction.
4. **Cached LLM responses**: Evicted by LRU policy when the cache exceeds size limit.
5. **Checkpoints**: Retained according to retention policy (default: keep last 10, delete older).

This deterministic approach eliminates GC pauses and simplifies checkpointing (no need to trace live references).

---

## 7. Concurrency Model

### 7.1 Task Parallelism

The fundamental concurrency unit is the **task instance** (a running node in the execution DAG). The runtime exploits DAG parallelism: any nodes whose incoming edges are all satisfied are eligible for concurrent execution.

```
// Parallelism detection (compile-time, embedded in IR)
//
//    A ──→ B ──→ D ──→ E
//          ↘    ↗
//            C
//
// Ready sets at each step:
//   t=0: {A}
//   t=1: {B, C}   ← parallel
//   t=2: {D}      ← serial (waits for both B and C)
//   t=3: {E}
```

### 7.2 Execution Slots

The runtime maintains a pool of execution slots. Each slot can run one task instance.

```
struct ExecutionSlot {
    id: u32,
    state: Idle | Busy(NodeId),
    thread: JoinHandle<()>,
    metrics: SlotMetrics,
    backend: BackendHandle,
}

struct WorkerPool {
    slots: Vec<ExecutionSlot>,
    local_executor: LocalExecutor,
    io_executor: IoExecutor,
}
```

#### Slot Allocation Policy

When the scheduler selects a ready node for dispatch:

1. **Prefer idle slots** that previously executed nodes of the same agent (memory locality).
2. **Prefer local executor** for deterministic tasks (steps within a TaskNode).
3. **Prefer I/O executor** for LLM calls and tool invocations.
4. If no slot is available, the node remains in the ready queue.

#### Slot Lifecycle

```
Idle → (dispatch) → Busy → (complete | fail) → Idle
```

### 7.3 Parallel Node Execution

When the executor encounters a ParallelNode:

```
fn execute_parallel(parallel_node, context):
    branches = parallel_node.branches
    results = Vec::with_capacity(branches.len())

    # Dispatch all branches concurrently
    handles = Vec::new()
    for branch in branches:
        subgraph = build_subgraph(branch)
        handle = spawn(subgraph, context.clone())
        handles.push(handle)

    # Collect results
    for handle in handles:
        result = await handle
        results.push(result)

    return aggregate_results(results)
```

#### Strategies

| Strategy | Behavior |
|---|---|
| `all` (default) | Wait for all branches; fail if any branch fails. |
| `any` | Complete when any branch succeeds; cancel others. |
| `race` | Complete when the first branch finishes (success or failure). |

### 7.4 Synchronization Primitives

#### Barrier (Wait)

```lucky
parallel
    branch_a
    branch_b
wait
```

Implemented as a `JoinNode` in the IR. The join node accumulates completion signals from all branches before activating its successor.

#### Channels

```
struct Channel<T> {
    buffer: RingBuffer<T>,
    capacity: usize,
    senders: AtomicUsize,
    receivers: AtomicUsize,
    closed: AtomicBool,
    send_mutex: Mutex<()>,
    recv_condvar: Condvar,
}
```

* **Bounded channels**: Block the sender when the buffer is full (backpressure).
* **Unbounded channels**: Grow the buffer dynamically (may cause memory pressure).
* **Closing**: A closed channel drains remaining items, then returns `None` to receivers.

Channel operations:
* `send(value)` &mdash; push value; block if full (bounded)
* `receive()` &mdash; pop value; block if empty and not closed
* `try_send(value)` &mdash; non-blocking send; return false if full
* `try_receive()` &mdash; non-blocking receive; return null if empty
* `close()` &mdash; mark channel as closed

#### Atomic Operations (Agent Memory Only)

Agent memory fields support atomic operations for safe concurrent updates:

```
fn compare_and_swap(memory, field, expected, new):
    lock(memory.mutex)
    current = memory.fields[field]
    if current == expected:
        memory.fields[field] = new
        return (true, current)
    return (false, current)

fn atomic_update(memory, field, update_fn):
    lock(memory.mutex)
    memory.fields[field] = update_fn(memory.fields[field])
```

#### Mutex (Agent-Scoped)

```
struct AgentMutex {
    owner: Option<UUID>,  // task_id of current holder
    wait_queue: VecDeque<UUID>,
}

fn lock(mutex, task_id):
    if mutex.owner is None:
        mutex.owner = Some(task_id)
        return Ok
    else:
        mutex.wait_queue.push_back(task_id)
        block_until(mutex.owner == task_id)

fn unlock(mutex, task_id):
    assert(mutex.owner == Some(task_id))
    mutex.owner = mutex.wait_queue.pop_front()
    if mutex.owner is not None:
        wake(mutex.owner)
```

### 7.5 Deadlock Detection

The runtime constructs a **wait-for graph** at runtime:

* Nodes: tasks and agents.
* Edges: T1 &rarr; T2 if T1 is waiting for T2 (via await, channel receive, mutex lock, or DAG dependency).

A background thread periodically (every 5 seconds) runs cycle detection on the wait-for graph. If a cycle is found:

1. Collect the deadlocked task IDs.
2. Emit a `DEADLOCK_DETECTED` event with the cycle.
3. Cancel the lowest-priority task in the cycle.
4. If the deadlock persists after 3 detection cycles, escalate to human.

### 7.6 Swarm Execution

Swarms instantiate multiple identical agent-task pairs:

```lucky
swarm 20 Reviewer.review_patch(patches)
```

Implementation:

```
fn execute_swarm(agent, task, input_list, count):
    batch_size = min(count, runtime_config.swarm_max_concurrency)

    for batch in input_list.chunks(batch_size):
        handles = []
        for input in batch:
            instance = agent.instantiate()
            handle = spawn(instance.task(task, input))
            handles.push(handle)

        results = await_all(handles)
        yield results

    # Optional: merge results
    merged = merge_results(all_results, task.merge_strategy)
    return merged
```

Swarm configuration:
* `swarm_max_concurrency`: Maximum simultaneous swarm instances (default: 100).
* `swarm_batch_size`: Instances per scheduling batch (default: 10).
* `swarm_merge`: Merge strategy for results (`concat`, `majority_vote`, `best`).

---

## 8. Checkpoint System

### 8.1 Design Principles

1. **Incremental**: Only state changed since the previous checkpoint is written.
2. **Non-blocking**: Checkpoints are written asynchronously; task execution continues.
3. **Immutable**: Once written, a checkpoint is never modified.
4. **Portable**: Checkpoints are backend-agnostic (JSON + binary blobs).
5. **GC-friendly**: Checkpoints have explicit retention policies.

### 8.2 Checkpoint Content

```
struct Checkpoint {
    id: UUID,
    parent_id: Option<UUID>,
    timestamp: Timestamp,
    version: String,
    run_id: UUID,

    # DAG progress
    graph_state: {
        completed_nodes: Vec<NodeId>,
        active_nodes: Vec<{
            node_id: NodeId,
            status: Running | Waiting | Checkpointed,
            step_index: u32,                # position within task steps
            step_progress: serde_json::Value # step-specific state
        }>,
        failed_nodes: Vec<{
            node_id: NodeId,
            error: ErrorValue,
            recovery_attempts: u8,
        }>,
    },

    # Agent memories
    agent_snapshots: HashMap<AgentId, {
        entries: Vec<MemoryEntry>,
        vector_index_snapshot: Option<Vec<u8>>,  # serialized HNSW
    }>,

    # Context
    context_layers: Vec<{
        scope: ScopeId,
        entries: HashMap<String, Value>,
    }>,

    # Cost tracking
    cost: {
        total_cost_usd: f64,
        cost_by_model: HashMap<String, f64>,
        tokens_used: u64,
        tokens_by_model: HashMap<String, u64>,
    },

    # Pending approvals
    pending_approvals: Vec<{
        approval_id: UUID,
        node_id: NodeId,
        description: String,
        created_at: Timestamp,
        timeout_at: Option<Timestamp>,
    }>,

    # Artifact references
    artifacts: Vec<{
        artifact_id: UUID,
        node_id: NodeId,
        kind: String,
        uri: String,
        size_bytes: u64,
        checksum: String,
    }>,
}
```

### 8.3 Checkpoint Triggers

| Trigger | Description |
|---|---|
| **After each task** | Checkpoint after every TaskNode completes (policy: `checkpoint after each task`) |
| **After each workflow** | Checkpoint after each workflow subgraph completes (policy: `checkpoint after each workflow`) |
| **Time interval** | Checkpoint every N minutes (policy: `checkpoint interval 5m`) |
| **Before risky operation** | Checkpoint before a `transaction` block |
| **Before retry** | Checkpoint before each retry attempt (policy: `checkpoint before retry`) |
| **On memory pressure** | Checkpoint when memory pressure is detected |
| **On signal** | Checkpoint on SIGUSR1 (Unix) or explicit API call |
| **Manual** | User-requested checkpoint via CLI or API |

### 8.4 Checkpoint Storage Backend

The checkpoint store is an abstract interface with pluggable backends:

```
trait CheckpointStore {
    fn save(checkpoint: Checkpoint) -> Result<UUID>;
    fn load(id: UUID) -> Result<Checkpoint>;
    fn list(run_id: UUID) -> Result<Vec<CheckpointSummary>>;
    fn delete(id: UUID) -> Result<()>;
    fn delete_range(before: Timestamp) -> Result<usize>;
}

struct FilesystemCheckpointStore {
    root: PathBuf,
    format: Json | Cbor | MessagePack,
}
impl CheckpointStore for FilesystemCheckpointStore { ... }

struct S3CheckpointStore {
    bucket: String,
    prefix: String,
    client: S3Client,
}
impl CheckpointStore for S3CheckpointStore { ... }

struct DatabaseCheckpointStore {
    pool: ConnectionPool,
    table: String,
}
impl CheckpointStore for DatabaseCheckpointStore { ... }
```

#### Incremental Checkpoints

The filesystem and S3 backends support incremental checkpoints:

1. Compute a diff against the parent checkpoint.
2. Write only changed sections as separate files.
3. The checkpoint ID points to a manifest that references the parent and the diff files.

```
checkpoints/
    {run_id}/
        manifest.json          # checkpoint index
        ckp_001/               # full checkpoint
            checkpoint.json
            agent_memory_1.json
            artifacts/
        ckp_002/               # incremental (diff from ckp_001)
            patch.json         # only changed fields
            agent_memory_1_patch.json
            new_artifacts/
```

### 8.5 Recovery from Checkpoint

```
fn recover(checkpoint_id):
    checkpoint = checkpoint_store.load(checkpoint_id)

    # 1. Restore DAG state
    graph = load_graph_from_ir(checkpoint.run_id)
    graph.mark_completed(checkpoint.graph_state.completed_nodes)
    graph.mark_failed(checkpoint.graph_state.failed_nodes)
    for active in checkpoint.graph_state.active_nodes:
        graph.mark_active(active.node_id, active.status)
        # Resume from step_index + 1

    # 2. Restore agent memories
    for (agent_id, snapshot) in checkpoint.agent_snapshots:
        memory_manager.restore(agent_id, snapshot)

    # 3. Restore context layers
    context_manager.restore(checkpoint.context_layers)

    # 4. Restore cost tracking
    cost_tracker.restore(checkpoint.cost)

    # 5. Restore pending approvals
    approval_manager.restore(checkpoint.pending_approvals)

    # 6. Resume scheduling
    scheduler.resume(graph)
```

### 8.6 Checkpoint Consistency Guarantees

* **Atomic writes**: A checkpoint is written to a temporary location and atomically renamed/moved into place. Partial checkpoints are never visible.
* **Crash safety**: If the runtime crashes during checkpoint writing, the incomplete checkpoint is discarded. The previous checkpoint remains valid.
* **Fencing**: Only one runtime instance may own a run ID at a time. A fencing token in the checkpoint store prevents split-brain scenarios.

### 8.7 Retention Policy

```
struct RetentionPolicy {
    max_checkpoints: Option<usize>,        # keep at most N checkpoints
    max_age: Option<Duration>,             # delete checkpoints older than this
    keep_failed: bool,                     # retain checkpoints of failed runs
    keep_last_n_per_run: usize,            # keep last N per run (default: 5)
}
```

Default: keep the last 10 checkpoints, delete older ones.

---

## 9. Permission & Security System

### 9.1 Capability-Security Model

The Lucky runtime enforces **capability security** (object-capability model adaptated for AI agents). Every operation is gated by a permission check. Agents possess only the permissions explicitly granted to them.

### 9.2 Permission Hierarchy

```
Permission
├── filesystem
│   ├── read
│   │   ├── read("/home/user/*")
│   │   └── read("/etc/config")
│   ├── write
│   │   ├── write("./output/*")
│   │   └── write("/tmp/*")
│   └── delete
│       └── delete("/tmp/*")
├── git
│   ├── clone
│   ├── commit
│   ├── push
│   │   ├── push("*")           # any branch
│   │   └── push("feature/*")   # feature branches only
│   └── force_push              # denied by default
├── browser
│   ├── navigate
│   ├── click
│   ├── type
│   ├── extract
│   └── screenshot
├── shell
│   └── exec
│       ├── exec("cargo *")
│       └── exec("npm *")
├── http
│   ├── get
│   ├── post
│   ├── put
│   └── delete
├── memory
│   ├── read
│   └── write
├── model
│   ├── use("Claude")
│   ├── use("GPT")
│   └── use("Local")
└── agent
    ├── invoke
    └── delegate
```

### 9.3 Permission Set Resolution

The effective permission set for a node is computed by:

```
fn effective_permissions(node, agent, workflow, project):
    permissions = PermissionSet()

    # Start with project-level permissions
    permissions.merge(project.permissions)

    # Apply workflow-level restrictions
    permissions.restrict(workflow.permissions)

    # Apply agent-level restrictions
    permissions.restrict(agent.permissions)

    # Apply node-level restrictions (most specific)
    permissions.restrict(node.permissions)

    return permissions
```

Where `restrict` means:
* `allow X` in the more-specific scope can only narrow an existing `allow X`.
* `deny X` in any scope removes `X` from the allowed set.
* A scope CANNOT grant a permission that was denied by an outer scope.

```
fn PermissionSet::restrict(other):
    # Denials propagate inward
    self.denied.extend(other.denied)

    # Remaining allows must be subset of parent allows
    for allowed in other.allowed:
        if allowed in self.denied:
            # error: cannot allow what parent denied
            raise PermissionConflict
        if not self.covers(allowed):
            # error: cannot grant what parent doesn't have
            raise PermissionEscalation

    self.allowed = other.allowed
```

### 9.4 Permission Check Protocol

Before executing any tool operation, the runtime calls:

```
fn check_permission(permission_set, operation):
    # 1. Exact match
    if permission_set.allows_exact(operation):
        return Allow

    # 2. Wildcard match (most specific wins)
    if let Some(deny_rule) = permission_set.denies_wildcard(operation):
        return Deny(deny_rule)

    if let Some(allow_rule) = permission_set.allows_wildcard(operation):
        return Allow

    # 3. Default deny
    return Deny("no matching permission rule")
```

Wildcard matching uses glob semantics:
* `*` matches any single path component.
* `**` matches zero or more path components.
* `?` matches any single character.

### 9.5 Permission Enforcement Points

The permission enforcer interposes at these boundaries:

| Boundary | Mechanism |
|---|---|
| **Tool invocation** | Check before calling any tool method |
| **File I/O** | Check on open/create/delete |
| **Shell execution** | Check command against allowed list before spawning |
| **HTTP requests** | Check URL against allowed patterns |
| **Model usage** | Check model name against allowed models |
| **Agent delegation** | Check invoked agent against allowed delegate set |
| **Memory access** | Check read/write permission on the memory scope |

### 9.6 Runtime Permission Violation Handling

When a permission check fails:

1. The operation is **blocked** (the tool call, file open, etc. returns an error).
2. The node's status transitions to `Failed` with a `PermissionError`.
3. The recovery policy is consulted. Standard recovery options:
   * `retry` is usually ineffective for permission errors.
   * `fallback` to a less-privileged alternative may work.
   * `human escalate` notifies the operator: "Agent X attempted operation Y but lacks permission Z. Allow? [yes/once/always/no]"

### 9.7 Security Boundaries

```
┌────────────────────────────────────────────┐
│               Runtime Process               │
│                                            │
│  ┌──────────────────────────────────────┐ │
│  │         Sandbox (per agent)           │ │
│  │                                       │ │
│  │  ┌─────────────────────────────────┐ │ │
│  │  │    Agent Context                │ │ │
│  │  │    - Memory (isolated)          │ │ │
│  │  │    - Tools (permitted subset)   │ │ │
│  │  │    - Context (filtered)         │ │ │
│  │  └─────────────────────────────────┘ │ │
│  │                                       │ │
│  │  Allowed: filesystem.read(./project)  │ │
│  │  Blocked: shell.exec, http.post       │ │
│  └──────────────────────────────────────┘ │
│                                            │
│  ┌──────────────────────────────────────┐ │
│  │         Host System Interface         │ │
│  │  (validates all cross-boundary calls) │ │
│  └──────────────────────────────────────┘ │
└────────────────────────────────────────────┘
```

### 9.8 Network Security

* All HTTP(S) requests from tools are routed through a **proxy controller** that enforces URL allow/deny lists.
* Internal IP ranges (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16) are denied by default unless explicitly allowed.
* DNS rebinding protection: resolved IPs are validated against the original URL's domain.

### 9.9 Secret Management

The runtime MUST NOT log, checkpoint, or serialize values marked as secrets. Secret values are:

* Values where the key name matches `*secret*`, `*token*`, `*password*`, `*key*`, `*credential*`.
* Values explicitly wrapped in `Secret<T>` types.

```
struct Secret<T> {
    inner: T,
}

impl Display for Secret<T> {
    fn fmt(&self, f) { write!(f, "***") }  // never reveal
}

impl Serialize for Secret<T> {
    fn serialize(&self) { /* write placeholder, never real value */ }
}
```

Secrets are held in memory only while in use. The runtime SHOULD use `mlock` / `VirtualLock` to prevent secrets from being swapped to disk on supported platforms.

---

## 10. Tool Execution & Sandboxing

### 10.1 Tool Adapter Interface

Every tool is implemented as a backend adapter:

```
trait ToolAdapter {
    fn name() -> &str;
    fn methods() -> Vec<MethodDef>;
    fn invoke(method: &str, args: Vec<Value>, context: &Context) -> Result<Value, Error>;
}

struct MethodDef {
    name: String,
    description: String,
    parameters: Vec<ParameterDef>,
    returns: TypeRef,
}
```

### 10.2 Built-in Tool Adapters

| Adapter | Implementation |
|---|---|
| **Filesystem** | Direct OS calls with path sandboxing |
| **Git** | `git2` library or CLI subprocess |
| **Browser** | Playwright / Puppeteer process |
| **Shell** | Subprocess with command allow-listing |
| **HTTP** | reqwest / hyper with URL filtering |
| **Database** | SQL driver with query allow-listing |
| **Memory** | Internal memory manager |
| **Knowledge** | Internal RAG pipeline |
| **Model** | LLM backend router |

### 10.3 Sandbox Architecture

Sandboxes provide OS-level isolation for high-risk tools.

```
trait Sandbox {
    fn execute(command: &str, args: &[&str], working_dir: &Path) -> Result<Output>;
    fn read_file(path: &Path) -> Result<Vec<u8>>;
    fn write_file(path: &Path, content: &[u8]) -> Result<()>;
}
```

Sandbox backends:

| Backend | Isolation Level | Performance | Platform |
|---|---|---|---|
| **None** | Direct process execution | Fastest | All |
| **Subprocess** | Separate process | Fast | All |
| **Docker** | Container isolation | Moderate | Linux, macOS, Windows |
| **Firecracker** | microVM | Moderate | Linux |
| **gVisor** | User-space kernel | Moderate | Linux |

The sandbox backend is selected by policy:

```lucky
policy
    sandbox docker
    sandbox firecracker
```

### 10.4 Filesystem Sandboxing

When a tool accesses the filesystem, paths are resolved against the agent's working directory and checked against allow/deny rules:

```
fn resolve_file_path(agent_root, requested_path, permissions):
    canonical = canonicalize(agent_root.join(requested_path))

    # Must be within agent root (no escape via ..)
    if not canonical.starts_with(agent_root):
        return Err(PermissionError("path escapes sandbox"))

    # Check permissions
    relative = canonical.strip_prefix(agent_root)
    return check_permission(permissions, FilesystemAccess { path: relative })
```

### 10.5 Shell Command Sandboxing

Shell commands are validated before execution:

```
fn check_shell_command(command, permissions):
    # 1. Parse command into executable + arguments
    parts = shell_split(command)
    if parts.is_empty():
        return Deny("empty command")

    executable = parts[0]

    # 2. Check against allowed commands
    if not permissions.allows(ShellExec { command: executable }):
        return Deny("command '{executable}' not allowed")

    # 3. Check against denied patterns
    full_command = command.trim().to_lowercase()
    for pattern in permissions.denied_patterns:
        if glob_match(pattern, full_command):
            return Deny("command matches denied pattern '{pattern}'")

    # 4. Check for dangerous operators
    dangerous = [";", "&&", "||", "|", "`", "$(", ">", ">>", "<"]
    if not permissions.allows_shell_operators:
        for op in dangerous:
            if op in full_command:
                return Deny("dangerous operator '{op}' not allowed")

    return Allow
```

### 10.6 Resource Limits per Tool Invocation

| Limit | Default | Description |
|---|---|---|
| `cpu_time_ms` | 30000 | Maximum CPU time per invocation |
| `wall_time_ms` | 120000 | Maximum wall-clock time per invocation |
| `memory_mb` | 512 | Maximum memory per invocation |
| `disk_mb` | 1024 | Maximum disk usage per invocation |
| `network_mb` | 100 | Maximum network transfer per invocation |
| `file_descriptors` | 64 | Maximum open file descriptors |
| `subprocesses` | 4 | Maximum child processes |

---

## 11. Backend Adapter Interface

### 11.1 Backend Architecture

Backend adapters bridge the Lucky runtime to specific AI platforms:

```
┌──────────────────────────────────────┐
│           Backend Router             │
│                                      │
│  ┌──────────┐  ┌──────────┐         │
│  │ Claude   │  │ Codex    │  ...    │
│  │ Adapter  │  │ Adapter  │         │
│  └────┬─────┘  └────┬─────┘         │
│       │             │                │
│       ▼             ▼                │
│  ┌──────────┐  ┌──────────┐         │
│  │Anthropic │  │ OpenAI   │         │
│  │ API      │  │ API      │         │
│  └──────────┘  └──────────┘         │
└──────────────────────────────────────┘
```

### 11.2 Backend Trait

```
trait BackendAdapter: Send + Sync {
    fn name() -> &str;

    /// Initialize the backend connection
    fn initialize(config: BackendConfig) -> Result<Self>;

    /// Execute a prompt and return the model's response
    fn complete(
        &self,
        model: &str,
        messages: Vec<Message>,
        options: CompleteOptions,
    ) -> Result<CompleteResponse>;

    /// Stream tokens from the model
    fn complete_stream(
        &self,
        model: &str,
        messages: Vec<Message>,
        options: CompleteOptions,
    ) -> Result<Box<dyn Stream<Item = Token>>>;

    /// Execute a tool call on behalf of the model
    fn execute_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value>;

    /// Check backend health
    fn health_check(&self) -> Result<BackendHealth>;

    /// Get current cost/usage statistics
    fn usage(&self) -> Result<UsageStats>;
}

struct CompleteOptions {
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    stop_sequences: Vec<String>,
    tools: Vec<ToolDef>,
    response_format: Option<ResponseFormat>,
}

struct CompleteResponse {
    content: String,
    tool_calls: Vec<ToolCall>,
    finish_reason: FinishReason,
    usage: UsageStats,
    model: String,
}

struct UsageStats {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    cost_usd: f64,
    latency_ms: u64,
}
```

### 11.3 Backend Configuration

```
struct BackendConfig {
    provider: String,              // "anthropic", "openai", "google", "ollama", "openai-compatible"
    api_key: Secret<String>,
    api_base_url: Option<String>,  // for proxies and self-hosted
    default_model: String,
    models: HashMap<String, ModelConfig>,
    rate_limit: RateLimit,
    timeout: Duration,
    max_retries: u8,
}

struct ModelConfig {
    name: String,
    max_context_tokens: u64,
    max_output_tokens: u32,
    cost_per_1k_prompt_tokens: f64,
    cost_per_1k_completion_tokens: f64,
    supports_vision: bool,
    supports_tools: bool,
    supports_streaming: bool,
}

struct RateLimit {
    requests_per_minute: u32,
    tokens_per_minute: u64,
    concurrent_requests: u32,
}
```

### 11.4 Backend Router

The backend router selects the appropriate adapter for each LLM request:

```
struct BackendRouter {
    adapters: HashMap<String, Box<dyn BackendAdapter>>,
    model_map: HashMap<String, (String, String)>,  // model_name -> (adapter_name, provider_model_id)
    fallback_chain: Vec<String>,                   // try these adapters in order on failure
}

fn BackendRouter::route(&self, model_name: &str, request: LLMRequest) -> Result<LLMResponse> {
    let (adapter_name, provider_model_id) = self.model_map.get(model_name)?;
    let adapter = self.adapters.get(adapter_name)?;

    // Try primary adapter
    match adapter.complete(provider_model_id, request.messages, request.options) {
        Ok(response) => return Ok(response),
        Err(e) if e.is_transient() => {
            // Try fallback chain
            for fallback_name in &self.fallback_chain {
                if fallback_name == adapter_name { continue; }
                let fallback = self.adapters.get(fallback_name)?;
                match fallback.complete(provider_model_id, request.messages.clone(), request.options.clone()) {
                    Ok(response) => return Ok(response),
                    Err(_) => continue,
                }
            }
            Err(e)
        }
        Err(e) => Err(e),  // permanent failure
    }
}
```

### 11.5 Adapter Implementations

#### Claude Adapter (Anthropic)

```
struct ClaudeAdapter {
    client: AnthropicClient,
    config: BackendConfig,
}

impl BackendAdapter for ClaudeAdapter {
    fn complete(&self, model, messages, options) -> Result<CompleteResponse> {
        // Map Lucky Message format to Anthropic Messages API format
        // Handle tool use (Anthropic's native tool_use content blocks)
        // Extract usage from response headers
    }
}
```

#### GPT Adapter (OpenAI / OpenAI-compatible)

```
struct GPTAdapter {
    client: OpenAIClient,
    config: BackendConfig,
}

impl BackendAdapter for GPTAdapter {
    // Uses OpenAI Chat Completions API
    // Supports any OpenAI-compatible endpoint (Azure, Ollama, vLLM, etc.)
}
```

#### Local Adapter (Ollama / llama.cpp)

```
struct LocalAdapter {
    client: OllamaClient,
    config: BackendConfig,
}

impl BackendAdapter for LocalAdapter {
    // Uses Ollama API or llama.cpp server
    // Typically no rate limiting, no cost tracking
}
```

### 11.6 Tool-Use Loop

For agent invocations that involve tool calling, the runtime implements a standard tool-use loop:

```
fn agent_invoke(agent, task, inputs, backend, max_turns=10):
    messages = build_messages(agent.prompt, task.description, inputs)
    context = agent.context.clone()

    for turn in 0..max_turns:
        response = backend.complete(agent.model, messages, CompleteOptions {
            tools: agent.tools,
            ...
        })

        match response.finish_reason {
            Stop => return Complete(response.content),
            ToolCalls => {
                for tool_call in response.tool_calls:
                    check_permission(agent.permissions, tool_call)
                    result = execute_tool(tool_call)
                    messages.push(AssistantMessage { tool_calls })
                    messages.push(ToolResult { result })
            }
            LengthLimit => {
                # Context window exceeded; summarize or compress
                messages = compress_messages(messages)
            }
        }

    return Err("Max tool-use turns exceeded")
```

---

## 12. Recovery & Fault Tolerance

### 12.1 Error Classification

The runtime classifies errors to determine the appropriate recovery strategy:

```
enum ErrorCategory {
    Transient,       # temporary: network, rate limit, timeout
    Permanent,       # unrecoverable: validation, type error, permission
    Resource,        # resource exhaustion: memory, disk, budget
    Cancellation,    # explicit cancellation
    Timeout,         # exceeded configured time limit
    Permission,      # capability-security violation
    Model,           # LLM-specific: content filter, safety refusal
    Tool,            # tool execution failure
    Internal,        # runtime bug (should not happen)
}

fn classify_error(error):
    match error.code:
        # Network errors
        408 | 429 | 502 | 503 | 504 => Transient
        # Permission errors
        401 | 403 => Permission
        # Validation errors
        400 | 422 => Permanent
        # LLM content filter
        400 if "content_filter" in error.message => Model
        # Resource errors
        if "memory" in error.message => Resource
        if "budget" in error.message => Resource
        # Default
        _ => Permanent
```

### 12.2 Recovery State Machine

```
   ┌──────────┐
   │  Failed   │
   └────┬─────┘
        │ classify error
        ▼
   ┌──────────┐
   │ Evaluate  │
   │ Policy    │
   └────┬─────┘
        │
   ┌────┼─────────────────────────────────┐
   │    │                                 │
   ▼    ▼                                 ▼
Retry Fallback                          Human
   │    │                                 │
   │    ├──→ Execute alternative     ┌────┴────┐
   │    │                            │ Pending  │
   │    ▼                            │ Approval │
   │  Completed                      └────┬────┘
   │                                      │
   ▼                                 ┌────┴────────────┐
 Retry                              │                  │
   │                                ▼                  ▼
   ├──→ (attempts < max) → Running  │              ┌────────┐
   │                                │ Approved     │Rejected│
   └──→ (exhausted) → Next recovery │              └────────┘
                                    ▼
                                 Completed │ Aborted
```

### 12.3 Retry with Backoff

```
fn retry_with_backoff(node, max_retries, backoff_strategy):
    for attempt in 1..=max_retries:
        delay = compute_delay(attempt, backoff_strategy)
        sleep(delay)

        # Checkpoint before retry
        checkpoint_manager.snapshot_node(node)

        # Re-execute
        result = execute_node(node)
        match result {
            Ok(output) => return Ok(output),
            Err(e) if not e.is_transient() => return Err(e),
            Err(e) => log.warn("Retry {attempt}/{max_retries}: {e}"),
        }

    return Err("max retries ({max_retries}) exceeded")

fn compute_delay(attempt, strategy):
    match strategy:
        Linear(base=1s)       => base * attempt
        Exponential(base=1s, max=10m) =>
            min(base * 2.pow(attempt - 1), max)
        with_jitter(delay) =>
            delay * random(0.5, 1.5)
```

### 12.4 Circuit Breaker

The runtime implements a circuit breaker to prevent cascading failures:

```
struct CircuitBreaker {
    state: Closed | HalfOpen | Open,
    failure_count: AtomicUsize,
    failure_threshold: usize,       # open after N failures (default: 5)
    recovery_timeout: Duration,     # try half-open after (default: 30s)
    last_failure_time: Instant,
    half_open_max_requests: usize,  # limit requests in half-open (default: 1)
}

fn CircuitBreaker::call(f):
    match self.state {
        Open => {
            if elapsed(self.last_failure_time) > self.recovery_timeout:
                self.state = HalfOpen
            else:
                return Err(CircuitBreakerOpen)
        }
        HalfOpen => {
            if self.current_requests >= self.half_open_max_requests:
                return Err(CircuitBreakerOpen)
        }
        Closed | HalfOpen => {}
    }

    result = f()
    match result {
        Ok(value) => {
            if self.state == HalfOpen:
                self.state = Closed
                self.failure_count = 0
            Ok(value)
        }
        Err(e) => {
            self.failure_count += 1
            self.last_failure_time = now()
            if self.failure_count >= self.failure_threshold:
                self.state = Open
            Err(e)
        }
    }
```

### 12.5 Transaction Rollback

```
fn execute_transaction(steps):
    completed = Vec::new()
    for step in steps:
        result = execute_step(step)
        match result:
            Ok(output) => completed.push((step, output)),
            Err(e) => {
                # Rollback completed steps in reverse order
                for (step, _) in completed.reverse():
                    if let Some(rollback_fn) = step.rollback:
                        rollback_result = execute_rollback(rollback_fn)
                        if rollback_result.is_err():
                            log.error("Rollback failed for step: {step.id}")
                            # Escalate to human
                            human_escalate(step, rollback_result.unwrap_err())
                return Err(e)
            }
    return Ok(collect_outputs(completed))

fn execute_rollback(rollback_fn):
    # Rollback is best-effort
    # If it fails, we escalate but do not abort the overall recovery
    match rollback_fn() {
        Ok(_) => Ok(()),
        Err(e) => {
            telemetry.emit(RollbackFailed { error: e })
            Err(e)
        }
    }
```

---

## 13. Cost & Resource Management

### 13.1 Cost Model

The runtime tracks cost at multiple granularities:

```
struct CostTracker {
    total_cost_usd: AtomicF64,
    cost_by_model: DashMap<String, AtomicF64>,
    cost_by_node: DashMap<NodeId, AtomicF64>,
    cost_by_agent: DashMap<AgentId, AtomicF64>,
    budget: Budget,
    budget_consumed_callback: Option<Box<dyn Fn(f64)>>,
}

struct Budget {
    total_usd: f64,
    per_workflow_usd: Option<f64>,
    per_agent_usd: Option<f64>,
    per_day_usd: Option<f64>,
    alert_thresholds: Vec<f64>,   # e.g., [0.5, 0.75, 0.9, 1.0]
}
```

### 13.2 Cost Calculation

Cost is calculated from backend-reported token usage:

```
fn calculate_cost(model_config, prompt_tokens, completion_tokens):
    prompt_cost = prompt_tokens / 1000.0 * model_config.cost_per_1k_prompt_tokens
    completion_cost = completion_tokens / 1000.0 * model_config.cost_per_1k_completion_tokens
    return prompt_cost + completion_cost
```

For models that don't report cost (local models), cost is computed from the local compute rate:

```
fn calculate_local_cost(gpu_time_seconds, gpu_type):
    # Configurable rate per GPU-hour
    rate = config.local_gpu_rates.get(gpu_type) or 0.0
    return gpu_time_seconds / 3600.0 * rate
```

### 13.3 Budget Enforcement

```
fn check_budget(cost_tracker, new_cost):
    # Per-workflow budget
    if let Some(limit) = cost_tracker.budget.per_workflow_usd:
        if cost_tracker.total_cost_usd + new_cost > limit:
            return BudgetExceeded("workflow budget exceeded")

    # Per-agent budget
    if let Some(limit) = cost_tracker.budget.per_agent_usd:
        # checked against agent-specific cost

    # Total budget
    if cost_tracker.total_cost_usd + new_cost > cost_tracker.budget.total_usd:
        return BudgetExceeded("total budget exceeded")

    return Ok(())

fn emit_budget_alerts(cost_tracker):
    consumed = cost_tracker.total_cost_usd / cost_tracker.budget.total_usd
    for threshold in cost_tracker.budget.alert_thresholds:
        if consumed >= threshold and not already_alerted(threshold):
            telemetry.emit(BudgetAlert {
                threshold: threshold,
                consumed_usd: cost_tracker.total_cost_usd,
                budget_usd: cost_tracker.budget.total_usd,
            })
            if let Some(cb) = cost_tracker.budget_consumed_callback:
                cb(consumed)
```

### 13.4 Cost Optimization Strategies

#### Model Routing

```
fn select_model(task):
    # 1. User-specified model override
    if task.model_override is not None:
        return task.model_override

    # 2. Policy-based routing
    policy = task.effective_policy()
    if policy.prefer_cheapest_model:
        candidates = available_models.filter(m => m.capabilities >= task.requirements)
        return candidates.min_by(|m| m.cost_per_1k_tokens)
    else:
        return policy.default_model
```

#### Response Caching

```
struct LLMCache {
    store: LruCache<CacheKey, CompleteResponse>,
    max_entries: usize,
    ttl: Duration,
}

struct CacheKey {
    model: String,
    messages_hash: String,  // SHA256 of canonicalized messages
    options_hash: String,   // SHA256 of serialized options
    temperature: u32,       // quantized to 2 decimal places
}

fn LLMCache::get_or_complete(key, backend_fn):
    if let Some(cached) = self.store.get(&key):
        if now() - cached.timestamp < self.ttl:
            return Ok(cached.response.clone())

    response = backend_fn()
    self.store.insert(key, CachedResponse {
        response: response.clone(),
        timestamp: now(),
    })
    return response
```

Caching applies only to operations explicitly marked as cacheable (read-only prompts, no tool calls, temperature = 0 preferred).

#### Batching

Where the backend supports it, multiple independent LLM calls are batched:

```
fn batch_complete(requests, adapter, max_batch_size, max_wait_ms):
    if requests.len() < 2:
        return adapter.complete(requests[0])

    # Wait up to max_wait_ms for more requests to accumulate
    batch = collect_batch(max_batch_size, max_wait_ms)

    if adapter.supports_batching:
        return adapter.batch_complete(batch)
    else:
        return parallel_map(batch, |r| adapter.complete(r))
```

---

## 14. Observability & Telemetry

### 14.1 Telemetry Architecture

```
┌───────────────────────────────────────────┐
│               Runtime                      │
│                                            │
│  ┌─────────┐  ┌──────────┐  ┌──────────┐ │
│  │ Metrics │  │  Traces  │  │   Logs   │ │
│  │ (OTel)  │  │  (OTel)  │  │ (JSON)   │ │
│  └────┬────┘  └────┬─────┘  └────┬─────┘ │
│       │            │             │        │
│       └────────────┼─────────────┘        │
│                    │                      │
│              ┌─────▼──────┐               │
│              │  Exporter  │               │
│              │  (pluggable)│               │
│              └─────┬──────┘               │
└────────────────────┼──────────────────────┘
                     │
          ┌──────────┼──────────┐
          ▼          ▼          ▼
     ┌────────┐ ┌────────┐ ┌────────┐
     │OTLP    │ │Prometh │ │  File  │
     │gRPC    │ │exporter│ │exporter│
     └────────┘ └────────┘ └────────┘
```

### 14.2 Metrics

| Metric | Type | Description |
|---|---|---|
| `lucky.nodes.total` | Counter | Total nodes in execution |
| `lucky.nodes.completed` | Counter | Nodes completed successfully |
| `lucky.nodes.failed` | Counter | Nodes that failed |
| `lucky.nodes.retried` | Counter | Retry attempts |
| `lucky.nodes.active` | Gauge | Currently executing nodes |
| `lucky.scheduler.queue_depth` | Gauge | Nodes in ready queue |
| `lucky.llm.calls` | Counter | Total LLM calls |
| `lucky.llm.tokens.prompt` | Counter | Prompt tokens used |
| `lucky.llm.tokens.completion` | Counter | Completion tokens used |
| `lucky.llm.latency_ms` | Histogram | LLM call latency |
| `lucky.llm.cost_usd` | Counter | Cumulative cost |
| `lucky.tool.calls` | Counter | Tool invocations |
| `lucky.tool.errors` | Counter | Tool invocation errors |
| `lucky.checkpoint.size_bytes` | Histogram | Checkpoint size |
| `lucky.checkpoint.duration_ms` | Histogram | Checkpoint write time |
| `lucky.memory.heap_bytes` | Gauge | Value heap size |
| `lucky.memory.agent_entries` | Gauge | Agent memory entries |
| `lucky.workflow.duration_ms` | Histogram | Total workflow duration |
| `lucky.human.approvals.pending` | Gauge | Pending human approvals |
| `lucky.human.approvals.duration_ms` | Histogram | Time to approval decision |

### 14.3 Tracing

Each node execution produces a span:

```
Span {
    trace_id: UUID,        # per-run trace ID
    span_id: UUID,         # per-invocation
    parent_span_id: UUID,  # parent node's span (for sub-tasks)
    name: "node:{node_id}",
    kind: Internal | Client | Server,
    start_time: Timestamp,
    end_time: Timestamp,
    status: Ok | Error { message },
    attributes: {
        "lucky.node.id": node_id,
        "lucky.node.kind": node.kind,
        "lucky.node.label": node.label,
        "lucky.agent.id": agent_id,
        "lucky.model.name": model_name,
        "lucky.cost.usd": cost,
        "lucky.tokens.total": tokens,
    },
    events: [
        { name: "step.started", timestamp, attributes: { step_index } },
        { name: "tool.called", timestamp, attributes: { tool, method } },
        { name: "llm.called", timestamp, attributes: { model, tokens } },
        { name: "checkpoint.saved", timestamp, attributes: { checkpoint_id } },
    ],
}
```

### 14.4 Structured Logging

```
enum LogLevel {
    Trace, Debug, Info, Warn, Error, Fatal,
}

struct LogEntry {
    timestamp: Timestamp,
    level: LogLevel,
    target: String,           # module path
    message: String,
    run_id: UUID,
    node_id: Option<UUID>,
    agent_id: Option<UUID>,
    structured_data: Value,   # arbitrary JSON
}

# Usage:
log.info!("Node started", {
    node_id: node.id,
    node_kind: node.kind,
    estimated_cost: node.estimated_cost_usd,
})
```

### 14.5 Audit Trail

Every permission-sensitive operation produces an audit event:

```
struct AuditEvent {
    timestamp: Timestamp,
    run_id: UUID,
    agent_id: UUID,
    operation: String,         # e.g., "filesystem.write", "git.push"
    target: String,            # e.g., "/path/to/file", "branch:main"
    result: Allowed | Denied(reason) | Error(message),
    context: Value,            # relevant context (redacted for secrets)
}
```

Audit events are written to an append-only log and are never deleted. They can be shipped to a SIEM via the telemetry exporter.

---

## 15. Distributed Execution

### 15.1 Overview

For large-scale workflows (100+ agents, 1000+ tasks), the runtime supports distributed execution across multiple nodes.

### 15.2 Architecture

```
┌─────────────────────────────────────────────────┐
│                  Coordinator                     │
│                                                  │
│  ┌────────────┐  ┌──────────┐  ┌─────────────┐ │
│  │ Scheduler  │  │ DAG      │  │ Checkpoint   │ │
│  │            │  │ State    │  │ Coordinator  │ │
│  └─────┬──────┘  └──────────┘  └─────────────┘ │
│        │ dispatches nodes                        │
└────────┼─────────────────────────────────────────┘
         │
    ┌────┴──────────────────────┐
    │     Message Bus (NATS)    │
    └────┬──────────────────────┘
         │
    ┌────┼──────────────┬──────────────┐
    ▼    ▼              ▼              ▼
┌──────┐ ┌──────┐  ┌──────┐      ┌──────┐
│Worker│ │Worker│  │Worker│ ...  │Worker│
│  1   │ │  2   │  │  3   │      │  N   │
└──────┘ └──────┘  └──────┘      └──────┘
```

### 15.3 Coordinator

The coordinator owns the DAG state and scheduling decisions. It is a single node (with hot standby for HA).

```
struct Coordinator {
    dag_state: DAGState,
    scheduler: DistributedScheduler,
    checkpoint_coordinator: CheckpointCoordinator,
    workers: Vec<WorkerHandle>,
    bus: MessageBus,
}
```

### 15.4 Worker

Workers receive node dispatch messages and execute them locally.

```
struct Worker {
    id: UUID,
    capacity: WorkerCapacity,
    local_executor: ExecutionEngine,
    bus: MessageBus,
}

struct WorkerCapacity {
    max_concurrent_nodes: u32,
    available_memory_mb: u64,
    supported_backends: Vec<String>,
    labels: HashMap<String, String>,  # for affinity scheduling
}
```

### 15.5 Message Protocol

```
enum Message {
    # Coordinator → Worker
    DispatchNode { node: Node, context: ContextLayer },

    # Worker → Coordinator
    NodeCompleted { node_id: UUID, output: Value, cost: f64 },
    NodeFailed { node_id: UUID, error: ErrorValue, recovery_attempted: bool },
    NodeCheckpointed { node_id: UUID, checkpoint_id: UUID },
    WorkerHeartbeat { worker_id: UUID, load: f64, memory_mb: u64 },
    WorkerRegister { worker_id: UUID, capacity: WorkerCapacity },

    # Coordinator → Worker
    CancelNode { node_id: UUID },
    Shutdown,
}
```

### 15.6 Affinity Scheduling

Nodes are preferentially dispatched to workers that already hold the relevant agent state:

```
fn select_worker(workers, node):
    # 1. Agent affinity: prefer worker that last executed this agent
    agent_worker = workers.filter(|w| w.has_agent_cache(node.agent_ref))
    if not agent_worker.is_empty():
        return agent_worker.min_by(|w| w.current_load)

    # 2. Capacity: pick the least-loaded worker that supports the required backends
    eligible = workers.filter(|w|
        w.supported_backends.contains(node.required_backend)
        && w.available_memory_mb >= node.resource_requirements.memory_mb
    )

    return eligible.min_by(|w| w.current_load)
```

### 15.7 Fault Tolerance in Distributed Mode

* **Worker failure**: The coordinator detects a worker failure via heartbeat timeout (default: 30s). Nodes assigned to the dead worker are re-queued and dispatched to healthy workers, resuming from the last checkpoint.
* **Coordinator failure**: A hot standby coordinator replays the message log and resumes scheduling. Workers reconnect to the new coordinator transparently.
* **Network partition**: The coordinator uses a distributed lease (etcd/consul) to ensure only one active coordinator exists. Workers in the minority partition are fenced.

---

## 16. Runtime Configuration

### 16.1 Configuration File

The runtime configuration is specified in `lucky.toml` (for development) or via environment variables (for production).

```toml
[runtime]
max_concurrency = 16
poll_interval_ms = 100
node_default_timeout_ms = 300000
checkpoint_after_each_node = false
checkpoint_interval_minutes = 5

[runtime.sandbox]
backend = "docker"           # none | subprocess | docker | firecracker | gvisor
image = "lucky-sandbox:latest"
network = "isolated"         # isolated | host | bridged

[runtime.memory]
heap_size_mb = 4096
agent_memory_max_entries = 10000
vector_index_max_vectors = 100000
cache_max_entries = 1000
cache_ttl_seconds = 3600

[runtime.checkpoint]
backend = "filesystem"       # filesystem | s3 | database
path = "./.lucky/checkpoints"
max_checkpoints = 50
retention_hours = 168
compression = true

[runtime.cost]
total_budget_usd = 100.0
per_workflow_budget_usd = 10.0
alert_thresholds = [0.5, 0.75, 0.9, 1.0]

[runtime.backends]
[runtime.backends.claude]
provider = "anthropic"
api_key = "${ANTHROPIC_API_KEY}"
default_model = "claude-sonnet-4-20250514"
rate_limit_requests_per_minute = 50

[runtime.backends.claude.models]
"claude-sonnet-4-20250514" = { max_context_tokens = 200000, max_output_tokens = 4096, cost_per_1k_prompt_tokens = 3.0, cost_per_1k_completion_tokens = 15.0 }

[runtime.backends.gpt]
provider = "openai"
api_key = "${OPENAI_API_KEY}"
default_model = "gpt-4o"
api_base_url = "https://api.openai.com/v1"
rate_limit_requests_per_minute = 500

[runtime.backends.gpt.models]
"gpt-4o" = { max_context_tokens = 128000, max_output_tokens = 16384, cost_per_1k_prompt_tokens = 2.5, cost_per_1k_completion_tokens = 10.0 }

[runtime.backends.local]
provider = "ollama"
api_base_url = "http://localhost:11434"
default_model = "llama3"

[runtime.telemetry]
metrics_enabled = true
traces_enabled = true
log_level = "info"
log_format = "json"

[runtime.telemetry.otlp]
endpoint = "http://localhost:4317"
protocol = "grpc"

[runtime.telemetry.prometheus]
enabled = false
port = 9090

[runtime.security]
secret_redaction = true
audit_log_path = "./.lucky/audit.log"

[runtime.distributed]
enabled = false
coordinator_address = "localhost:9700"
worker_count = 4
message_bus = "nats://localhost:4222"

[runtime.swarm]
max_concurrency = 100
batch_size = 10
default_merge_strategy = "concat"
```

### 16.2 Environment Variables

All configuration values can be overridden by environment variables:

```
LUCKY_RUNTIME_MAX_CONCURRENCY=32
LUCKY_RUNTIME_COST_TOTAL_BUDGET_USD=200.0
LUCKY_RUNTIME_BACKENDS_CLAUDE_API_KEY=sk-ant-...
LUCKY_RUNTIME_TELEMETRY_LOG_LEVEL=debug
```

Environment variables use the prefix `LUCKY_` and replace dots and dashes with underscores. Nested keys use double underscores.

### 16.3 Runtime API

The runtime exposes a gRPC API for external control:

```protobuf
service LuckyRuntime {
    rpc StartRun(StartRunRequest) returns (StartRunResponse);
    rpc GetRunStatus(GetRunStatusRequest) returns (RunStatus);
    rpc CancelRun(CancelRunRequest) returns (CancelRunResponse);
    rpc ListRuns(ListRunsRequest) returns (ListRunsResponse);
    rpc GetNodeStatus(GetNodeStatusRequest) returns (NodeStatus);
    rpc RespondApproval(RespondApprovalRequest) returns (RespondApprovalResponse);
    rpc StreamRunEvents(StreamRunEventsRequest) returns (stream RunEvent);
    rpc GetCostReport(GetCostReportRequest) returns (CostReport);
    rpc ListCheckpoints(ListCheckpointsRequest) returns (ListCheckpointsResponse);
    rpc RestoreFromCheckpoint(RestoreFromCheckpointRequest) returns (RestoreFromCheckpointResponse);
}
```

---

*End of Lucky Runtime Specification, Version 0.1*
