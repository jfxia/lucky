![Lucky-lang logo](logo/logo256.png)

## Lucky: a Goal-Oriented Agent Language

Lucky is a goal-oriented orchestration language for AI agents. Unlike traditional programming languages designed around CPU execution and deterministic algorithms, Lucky is designed around goal execution — the coordination of autonomous AI agents that reason, plan, and act under explicit human supervision.

> **📖 New to Lucky?** Start with the [Quickstart Guide](docs/quickstart.md) or follow the [Tutorial](docs/tutorial.md).

Most programming languages today assume:

* Humans write code.
* Compilers optimize execution.
* AI merely assists.

An AI-native language should instead assume:

> **AI is the primary programmer. Humans are architects.**

That changes almost everything.

In Lucky:

**-Functions become Tasks** (deterministic units of work)

**-Classes become Agents** (stateful entities with memory and capabilities)

**-Programs become Workflows** (graphs of tasks and agents)

**-Variables become Context** (automatically propagated execution state)

**-Exceptions become Recovery Policies** (explicit retry, fallback, escalation)

**-Compilation produces an execution graph (Lucky IR)** instead of native machine code


---

# Lucky

> **Lucky = Language for AI Agents**

> *"Think in goals, not syntax."*

---

# Philosophy

Lucky is **not another systems language**.

It is the orchestration language for AI.

Its abstraction level sits **above Python**.

Think of the hierarchy:

```
Assembly
↑
C
↑
Rust/C++
↑
Go
↑
Python
↑
Lucky
↑
Natural Language
```

Lucky fills the missing layer between natural language and executable software.

---

# Core Principles

## 1. Intent-first

Instead of

```python
for user in users:
    if user.age > 18:
        ...
```

Lucky thinks

```lucky
select adults from users
```

or

```lucky
users
    |> where age > 18
```

---

## 2. Everything is an Agent

Not objects.

Not classes.

Not threads.

Everything is an Agent.

```lucky
agent Browser
agent Researcher
agent Planner
agent Coder
agent Tester
```

An agent owns

* memory
* tools
* prompts
* reasoning strategy
* permissions

Example

```lucky
agent Researcher

Researcher.search(
    "latest AI papers"
)
```

---

## 3. LLM is Built-in

No SDK. No API wrapper.

```lucky
model Claude(
    provider = "anthropic",
    version = "claude-sonnet-4-20250514",
)

model GPT(
    provider = "openai",
    version = "gpt-4o",
)

use Claude

ask GPT:
    summarize report
```

Models are declared and switched at the language level — no imports needed.

---

# First-class Concepts

Instead of

```
class
function
thread
```

Lucky has

```
goal
task
workflow
tool
agent
memory
context
resource
permission
```

---

# Program Structure

Example

```lucky
project WebsiteBuilder

use Claude

agent Planner
agent Designer
agent Coder
agent Reviewer

goal BuildLandingPage

workflow
    Planner ->
    Designer ->
    Coder ->
    Reviewer
```

---

# Tasks

Instead of functions

```
task download()
```

Lucky uses

```lucky
task AnalyzeRepo
    input
        repo
    output
        architecture.md
    steps
        clone repo
        read README
        understand architecture
        generate report
```

Looks like YAML?

No.

Indentation represents executable structure.

---

# Pipeline Syntax

Everything can flow.

```lucky
files
    |> filter *.py
    |> summarize
    |> save report.md
```

or

```lucky
Search.search("AI agents")
    |> extract
    |> rank
    |> answer
```

---

# Context is Native

Instead of manually passing variables

```python
foo(user, memory, config, session)
```

Lucky

```lucky
context
    user
    memory
    repo
    history
```

Everything automatically propagates.

---

# Memory

Agents have persistent memory and knowledge.

```lucky
memory ProjectMemory
    scope project

agent Planner
    memory ProjectMemory

Planner.memory.remember("architecture", architectureDoc)
let docs = Planner.memory.recall("coding conventions")
```

---

# Multi-Agent

Native.

```lucky
parallel
    Researcher
    Architect
    Security
wait
```

or

```lucky
swarm
    20 Reviewer
```

---

# Prompt Blocks

Instead of strings

```python
prompt = """
...
"""
```

Lucky

```lucky
prompt Reviewer
    Review code.
    Focus
        - bugs
        - security
        - performance
```

Prompt is a language construct.

---

# Tool Calling

Instead of

```python
browser.search()
```

Lucky

```lucky
tool Browser
tool Git
tool Docker
tool Shell
```

Use

```lucky
Browser.search("AI Agent frameworks")
Git.commit("feat: add new workflow")
Shell.exec("npm test")
```

---

# Permissions

Built-in capability security.

