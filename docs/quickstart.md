# Lucky Quickstart Guide

Get up and running with Lucky in 5 minutes.

---

## 1. Install

Lucky is a single binary. Download or build from source:

```bash
# Clone and build
git clone https://github.com/jfxia/lucky.git
cd lucky/lucky-compiler
cargo build --release

# Add to PATH (or copy the binary)
# On Linux/macOS:
cp target/release/lucky /usr/local/bin/
# On Windows:
copy target\release\lucky.exe C:\tools\
```

Verify the installation:

```bash
lucky --help
```

---

## 2. Create a Project

```bash
lucky init hello-world
cd hello-world
```

This creates:

```
hello-world/
├── lucky.toml          # Project manifest
├── main.lk             # Entry point
├── agents/             # Agent definitions
├── tasks/              # Task definitions
└── memory/             # Memory configurations
```

---

## 3. Your First Program

Open `main.lk` and write:

```lucky
project HelloWorld

use Claude

agent Greeter
    model Claude

task SayHello
    input
        name: String
    output
        greeting: String
    steps
        let greeting = "Hello, " + name
        return greeting

goal Greet
    success
        greeting_produced
    workflow SayHelloWorkflow

workflow SayHelloWorkflow
    Greeter.SayHello(name = "Lucky")
```

---

## 4. Run It

```bash
lucky run main.lk
```

Output:
```
=== Lucky Runtime Execution ===
Nodes: 3, Edges: 2

  START  [0] Goal:Greet
  DONE   [0] Goal:Greet
  START  [1] Task:SayHello
  DONE   [1] Task:SayHello

  === Execution success ===
Execution Completed: 3/3 completed | $0.000 | 3 steps
```

---

## 5. Check Syntax

```bash
lucky check main.lk
# No errors found in 'main.lk'.
```

---

## 6. Format Code

```bash
lucky fmt main.lk
# Formatted 'main.lk'.
```

---

## 7. Compile to IR

```bash
lucky ir main.lk
```

This outputs the HIR and MIR JSON representations. The MIR now contains real SSA basic blocks with proper instructions (Alloca, Store, AgentInvoke, LlmComplete, ToolInvoke) and control flow terminators (Br, CondBr, Ret).

---

## 8. Configure LLM Backends

Lucky v0.2 supports real LLM API calls. Set up your API keys:

```toml
# lucky.toml
[project]
name = "hello-world"
version = "0.1.0"

[models.deepseek-v4]
provider = "deepseek"

[models.gpt-4o]
provider = "openai"

[runtime]
budget_usd = 10.0
```

```bash
# Set API keys
export DEEPSEEK_API_KEY="sk-xxx"    # or: $env:DEEPSEEK_API_KEY="sk-xxx" on Windows
export OPENAI_API_KEY="sk-xxx"

# View resolved config
lucky config

# Run with real LLM backend
lucky run main.lk

# Run with streaming output
lucky run main.lk --stream

# Run with cost budget
lucky run main.lk --budget 5.00

# Run with audit trail
lucky run main.lk --audit execution.jsonl
```

---

## 9. Write a Test

Create `hello.test.lk`:

```lucky
test "greeting contains name" {
    let greeting = "Hello, Lucky"
    assert greeting contains "Lucky"
    assert greeting starts_with "Hello"
}

test "greeting is not empty" {
    let greeting = "Hello, World"
    assert greeting != ""
}
```

Run tests:

```bash
lucky test .
```

```
=== Lucky Test Runner ===
  PASS  greeting contains name
  PASS  greeting is not empty

Results: 2 passed, 0 failed, 0 skipped
```

---

## 10. Key Concepts at a Glance

| Concept | What It Is | Example |
|---|---|---|
| **Goal** | What success means | `goal Deploy { success service.online }` |
| **Workflow** | How to achieve a goal | `workflow CI { Build -> Test -> Deploy }` |
| **Agent** | Who does the work | `agent Reviewer { model Claude; tools Git }` |
| **Task** | A unit of work | `task Analyze { input repo; output report; steps ... }` |
| **Tool** | An external capability | `tool Browser; Browser.search("query")` |
| **Context** | Ambient state | `context { user, repo, session }` |
| **Memory** | Persistent state | `memory ProjectMemory; remember("key", val)` |
| **Pipeline** | Data flow | `data \|> filter \|> transform \|> save` |
| **Prompt** | Structured AI prompt | `prompt Reviewer { role ...; rules ... }` |

---

## 10. Next Steps

- Read the [Tutorial](tutorial.md) for a step-by-step walkthrough
- Read the [Language Reference Manual](Lucky%20Language%20Reference%20Manual%20V0.1.md) for complete syntax
- Read the [Standard Library](Lucky%20Standard%20Library%20Specification%20V0.1.md) for API reference
- Explore the [examples/](../lucky-compiler/examples/) directory

---

## 11. v0.2 CLI Commands

```bash
# Watch for file changes and auto re-check
lucky watch . --run

# Generate documentation from .lk files
lucky doc . -o docs/api

# Show resolved configuration
lucky config

# Run with production runtime features
lucky run main.lk --budget 5.00 --stream --audit audit.jsonl
lucky run main.lk --auto-approve --approve "before deploy"
```

---

## Built-in Tools

Lucky ships with these tools ready to use:

| Tool | Purpose | Example |
|---|---|---|
| `Filesystem` | Read/write files | `Filesystem.read("config.json")` |
| `Shell` | Run commands | `Shell.exec("cargo build")` |
| `Git` | Version control | `Git.clone(repo_url)` |
| `HTTP` | Web requests | `HTTP.get("/api/users")` |
| `Browser` | Web automation | `Browser.navigate(url)` |
| `Search` | Web search | `Search.search("AI agents")` |

---

## AI Models

Lucky v0.2 supports three LLM backends with real API calls (no stubs). Declare models at the language level and set API keys in the environment:

```lucky
model DeepSeek(
    provider = "deepseek",
    version = "deepseek-v4",
    temperature = 0.3,
)

model GPT(
    provider = "openai",
    version = "gpt-4o",
)

model LocalLLM(
    provider = "ollama",
    version = "llama3",
)

use DeepSeek         # Set default

agent Researcher
    use GPT          # Override for this agent
```

**API Keys** (set via environment variables):
- DeepSeek: `DEEPSEEK_API_KEY=sk-xxx`
- OpenAI: `OPENAI_API_KEY=sk-xxx`
- Ollama: No key needed (runs on `localhost:11434`)

```bash
# Run with real LLM
DEEPSEEK_API_KEY=sk-xxx lucky run main.lk

# Stream tokens as they arrive
lucky run main.lk --stream
```

---

## Getting Help

```bash
lucky --help           # All commands
lucky check --help     # Command-specific help
```

Report issues at: https://github.com/jfxia/lucky/issues
