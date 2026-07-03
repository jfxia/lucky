# Using Lucky Language in OpenCode

This guide shows how to write, check, run, and compile Lucky programs directly within OpenCode.

---

## Summary of Integration

The Lucky OpenCode adapter was installed into the local OpenCode skills directory with the following components:

| Component | Location | Purpose |
|---|---|---|
| **SKILL.md** | `~/.config/opencode/skills/lucky-executor/` | Skill definition registering 5 tools with OpenCode |
| **run.py** | `~/.config/opencode/skills/lucky-executor/` | Python executor that calls the Lucky CLI binary |
| **ltp_client.py** | `~/.config/opencode/skills/lucky-executor/` | LTP client for future server-based execution |

### Tools Registered

| Tool | CLI Equivalent | Description |
|---|---|---|
| `lucky_run` | `lucky run` | Execute a Lucky program from file or inline source |
| `lucky_check` | `lucky check` | Check syntax and report errors |
| `lucky_ir` | `lucky ir` | Compile to IR (HIR + MIR JSON) |
| `lucky_format` | `lucky fmt` | Format Lucky source code |
| `lucky_init` | `lucky init` | Create a new Lucky project scaffold |

### Trigger Words

OpenCode automatically activates this skill when you mention:
- **Lucky**, **.lk files**, **workflow**, **agent**, **goal**, **task**, **Lucky language**, **Lucky IR**

---

## Quickstart

### Prerequisites

1. The Lucky CLI binary must be built and accessible:

```bash
cd lucky-compiler
cargo build --release
```

2. Set the `LUCKY_BIN` environment variable (or ensure `lucky` is on PATH):

```bash
# Windows PowerShell
$env:LUCKY_BIN = "D:\test\lucky\lucky-compiler\target\release\lucky.exe"

# Linux/macOS
export LUCKY_BIN="/path/to/lucky-compiler/target/release/lucky"
```

3. Verify it works:

```bash
$env:LUCKY_BIN --help
```

---

### Step 1: Create a Lucky Project

In OpenCode, type:

```
Please create a new Lucky project called "my-agent" using lucky_init
```

Or run directly:

```bash
python ~/.config/opencode/skills/lucky-executor/run.py init ./my-agent
```

This creates:

```
my-agent/
├── lucky.toml
├── main.lk
├── agents/
├── tasks/
└── memory/
```

---

### Step 2: Write a Lucky Program

Open `my-agent/main.lk` and write a Lucky program. Example — a simple research assistant:

```lucky
project MyAgent

use DeepSeek

agent Researcher
    model DeepSeek(
        provider = "deepseek",
        version = "deepseek-v4",
    )
    tools
        Search, Browser

task Investigate
    input
        topic: String
    output
        report: String
    steps
        let results = Search.search(topic, max_results = 5)
        let summary = "Research on " + topic + " complete"
        return summary

goal ResearchGoal
    success
        report_generated
    workflow MainWorkflow

workflow MainWorkflow
    Researcher.Investigate(topic = "AI agent frameworks")
```

---

### Step 3: Check for Errors

In OpenCode:

```
Please check my-agent/main.lk for syntax errors using lucky_check
```

Or:

```bash
python ~/.config/opencode/skills/lucky-executor/run.py check ./my-agent/main.lk
```

Output when clean:
```json
{ "valid": true, "errors": [], "warnings": [] }
```

Output with errors:
```json
{
  "valid": false,
  "errors": [
    "my-agent/main.lk: error Expected indented block but found ''"
  ]
}
```

---

### Step 4: Format the Code

```
Please format my-agent/main.lk using lucky_format
```

Or:

```bash
python ~/.config/opencode/skills/lucky-executor/run.py fmt ./my-agent/main.lk
```

---

### Step 5: Run the Program

```
Please run my-agent/main.lk using lucky_run
```

Or:

```bash
python ~/.config/opencode/skills/lucky-executor/run.py run ./my-agent/main.lk
```

Output:
```json
{
  "execution_id": "5caa9c90",
  "status": "completed",
  "result": "success",
  "output": "",
  "stderr": "=== Lucky Runtime Execution ===\nNodes: 2, Edges: 0\n\n=== Execution Events ===\n  START  [0] ...\n  === Execution success ==="
}
```

---

### Step 6: Compile to IR

```
Please compile my-agent/main.lk to IR using lucky_ir
```

Or:

```bash
python ~/.config/opencode/skills/lucky-executor/run.py ir ./my-agent/main.lk
```

This produces HIR and MIR JSON — useful for debugging and integration with other tools.

---

## Using Inline Source

You don't need to create a file first. You can pass Lucky source code directly:

```
Please run this Lucky program using lucky_run:

project Inline
use DeepSeek

task Hello
    steps
        return "Hello from OpenCode!"

goal Main
    success ok
    workflow MainWorkflow

workflow MainWorkflow
    Hello
```

OpenCode will call `lucky_run(source="...")` which writes the source to a temp file and executes it.

---

## Full Conversation Example

```
User: I need to create a Lucky workflow that reviews pull requests.

OpenCode: Let me help you with that. First, I'll create a new Lucky project:
[lucky_init(path="./pr-reviewer")]

Now let me write the Lucky program:
[lucky_check(source="project PRReviewer\nuse DeepSeek\n...")]

The syntax checks out. Let me run it:
[lucky_run(file="./pr-reviewer/main.lk")]

Execution completed successfully. The workflow ran 3 agents in parallel
(SecurityReviewer, StyleReviewer, PerformanceReviewer) and produced a report.
```

---

## Configuration Reference

| Env Variable | Default | Description |
|---|---|---|
| `LUCKY_BIN` | `lucky` | Path to the Lucky CLI binary |

---

## Troubleshooting

| Problem | Solution |
|---|---|
| `FileNotFoundError: lucky` | Set `LUCKY_BIN` to the full path of `lucky.exe` |
| `valid: false` after check | Read the `errors` array for line numbers and fix the source |
| `status: failed` after run | Check the `error` field for the failure reason |
| Tool not appearing in OpenCode | Verify SKILL.md is in `~/.config/opencode/skills/lucky-executor/` |
| Python import errors | Ensure Python 3.10+ is installed and `python` is on PATH |

---

## Next Steps

- Read the [Lucky Tutorial](../tutorial.md) for language concepts
- Read the [Quickstart Guide](../quickstart.md) for CLI usage
- Explore [example programs](../../lucky-compiler/examples/) for real-world patterns
