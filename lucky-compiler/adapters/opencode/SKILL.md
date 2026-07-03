---
name: lucky-executor
description: Execute Lucky programs via the Lucky Tool Protocol. Use this skill when working with Lucky workflows (.lk files), running Lucky agent pipelines, or executing Lucky IR programs.
tools:
  - name: lucky_run
    description: Run a Lucky program using a pre-compiled IR file.
    parameters:
      ir_path:
        type: string
        description: Path to the .lir IR JSON file.
        required: true
      goal:
        type: string
        description: Goal name to pursue (entry point into the program).
        required: false
      context:
        type: object
        description: Execution context dictionary (e.g. user, repo, branch).
        required: false
  - name: lucky_status
    description: Check the status of a running Lucky execution.
    parameters:
      exec_id:
        type: string
        description: Execution ID returned by lucky_run.
        required: true
  - name: lucky_approve
    description: Respond to a Lucky human-approval request.
    parameters:
      approval_id:
        type: string
        description: Approval request ID.
        required: true
      decision:
        type: string
        description: Decision — approve, reject, or modify.
        required: true
      reason:
        type: string
        description: Reason for the decision.
        required: false
---

# Lucky Executor

This skill allows OpenCode to execute Lucky programs through the Lucky Tool Protocol (LTP).

## When to Use

Use this skill whenever the user mentions or the task involves:

- **Lucky** programs, workflows, or pipelines
- **.lk** source files or **.lir** IR files
- Lucky **agents** or multi-agent orchestration
- Running a Lucky **workflow** or **goal**
- Approving a Lucky **deployment gate** or human-in-the-loop step
- Checking the **status** of a running Lucky execution

Trigger words: Lucky, .lk, .lir, workflow, agent, goal, Lucky IR, LTP.

## How It Works

1. The user provides a path to a Lucky IR file (`.lir`), a goal name, and optionally a context dictionary.
2. Call `lucky_run` to compile (if needed) and execute the program against the Lucky runtime.
3. If the execution pauses for human approval, use `lucky_approve` to respond.
4. Use `lucky_status` to poll an async execution's progress.

## Execution Flow

```
User request
    ↓
lucky_run(ir_path, goal, context)
    ↓
LTP client → Lucky runtime → executes the Lucky IR DAG
    ↓
Result: { result, cost, outputs, duration_ms, execution_id }
```

## Server Configuration

By default, `run.py` spawns a Lucky runtime via stdio (`lucky serve --transport stdio`).
Set the environment variable `LUCKY_SERVER_URL` to use an existing HTTP server instead.

```bash
export LUCKY_SERVER_URL="http://localhost:9700"
export LUCKY_SERVER_TOKEN="ltp-token-xyz"
```
