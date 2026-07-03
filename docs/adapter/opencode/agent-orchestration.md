# Orchestrating AI Agents with Lucky in OpenCode

This guide shows how to define, compose, and run multi-agent pipelines using Lucky directly within OpenCode.

---

## What is Agent Orchestration?

Agent orchestration means coordinating multiple AI agents -- each with its own model, tools, memory, and permissions -- to accomplish a complex goal. Lucky makes this declarative: you describe **who** does **what** and **in what order**, and the runtime handles scheduling, context propagation, concurrency, and error recovery.

```
+------------+    +------------+    +------------+
| Researcher |--->|  Planner   |--->|   Coder    |
| DeepSeek   |    | DeepSeek   |    | DeepSeek   |
| Search     |    | Filesystem |    | Git,Shell  |
+------------+    +------------+    +------------+
     |                                     |
     +------- context flows -------------->+
```

---

## Quickstart: Build a 3-Agent Pipeline

We'll build a system with three agents that research a topic, plan an approach, and generate output. Everything happens inside OpenCode.

### Step 1: Create the Project

```
lucky_init(path="./agent-pipeline")
```

This creates `agent-pipeline/` with `main.lk` ready to edit.

### Step 2: Define the Agents

Each agent gets a model, tools, and a role. Open `main.lk`:

```lucky
project AgentPipeline

use DeepSeek

model DeepSeek(
    provider = "deepseek",
    version = "deepseek-v4",
    temperature = 0.3,
)

agent Researcher
    model DeepSeek
    tools
        Search, Browser
    permissions
        allow search.*, browser.*
        deny filesystem.*
    policy
        timeout 5m

agent Planner
    model DeepSeek
    tools
        Filesystem
    permissions
        allow filesystem.write
    policy
        timeout 3m

agent Writer
    model DeepSeek
    tools
        Filesystem
    permissions
        allow filesystem.write
    policy
        timeout 5m
```

### Step 3: Define Tasks

Tasks are the work units each agent performs:

```lucky
task GatherSources
    input
        topic: String
        max_results: Int
    output
        sources: List<String>
    steps
        let results = Search.search(topic, max_results = max_results)
        return results

task CreatePlan
    input
        topic: String
        research: List<String>
    output
        plan: String
    steps
        let plan = "Plan for: " + topic
        return plan

task GenerateReport
    input
        topic: String
        plan: String
    output
        report: String
    steps
        let report = "Report on: " + topic
        return report
```

### Step 4: Wire the Workflow

This is where orchestration happens. The `->` arrows define execution order. Without arrows, agents run in parallel:

```lucky
workflow ResearchPipeline
    context
        topic: String
        max_results: Int = 10

    GatherSources(
        topic = context.topic,
        max_results = context.max_results,
    )
        ->
    CreatePlan(
        topic = context.topic,
        research = context.sources,
    )
        ->
    GenerateReport(
        topic = context.topic,
        plan = context.plan,
    )
```

### Step 5: Define the Goal

```lucky
goal ProduceReport
    success
        sources_gathered
        plan_created
        report_generated
    workflow ResearchPipeline
```

### Step 6: Check and Run

```
# Check syntax
lucky_check(file="agent-pipeline/main.lk")
# -> { "valid": true }

# Run the pipeline
lucky_run(file="agent-pipeline/main.lk")
# -> { "status": "completed", "result": "success" }
```

---

## Orchestration Patterns

### Pattern 1: Sequential Chain (`->`)

Each agent waits for the previous one. Data flows through context automatically.

```lucky
workflow Sequential
    AgentA.task1()
        ->
    AgentB.task2()    # receives AgentA's output via context
        ->
    AgentC.task3()    # receives AgentB's output via context
```

### Pattern 2: Parallel Fan-Out

Agents at the same indentation level run simultaneously:

```lucky
workflow ParallelAudit
    # All three start at the same time
    SecurityAuditor.scan(repo)
    StyleChecker.lint(repo)
    PerformanceProfiler.analyze(repo)
    # Workflow completes when all three finish
```

### Pattern 3: Fan-Out -> Fan-In

Run parallel work, then aggregate results:

