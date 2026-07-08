<div align="center">
  <img src="logo/logo256.png">
  <h1>Lucky: a Goal-Oriented Agent Language</h1>
  <p><em>Write the orchestration. Let agents do the work.</em></p>
</div>

Lucky is a **goal-oriented orchestration language for AI agents**. You describe multi-agent workflows — review code, run tests, deploy, audit — and Lucky compiles them into a portable execution graph that runs inside your agent platform of choice: Claude Code, Codex CLI, OpenCode, Cursor, WorkBuddy, Windsurf, or any LTP-compatible tool.

> **📖 New to Lucky?** Start with the [Quickstart Guide](docs/quickstart.md) or follow the [Tutorial](docs/tutorial.md).

---

## How It Works

```lucky
# review-workflow.lk — one file describes the whole pipeline
project CodeReview

use DeepSeek

agent Reviewer
  model DeepSeek
  tools Git, Filesystem
  permissions allow git.diff, allow git.comment

agent Tester
  model DeepSeek
  tools Shell
  permissions allow shell.exec("cargo test")

agent Deployer
  model DeepSeek
  tools Git, Shell
  permissions allow git.push(staging/*), deny git.push(main)

workflow PRCheck
  CloneRepo -> parallel Review Diff, Run Tests -> wait ->
  if passed then Deploy to Staging else Post Failure Comment

goal AutoReview
  success reviewed and tested and deployed
  workflow PRCheck
```

**Write once, run on any agent platform:**

```bash
# Standalone (for testing/development)
lucky run review-workflow.lk

# Via LTP — your agent tool talks to the Lucky runtime
lucky serve --port 9700          # start the runtime server
# Now Claude Code / Codex / OpenCode / Cursor can submit IR

# Via MCP (for Claude Desktop, Windsurf, Cline)
lucky serve --mcp                 # Lucky as a Model Context Protocol server
```

**The agent platform executes every step.** Lucky defines *what* should happen; the platform (Claude Code, Codex, etc.) does *how*. Approvals, tool calls, LLM reasoning — all happen inside the agent you're already using.

---

## Quick Start

```bash
# Install the Lucky compiler and runtime
cargo install lucky-compiler

# Create a project
lucky init my-pipeline
cd my-pipeline

# Write a workflow (see examples/)
# Compile and check for errors
lucky check main.lk

# Run standalone (stub responses, no API key needed)
lucky run main.lk

# Run with real LLM backend
export DEEPSEEK_API_KEY="sk-..."
lucky run main.lk

# Run on a platform via LTP
lucky serve --port 9700 &
# Now configure your agent tool to connect to localhost:9700
```

---

## Where Lucky Runs

Lucky workflows execute inside **your existing agent tools**. The Lucky compiler produces platform-neutral IR; the Lucky Tool Protocol (LTP) delivers it to the runtime embedded in each platform.

