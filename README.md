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

### When to Use Lucky

Lucky is designed for a specific class of problems — ones where the "business logic" isn't a computation but a conversation with AI. You write Lucky when:

**You're orchestrating multiple AI agents.** A single LLM call can answer a question. But a real software project needs code review, security audit, test generation, and documentation — each done by a specialized agent, coordinated in sequence or parallel. Lucky makes that orchestration the language itself, not a library bolted onto Python.

**You need auditability, not just automation.** Lucky logs every decision, every tool call, and every approval to a structured audit trail. You can checkpoint execution state and resume from any point. Cost budgets are enforced at the language runtime level. When something goes wrong, you know exactly which agent did what, when, and why.

**You want guardrails, not just prompts.** Agents run with explicit `allow`/`deny` permissions. Critical operations require human approval — not as an afterthought, but as a language-level construct. Recovery is declarative: retry with exponential backoff, fallback to another agent, escalate to a human. No try/catch spaghetti.

**Your workflow is valuable IP independent of any single LLM provider.** Model declarations live in `lucky.toml`, not hardcoded in scripts. Switch from DeepSeek to OpenAI to a local Ollama model without touching your orchestration logic. The IR is portable across execution platforms.

Lucky is **not** for:
- Crunching numbers or building web servers (use Rust, Go, Python)
- Real-time systems or embedded devices
- Single-shot LLM queries ("summarize this article")

Lucky **is** for:
- CI/CD bots that review, test, and deploy code
- Research pipelines that investigate, analyze, and report
- Document generation workflows with multi-stage review
- Security auditing across multiple dimensions
- Any multi-step process where AI agents coordinate under human oversight

---

### Lucky vs. Prompts vs. SKILLS

There are three levels of abstraction for working with AI agents, and Lucky occupies the highest layer:

| | **Natural Language Prompts** | **SKILL Files** | **Lucky Language** |
|---|---|---|---|
| **What it is** | Ad-hoc instructions written for a single LLM call | Reusable Markdown files with structured instructions (XML-like tags, metadata, workflow steps) | A compiled programming language with goals, tasks, agents, workflows, and IR |
| **Structure** | Free-form text | Semi-structured sections (instruction, environment, demo, reminder) | Formal grammar with AST, HIR, MIR — compiled to a portable SSA-based IR |
| **Reusability** | None — each prompt is written from scratch | Moderate — SKILL files can be shared and installed, but composition is manual | Full — tasks, agents, workflows, and policies are named, typed, and composable |
| **Orchestration** | Single-shot or manual chaining | Linear step sequences within a single skill file | Directed acyclic graphs with parallel branches, conditionals, loops, and recovery chains |
| **Determinism** | None — same prompt can produce different results | Low — relies on LLM following SKILL instructions | High — the execution graph is built at compile time; runtime follows the planned DAG |
| **State management** | Stateless — context must be manually passed | Implicit — relies on conversation state within the agent | Explicit — context auto-propagates through the workflow DAG; memory persists across executions |
| **Error recovery** | Manual — "try again" or rewrite the prompt | Manual — the skill can suggest recovery steps | Declarative — `attempt`/`recover` chains with retry, fallback, escalation, and circuit breakers |
| **Permissions** | None — the LLM has whatever access it was given | Limited — can specify required tools/capabilities | First-class — `allow`/`deny` per agent, lexical inheritance, human approval gates |
| **Audit trail** | None | None built-in | Built-in — every decision, tool call, and approval is logged to a structured audit file |
| **Portability** | Tied to the LLM | Tied to the agent platform (OpenCode, Claude Code) | Platform-neutral — Lucky IR runs on any LTP-compatible runtime |
| **Best for** | Simple questions, single-step tasks | Reusable agent capabilities (code review, research, testing) | Multi-agent orchestration, production workflows, compliance-critical systems |

Think of the hierarchy this way:

```
Natural Language Prompts
    ↓  (add structure)
SKILL Files
    ↓  (add compilation, types, state, permissions, recovery)
Lucky Language
```

Prompts are the assembly language of AI — flexible but untyped, unreusable, and unverifiable. SKILLs add reusable structure for individual agent capabilities. Lucky adds full programming language semantics for **orchestrating multiple agents** — the OS for AI, not a script for one.

> **When to use each:** Write a prompt for a one-off question. Install a SKILL when you want a repeatable agent capability (e.g., "review my code"). Write Lucky when you need to orchestrate multiple agents across a multi-step workflow with permissions, error recovery, and audit trails — the kind of thing that would otherwise require a Python script bolted onto an LLM SDK.

---

### Key Capabilities

**Multi-Agent Orchestration.** Define agents with models, tools, memory, and permissions. Compose them into workflows with `->` for sequential chains, or let them run in parallel at the same indentation level. Use `parallel`/`wait` for fork-join, `swarm` for mass fan-out, `if`/`else` for branching.

**AI-Native Language Primitives.** Declare models (`model DeepSeek(...)`), switch them (`use GPT`), write structured prompts (`prompt Reviewer { role ...; rules ... }`), and call LLMs inline (`ask DeepSeek: summarize this`). No SDKs, no API wrappers.

**Context Auto-Propagation.** Declare workflow-level context once. It flows automatically to every agent and task in the chain. Each task's output becomes context for downstream tasks. No manual parameter threading.

**Capability Security.** Every agent runs with explicit `allow`/`deny` permissions. Permissions inherit lexically and can only be restricted, never expanded. Built-in approval gates pause execution for human sign-off on critical operations.

**Declarative Error Recovery.** Replace try/catch with `attempt`/`recover` chains: retry with exponential backoff and jitter, fallback to alternative agents, escalate to human operators. A circuit breaker prevents retry storms (5 failures in 60 seconds stops retrying).

**Portable IR.** Lucky compiles to a language-neutral Intermediate Representation — an SSA-based DAG of execution nodes with 30+ opcodes, proper basic blocks, and control flow terminators. The same IR runs across Claude Code, Codex CLI, OpenCode, Cursor, and Dify via the Lucky Tool Protocol (LTP).

**Real LLM Backends.** Declare models in `lucky.toml` and call them at the language level. Zero-dependency adapters for DeepSeek, OpenAI, and Ollama use raw `TcpStream` with manual HTTP/1.1 and custom TLS 1.2. Set `DEEPSEEK_API_KEY` or `OPENAI_API_KEY` and go. Streaming token output via `--stream`.

**Developer Toolchain.** CLI with 16 commands (init, check, compile, fmt, ir, run, test, debug, pkg, serve, lsp, watch, doc, config), VS Code extension with syntax highlighting and snippets, LSP server with context-aware completions and real-time diagnostics, DAP debugger, formatter, test framework, and package manager. Rich ANSI-colored error messages with source context and fix suggestions. `lucky watch` auto-rechecks on file changes; `lucky doc` generates Markdown documentation from source.

**Production Runtime.** Checkpoint and resume execution state from disk. Track costs with budget enforcement (`--budget`). Log every step to JSONL audit trails (`--audit`). Interactive human approval gates with `--approve` and `--auto-approve` flags.


---

# Philosophy


> *"Think in goals, not syntax."*

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
| [IR Specification](docs/Lucky%20IR%20Specification%20V0.1.md) | SSA-based execution graph, 30+ opcodes, optimization passes, serialization |
| [Tool Protocol (LTP)](docs/Lucky%20Tool%20Protocol%20Specification%20V0.1.md) | JSON-RPC protocol for cross-platform execution (Claude Code, Codex CLI, etc.) |

*See [ROADMAP.md](ROADMAP.md) for the full v0.1 and v0.2 achievements and v0.3 plans.*