```lucky
workflow ReviewAndMerge
    FetchPR(pr_number = context.pr)
        ->
    parallel
        SecurityReviewer.review(diff)
        StyleReviewer.review(diff)
        PerfReviewer.review(diff)
    wait
        ->
    Aggregator.merge(findings)
        ->
    PostComment(pr_number = context.pr)
```

### Pattern 4: Conditional Branching

Route execution based on results:

```lucky
workflow ConditionalDeploy
    RunTests()
        ->
    if context.tests_passed
        Deployer.deploy()
    else
        Notifier.alert("Tests failed")
```

### Pattern 5: Swarm Execution

Run many copies of the same agent on different inputs:

```lucky
workflow BatchProcess
    FetchFiles(directory = "./data")
        ->
    swarm 20 Analyzer.process_file(files)
    # 20 instances run in parallel, each processing a different file
```

### Pattern 6: Attempt/Recover

Graceful error handling across agents:

```lucky
workflow ResilientPipeline
    attempt
        PrimaryAgent.run()
    recover
        retry 3 with backoff exponential(max: 5m)
    recover
        fallback BackupAgent.run()
    recover
        human escalate "Both primary and backup failed"
```

---

## Context Propagation

Context flows automatically from workflow -> agent -> task. No manual parameter passing:

```lucky
workflow BuildFeature
    context
        user: String
        repo: URI
        feature_spec: String

    Designer.design()
        ->
    Coder.implement()
        ->
    Tester.verify()
```

Within a task, access context directly:

```lucky
task ImplementFeature
    steps
        let user = context.user           # inherited from workflow
        let repo = context.repo           # inherited from workflow
        let design = context.design_output # set by Designer agent
        # ...
```

---

## Built-in Agents Reference

These standard agents ship with Lucky:

| Agent | Purpose | Tools |
|---|---|---|
| `Researcher` | Web research and synthesis | Search, Browser, HTTP |
| `Planner` | Task decomposition and planning | Filesystem |
| `Coder` | Code generation and refactoring | Filesystem, Git, Shell |
| `Reviewer` | Code and document review | Git, Filesystem |
| `Tester` | Test generation and execution | Filesystem, Shell, Git |
| `Architect` | System design | Search, Browser |
| `SecurityAuditor` | Vulnerability scanning | Filesystem, Git, Shell |
| `TechnicalWriter` | Documentation generation | Filesystem, Git |

---

## Complete Example: CI/CD Bot

Here's a complete orchestrator that reviews every PR:

```lucky
project CIBot

use DeepSeek

model DeepSeek(
    provider = "deepseek",
    version = "deepseek-v4",
)

agent Reviewer
    model DeepSeek
    tools Git, Filesystem

agent Tester
    model DeepSeek
    tools Shell, Git

agent Deployer
    model DeepSeek
    tools Git, Shell
    permissions
        allow git.push(staging/*)
        deny git.push(main)

workflow PRCheck
    context
        repo: URI
        pr_number: Int

    Git.clone(context.repo)
        ->
    parallel
        Reviewer.review(diff = Git.diff("main"))
        Tester.run(command = "cargo test")
    wait
        ->
    if context.review_approved and context.tests_passed
        Deployer.deploy(environment = "staging")
    else
        Git.comment(context.pr_number, "Fix issues before merge")

goal AutoReview
    success review_complete and tests_passed and deployed
    workflow PRCheck
```

---

## Running in OpenCode

Once your `.lk` file is ready, use the Lucky tools:

```
# Format the code
lucky_format(file="agent-pipeline/main.lk")

# Check for errors
lucky_check(file="agent-pipeline/main.lk")

# Run the orchestration
lucky_run(file="agent-pipeline/main.lk")
```

Or pass source inline:

```
lucky_run(source="project X
use DeepSeek

agent A
    model DeepSeek(...)

workflow W
    A.task1() -> A.task2()

goal G
    workflow W")
```

---

## Next Steps

- [Lucky Tutorial](../tutorial.md) -- Full language walkthrough
- [Language Reference](../Lucky%20Language%20Reference%20Manual%20V0.1.md) -- Complete syntax
- [Example Programs](../../lucky-compiler/examples/) -- Real-world patterns including CI/CD, research, ETL, security audit