```lucky
permissions
    allow
        filesystem.read
        git.clone
        browser.search
    deny
        filesystem.delete
        shell.exec
```

---

# Error Recovery

Instead of

```python
try:

except:
```

Lucky

```lucky
attempt
    build
recover
    ask Reviewer
    retry
```

---

# Human Approval

Native.

```lucky
approval
    before deploy
```

or

```lucky
ask human
    Delete production database?
```

---

# Workflows

Lucky is excellent for orchestration.

```lucky
workflow
    Research
        ->
    Plan
        ->
    Implement
        ->
    Review
        ->
    Fix
        ->
    Deploy
```

This is executable.

---

# AI Reasoning

Reasoning becomes explicit.

```lucky
reason
    deep
```

or

```lucky
reason
    fast
```

or

```lucky
reason
    none
```

---

# Confidence

Every result carries confidence.

```lucky
let result = ai.ask(question)

if result.confidence > 0.9
    use(result.value)
else
    result.citations
    |> ResearchAgain
```

---

# Native Tools

```lucky
tool Browser
tool Git
tool Shell
tool HTTP
```

Use

```lucky
Browser.search("AI Agent Framework")
    |> extract
    |> summarize

Git.clone(repo)
Git.commit("feat: add new workflow")
Git.push()

Shell.exec("cargo build")

HTTP.get("/api/users")
```

---

# Reactive Programming

```lucky
when
    README changes
run
    ArchitectureReview
```

---

# Deployment

```lucky
deploy
    Docker
    AWS
    Azure
    Local
```

---

# Package System

Not libraries. Capabilities.

```lucky
import browser
import database
import vision
import speech
import search
import rag
```

---

# Execution Flow

Lucky compiles to a language-neutral Intermediate Representation (Lucky IR). Multiple AI coding platforms can execute the same IR consistently via the Lucky Tool Protocol (LTP):

```
Lucky Source (.lk)
    ↓
Parser → Semantic Analyzer
    ↓
Lucky IR (.lir)
    ↓
LTP Server (Lucky Runtime)
    ↓
Backend Adapters
    ↓
Claude Code · Codex CLI · OpenCode · Cursor · Dify
```

# End-to-End Example

```lucky
project Website

use Claude

agent Planner
agent Coder
agent Tester

goal BuildBlog

workflow
    Planner
        ->
    Coder
        ->
    Tester

    if Tester.pass
        deploy
```

### Project Directory

```
project/
    main.lk
    agents/
        coder.lk
        reviewer.lk
        planner.lk
    tasks/
        build.lk
        deploy.lk
    memory/
        permissions.lk
```

---

# Specifications

Lucky is specified across five technical documents:

| Document | Description |
|---|---|
| [Language Reference Manual](docs/Lucky%20Language%20Reference%20Manual%20V0.1.md) | Syntax, types, expressions, statements, AI programming model, standard library |
| [Runtime Specification](docs/Lucky%20Runtime%20Specification%20V0.1.md) | Scheduler, memory model, concurrency, checkpoints, permissions, security |
| [Standard Library](docs/Lucky%20Standard%20Library%20Specification%20V0.1.md) | Built-in types, collections, AI primitives, tools, agents, utility modules |
| [IR Specification](docs/Lucky%20IR%20Specification%20V0.1.md) | SSA-based execution graph, optimization passes, serialization, backend API |
| [Tool Protocol (LTP)](docs/Lucky%20Tool%20Protocol%20Specification%20V0.1.md) | JSON-RPC protocol for cross-platform execution (Claude Code, Codex CLI, etc.) |

---

# Roadmap V0.1

| Phase | Goal | Deliverable | Status |
| ----- | ---- | ----------- | ------ |
| 1 | Design philosophy | Core concepts, semantic hierarchy, design principles | Done |
| 2 | Language specification | Syntax, grammar, semantics, type system, execution model | Done |
| 3 | Runtime specification | Scheduler, memory, concurrency, checkpoints, permissions, security | Done |
| 4 | Standard library | Built-in types, collections, AI primitives, tools, agents, APIs | Done |
| 5 | IR specification | SSA-based execution graph, optimization passes, serialization | Done |
| 6 | Tool Protocol (LTP) | JSON-RPC protocol for cross-platform AI execution | Done |
| 7 | Parser & AST | Lexer, parser, AST, diagnostics | Done |
| 8 | Compiler & IR | HIR/MIR/LIR lowering pipeline, optimization passes | Done |
| 9 | Runtime engine | Task scheduler, context propagation, memory, permissions, tool execution | Done |
| 10 | AI integrations | Adapters for Claude Code, Codex CLI, OpenCode, Cursor, Dify | Done |
| 11 | Ecosystem | Package manager, debugger, LSP, formatter, testing framework, VS Code extension | Done |

