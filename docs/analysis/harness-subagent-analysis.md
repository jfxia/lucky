# Analysis: How Lucky Can Improve Based on Solon AI Harness's Multi-Agent Patterns

Based on the article: [🔥 多 Agent 协作实战：任务编排与子代理系统](https://www.oschina.net/news/471580)

---

## What the Article Describes

Solon AI Harness implements a **Sub-Agent System** where a main agent (orchestrator) delegates work to specialized sub-agents via `task`/`multitask` tools. The example: a writing team with researcher → planner → writer → reviewer, orchestrated as a DAG by an "editor-in-chief" agent.

### 6 Key Design Principles from the Article

| Principle | Meaning |
|---|---|
| **Single Mind** (心智单一) | Each sub-agent does one thing, does it well |
| **Clear Contract** (契约明确) | Input/output format declared in system prompt |
| **Right Granularity** (粒度适中) | A task = "one independently deliverable intermediate product" |
| **Mind & Memory Isolation** (心智与记忆的隔离) | Independent system prompt + session per sub-agent |
| **Orchestrated > Autonomous** | Central orchestration as primary, peer-to-peer as secondary |
| **Dynamic Registration** | Sub-agents created at runtime, not pre-declared |

---

## Lucky vs Harness: Feature Comparison

| Feature | Lucky (v0.2) | Harness | Gap |
|---|---|---|---|
| DAG workflow | ✅ `workflow` with `->`, `parallel`, `if/else` | ✅ `multitask` with dependencies | Comparable |
| Agent declarations | ✅ Static `agent` blocks | ✅ `AgentDefinition` builder | Lucky's are richer (tools, memory, permissions) |
| Dynamic sub-agents | ❌ Not supported | ✅ `AgentManager.addAgent()` at runtime | **Big gap** |
| Agent registry | ❌ No runtime registry | ✅ `AgentManager.getAgents()` | **Missing** |
| Sub-session isolation | ❌ Single session only | ✅ Independent sub-sessions | **Missing** |
| Multi-level delegation | ❌ Flat workflow only | ✅ Sub-agents can also use `task` | **Missing** |
| External agent definitions | ❌ Not supported | ✅ Mount system (YAML/JSON files) | **Missing** |
| Auto-rethink | ❌ Not supported | ✅ `autoRethink(true)` | **Missing** |
| Incremental re-execution | ⚠️ Checkpoint resume (coarse) | ✅ Re-delegate individual steps | **Improvement needed** |
| Type-checked agent contracts | ⚠️ `input`/`output` declared but not enforced | ✅ System prompt defines contract | **Improvement needed** |
| Plugin-style agent loading | ❌ Not supported | ✅ Mount dirs for agent definitions | **Missing** |

---

## Proposed Improvements for Lucky

### 1. Dynamic Sub-Agent System (Highest Priority)

The article's most powerful pattern: **agents created at runtime, not just compile-time**.

```lucky
// Current Lucky — static only
agent Researcher
  model DeepSeek
  tools Browser
```

```lucky
// Proposed: dynamic agent registration
workflow WritingPipeline
  // Register sub-agents at runtime
  register agent Researcher
    model DeepSeek
    tools Browser
    prompt "You are a technical researcher..."

  register agent Writer
    model DeepSeek
    tools Filesystem
    prompt "You are a technical writer..."

  // Use them in the workflow
  Researcher.search(topic) -> Writer.draft(research) -> Reviewer.check(draft)
```

**Runtime API equivalent** (for platforms embedding Lucky):

```c
// C SDK — dynamic agent registration
lucky_agent_def_t agent = {
    .name = "researcher",
    .system_prompt = "You are a technical researcher...",
    .tools = (char*[]){"Browser", "Filesystem", NULL},
};
lucky_session_register_agent(session, &agent);
```

### 2. Sub-Session Isolation

The article emphasizes that each sub-agent needs **independent mind and memory**. Lucky's context propagation is automatic — which means everything flows everywhere. This is a bug, not a feature, for multi-agent systems.

```lucky
// Proposed: isolated context scoping
workflow WritingPipeline
  context
    topic: String          // inherited by all sub-agents (read-only)

  // Each sub-agent gets a scoped, isolated context
  Researcher.search(topic)
    isolate                // researcher sees only topic, not other agents' history
    -> Writer.draft
    isolate                // writer sees only topic + research output
    -> Reviewer.check
    isolate                // reviewer sees only topic + draft (not research internals)
```

**Design principle:** Context inheritance should be explicit, not automatic. `isolate` creates a sub-session that:
- Inherits declared context entries (opt-in)
- Does NOT carry other agents' conversation history
- Gets its own memory scope
- Returns structured output that becomes context for downstream

### 3. Multi-Level Delegation

The article shows sub-agents that can themselves delegate to other agents. Lucky's workflow is flat.

```lucky
// Proposed: nested delegation
agent EditorInChief
  model DeepSeek
  tools task            // the Editor can use `task` to delegate
  prompt "You coordinate the writing team..."

workflow ArticleProduction
  EditorInChief.run(goal)
  // Editor internally delegates to researcher → planner → writer → reviewer
  // Each of those could also delegate further
```

This requires the `task` tool to be composable — an agent that has the `task` tool can call other agents. The runtime tracks the delegation tree, not just a flat DAG.

### 4. Agent Registry at Runtime

```lucky
// Proposed: runtime agent management
// Query available agents
let available = agents()                    // -> ["Researcher", "Writer", ...]

// Register from external definition
register agent from "./agents/researcher.yaml"

// Register dynamically
register agent CustomAgent
  prompt "You handle edge cases..."
  tools Filesystem, Shell
```

The `agents()` built-in returns registered agents. The `register` statement accepts both inline declarations and external file references.

### 5. External Agent Definitions (Mount System)

The article's mount system maps agents from YAML files on disk. Lucky could support this natively:

```yaml
# agents/researcher.yaml
name: researcher
description: "Technical research specialist"
prompt: |
  You are a senior technical researcher...
  Output format: research brief in markdown
tools:
  - Browser
  - Filesystem
```

```lucky
// In .lk file — mount a directory of agent definitions
mount agents from "./agents"

// Now `researcher` is available as if declared inline
workflow ResearchTask
  researcher.search("Rust async patterns")
```

### 6. Auto-Rethink / Adaptive Workflows

The article's `autoRethink(true)` lets the agent self-correct. Lucky is purely static — the DAG is fixed at compile time.

```lucky
// Proposed: adaptive recovery
policy AdaptivePolicy
  rethink on partial_failure    // agent can retry with a different approach
  max_rethink 3                 // up to 3 re-planning attempts
  escalate_on_stuck             // if retthink fails, escalate to human

agent EditorInChief
  model DeepSeek
  policy AdaptivePolicy
  tools task
```

When a node fails, instead of just retrying the same task, the runtime gives the agent a chance to **re-plan** — choose a different sub-agent, reformulate the prompt, or try a different approach.

### 7. Contract Enforcement

The article stresses clear input/output contracts. Lucky declares them in `input`/`output` but doesn't enforce them at runtime.

```lucky
// Proposed: enforced contracts
agent Researcher
  output brief: ResearchBrief     // typed output — runtime validates shape

agent Writer
  input brief: ResearchBrief      // runtime checks that input matches
  output draft: ArticleDraft

// Type definitions for contracts
type ResearchBrief = {
  findings: List<String>,
  sources: List<Source>,
  confidence: Float,
}

type ArticleDraft = String
```

When a sub-agent's output doesn't match the declared contract (wrong shape, missing fields, low confidence), the runtime rejects it and the orchestrator can re-delegate.

---

## Summary: Priority Ranking

| # | Improvement | Effort | Impact | Closes Gap With |
|---|---|---|---|---|
| 1 | **Dynamic agent registration** | L | 🔥🔥🔥🔥🔥 | Core missing pattern |
| 2 | **Sub-session isolation** | M | 🔥🔥🔥🔥 | Mind & memory isolation |
| 3 | **External agent definitions (mount)** | M | 🔥🔥🔥🔥 | Plugin system |
| 4 | **Multi-level delegation** | L | 🔥🔥🔥 | Nested orchestration |
| 5 | **Contract enforcement** | M | 🔥🔥🔥 | Type safety for agent I/O |
| 6 | **Auto-rethink / adaptive** | M | 🔥🔥 | Self-correction |
| 7 | **Agent registry runtime API** | S | 🔥🔥 | Introspection |

---

## Quick Take

Lucky's static compilation model is both its strength (determinism, verification, auditability) and its weakness (inflexibility for the dynamic patterns Harness demonstrates). The right approach is **not** to make Lucky fully dynamic — that would lose the verifiability that makes it valuable. Instead:

1. Add **runtime registration hooks** — agents can be registered at runtime, but the workflow DAG is still compiled and verified before execution.
2. Add **`isolate` scoping** — make context inheritance opt-in, not automatic.
3. Add **mount declarations** — external agent definitions loaded at compile time, not runtime.

This way Lucky keeps its compile-time guarantees while gaining the flexibility Harness demonstrates.