| Platform | Integration | Status |
|---|---|---|
| [Claude Code](https://docs.anthropic.com/en/docs/claude-code) | MCP / LTP adapter | ✅ v0.1 |
| [Codex CLI](https://github.com/openai/codex) | YAML agent config + Python executor | ✅ v0.1 |
| [OpenCode](https://github.com/sampotts/opencode) | Skill definition + run scripts | ✅ v0.1 |
| [Cursor](https://cursor.sh) | VS Code extension | ✅ v0.1 |
| [Dify](https://dify.ai) | Tool YAML + Python provider | ✅ v0.1 |
| [WorkBuddy](https://workbuddy.ai) | Plugin adapter | 🔜 v0.3 |
| [Windsurf](https://codeium.com/windsurf) / Cline | MCP adapter | 🔜 v0.3 |

---

## Language Tour

### Agents

```lucky
agent SecurityAuditor
  model DeepSeek
  tools Git, Filesystem
  memory AuditMemory
  permissions
    allow filesystem.read
    allow git.diff
    deny shell.exec
```

### Tasks

```lucky
task ReviewCode
  input diff: String
  output approved: Bool
  steps
    let findings = SecurityAuditor.analyze(diff)
    return findings.is_empty()
```

### Workflows

```lucky
workflow DeployPipeline
  parallel
    ReviewCode
    RunTests
  wait
    -> if approved and passed then Deploy else Notify
```

### Error Recovery

```lucky
attempt
  deploy
recover
  retry 3 with backoff exponential(max: 5m)
  fallback RollbackDeploy
  escalate human
```

### Human Approval

```lucky
approval
  before deploy to production
```

---

## Key Capabilities

| Capability | Description |
|---|---|
| **Multi-Agent Orchestration** | Compose agents into DAG workflows with sequence, parallel, branch, swarm |
| **Context Auto-Propagation** | Context flows through the workflow graph — no manual parameter threading |
| **Capability Security** | `allow`/`deny` per agent, lexical inheritance, restrict-only semantics |
| **Declarative Recovery** | `attempt`/`recover` with retry, fallback, escalation, circuit breakers |
| **Human Approval Gates** | `approval` blocks pause execution for sign-off on critical operations |
| **Portable IR** | The same `.lir` file runs on Claude Code, Codex CLI, OpenCode, Cursor, Dify |
| **LLM Backends** | DeepSeek, OpenAI, Anthropic, Ollama — model config in `lucky.toml` |
| **Checkpoint & Resume** | Snapshot execution state — resume from any point |
| **Cost Budgets** | `--budget USD` enforces cost limits across all LLM calls |
| **Audit Trails** | Every decision, tool call, and approval logged to structured JSONL |

---

## Comparison

| | **Prompts** | **SKILL Files** | **Lucky** |
|---|---|---|---|
| **What** | Ad-hoc instructions for a single LLM call | Reusable markdown with structured agent instructions | Compiled orchestration language with types, graphs, permissions |
| **Best for** | One-shot questions | Repeatable single-agent capabilities | Multi-agent, multi-step production workflows |
| **Orchestration** | Manual chaining | Linear sequences | DAG workflows with parallel, branch, recovery |
| **Permissions** | None | Limited | First-class `allow`/`deny`, lexical inheritance |
| **Audit** | None | None | Built-in structured audit trail |
| **Portability** | Tied to LLM | Tied to platform | Platform-neutral via LTP |

---

## Learn More

| Resource | Description |
|---|---|
| [Quickstart Guide](docs/quickstart.md) | Get running in 5 minutes |
| [Tutorial](docs/tutorial.md) | 15 chapters from hello world to production patterns |
| [Language Reference Manual](docs/spec/Lucky%20Language%20Reference%20Manual%20V0.1.md) | Full language spec |
| [Runtime Specification](docs/spec/Lucky%20Runtime%20Specification%20V0.1.md) | Execution engine, scheduler, memory, security |
| [Standard Library](docs/spec/Lucky%20Standard%20Library%20Specification%20V0.1.md) | Built-in types, AI primitives, tools |
| [IR Specification](docs/spec/Lucky%20IR%20Specification%20V0.1.md) | SSA execution graph, opcodes, optimization |
| [Tool Protocol (LTP)](docs/spec/Lucky%20Tool%20Protocol%20Specification%20V0.1.md) | JSON-RPC protocol for cross-platform execution |
| [Scenarios Diagram](docs/spec/Lucky%20Scenarios%20Diagram.html) | Visual overview of when to use Lucky |
| [Roadmap](ROADMAP.md) | v0.1 → v0.2 → v0.3 plans |
| [Examples](lucky-compiler/examples/) | CI/CD bot, research assistant, security audit, ETL pipeline |

---

## Philosophy

> *"Think in goals, not syntax."*

Lucky fills the missing layer between natural language and executable software:

```
Natural Language
      ↑
    Lucky
      ↑
    Python / Go
      ↑
    Rust / C++
```

You don't write Lucky programs and run them standalone. You **describe** workflows in Lucky, and your **agent platform executes them**. Lucky is the orchestration layer — the bridge between what you want and what agents do.

---

<div align="center">
  <sub>Built with ❤️ by <a href="https://github.com/jfxia">Jingfeng Xia</a></sub>
</div>
