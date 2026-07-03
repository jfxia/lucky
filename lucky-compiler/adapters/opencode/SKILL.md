---
name: lucky-executor
description: Write, check, run, format, and compile Lucky programs (.lk files). Use this skill for all Lucky language operations — creating agents, workflows, goals, tasks, and executing Lucky programs.
tools:
  - name: lucky_run
    description: Run a Lucky program from a .lk source file or inline source code.
    parameters:
      file:
        type: string
        description: Path to the .lk source file.
        required: false
      source:
        type: string
        description: Inline Lucky source code to compile and run.
        required: false
      goal:
        type: string
        description: Goal name to pursue (entry point).
        required: false

  - name: lucky_check
    description: Check a Lucky program for syntax errors.
    parameters:
      file:
        type: string
        description: Path to the .lk source file.
        required: false
      source:
        type: string
        description: Inline Lucky source code to check.
        required: false

  - name: lucky_ir
    description: Compile a Lucky program to IR (HIR and MIR JSON output).
    parameters:
      file:
        type: string
        description: Path to the .lk source file.
        required: false
      source:
        type: string
        description: Inline Lucky source code to compile.
        required: false

  - name: lucky_format
    description: Format a Lucky source file.
    parameters:
      file:
        type: string
        description: Path to the .lk source file to format.
        required: false
      source:
        type: string
        description: Inline Lucky source code to format.
        required: false

  - name: lucky_init
    description: Initialize a new Lucky project with scaffolding.
    parameters:
      path:
        type: string
        description: Directory path for the new project.
        required: true
---

# Lucky Executor

This skill enables full Lucky language support in OpenCode: writing, checking, running, formatting, compiling, and initializing Lucky programs.

## When to Use

Use this skill whenever the user mentions or the task involves:

- **Lucky** programs, workflows, or pipelines
- **.lk** source files
- Lucky **agents** or multi-agent orchestration
- Running a Lucky **workflow** or **goal**
- Creating **Lucky projects** (`lucky init`)
- **Compiling** Lucky to IR
- **Formatting** Lucky code
- **Checking** Lucky syntax

Trigger words: Lucky, .lk, workflow, agent, goal, task, Lucky language.

## Tools

### lucky_run — Run a Lucky program

```python
lucky_run(file="main.lk")                    # Run from file
lucky_run(source="project X\nuse DeepSeek\ngoal G ...")  # Run inline source
```

### lucky_check — Check for errors

```python
lucky_check(file="main.lk")
lucky_check(source="task T { steps ...")
```

### lucky_ir — Compile to IR

```python
lucky_ir(file="main.lk")    # Returns HIR + MIR JSON
```

### lucky_format — Format source

```python
lucky_format(file="main.lk")
lucky_format(source="task  T   {  steps ...")  # Returns formatted source
```

### lucky_init — Create project

```python
lucky_init(path="./my-lucky-project")
```

## Configuration

Set `LUCKY_BIN` environment variable to point to the Lucky CLI binary:

```bash
# Default: looks for 'lucky' on PATH
export LUCKY_BIN="/path/to/lucky"
```
