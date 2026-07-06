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

The `lucky.toml` file is where you configure LLM backends and runtime settings:

```toml
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

View your resolved configuration anytime with `lucky config`.

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
# Basic run (stub responses)
lucky run main.lk

# Run with real LLM backend (set API key first)
$env:DEEPSEEK_API_KEY="sk-xxx"    # Windows
export DEEPSEEK_API_KEY="sk-xxx"  # Linux/macOS
lucky run main.lk

# Stream tokens as they arrive
lucky run main.lk --stream

# Run with a cost budget
lucky run main.lk --budget 5.00

# Log every step to an audit file
lucky run main.lk --audit execution.jsonl
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

This outputs the HIR and MIR JSON representations with SSA basic blocks, proper instructions (Alloca, Store, AgentInvoke, LlmComplete, ToolInvoke), and control flow terminators — useful for inspection and debugging.

---

## 8. Write a Test

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
- Use `lucky watch . --run` to auto-recheck files on change
- Use `lucky doc . -o docs/api` to generate Markdown documentation from your `.lk` files

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

Lucky has first-class model support. Declare models at the language level, set API keys in the environment, and go:

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

Three backends are supported out of the box:

| Backend | API Key | Endpoint |
|---------|---------|----------|
| DeepSeek | `DEEPSEEK_API_KEY` | `api.deepseek.com` |
| OpenAI | `OPENAI_API_KEY` | `api.openai.com` |
| Ollama | *(none)* | `localhost:11434` |

---

## Getting Help

```bash
lucky --help           # All commands
lucky check --help     # Command-specific help
```

Report issues at: https://github.com/jfxia/lucky/issues
