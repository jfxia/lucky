# Lucky Tutorial

A step-by-step guide to the Lucky language. By the end, you'll be able to write goal-oriented AI agent programs.

---

## Table of Contents

1. [Hello World](#1-hello-world)
2. [Tasks — The Building Blocks](#2-tasks--the-building-blocks)
3. [Agents — Intelligent Workers](#3-agents--intelligent-workers)
4. [Workflows — Orchestration](#4-workflows--orchestration)
5. [Goals — Defining Success](#5-goals--defining-success)
6. [Context — Ambient State](#6-context--ambient-state)
7. [Memory — Persistent Knowledge](#7-memory--persistent-knowledge)
8. [Tools — External Capabilities](#8-tools--external-capabilities)
9. [Pipelines — Data Flow](#9-pipelines--data-flow)
10. [AI Models & Prompts](#10-ai-models--prompts)
11. [Concurrency — Parallel & Swarm](#11-concurrency--parallel--swarm)
12. [Error Handling — Attempt & Recover](#12-error-handling--attempt--recover)
13. [Permissions & Security](#13-permissions--security)
14. [Human Approval](#14-human-approval)
15. [Patterns & Best Practices](#15-patterns--best-practices)

---

## 1. Hello World

### Project Setup

```bash
lucky init hello-world
cd hello-world
```

### Your First File

```lucky
project HelloWorld

use Claude

task SayHello
    steps
        let message = "Hello, Lucky!"
        return message

goal MainGoal
    success
        message_returned
    workflow MainWorkflow

workflow MainWorkflow
    SayHello
```

### Explanation

- `project HelloWorld` — Names the project
- `use Claude` — Sets the default AI model
- `task SayHello` — Defines a unit of work (like a function)
- `goal MainGoal` — Declares what success looks like
- `workflow MainWorkflow` — Orchestrates the tasks

### Run It

```bash
lucky run main.lk
```

---

## 2. Tasks — The Building Blocks

Tasks are Lucky's equivalent of functions. They are deterministic, checkpointable units of work.

### Basic Task

```lucky
task Greet
    output
        message: String
    steps
        let message = "Hello"
        return message
```

### Task with Input

```lucky
task Greet
    input
        name: String
    output
        greeting: String
    steps
        let greeting = "Hello, " + name
        return greeting
```

### Task with Policy

```lucky
task FetchData
    input
        url: URI
    output
        data: String
    policy
        retry 3
        timeout 30s
    steps
        let response = HTTP.get(url)
        return response.text()
```

### Control Flow in Tasks

```lucky
task ProcessUsers
    input
        users: List<String>
    output
        active_users: List<String>
    steps
        let active = []
        for user in users
            if user.starts_with("active_")
                active = active.append(user)
        return active
```

---

## 3. Agents — Intelligent Workers

Agents are stateful entities that own memory, tools, prompts, and permissions. They are the central abstraction in Lucky.

### Declaring an Agent

```lucky
agent Researcher
    model Claude
    memory ResearchMemory
    tools
        Browser, Search
    permissions
        allow browser.search, http.get
        deny filesystem.write, shell.exec
    policy
        retry 2
        timeout 5m
    prompt ResearchPrompt
```

### Agent with Embedded Tasks

```lucky
agent Coder
    model Claude
    tools
        Filesystem, Git, Shell

    task GenerateCode
        input
            spec: String
            language: String
        output
            code: String
        steps
            let code = ai.generate_code(spec, language)
            return code

    task FixBug
        input
            code: String
            bug_description: String
        output
            fixed_code: String
            explanation: String
        steps
            let result = ai.fix_code(code, bug_description)
            return result
```

### Using Agents

```lucky
workflow BuildFeature
    Researcher.Investigate(topic = "API design")
        ->
    Coder.GenerateCode(spec = context.research_output)
        ->
    Reviewer.ReviewCode(code = context.generated_code)
```

---

## 4. Workflows — Orchestration

Workflows define the directed acyclic graph (DAG) of agent and task execution.

### Sequential Workflow

```lucky
workflow BuildAndDeploy
    Research
        ->
    Design
        ->
    Implement
        ->
    Test
        ->
    Deploy
```

The `->` arrow means "execute after the previous step completes."

### Parallel Workflow

```lucky
workflow SecurityAudit
    StaticAnalysis
    DependencyScan
    SecretDetection
    ComplianceCheck
```

All four tasks start simultaneously. The workflow completes when all finish.

### Mixed Sequential + Parallel

```lucky
workflow CI
    Checkout
        ->
    parallel
        UnitTests
        IntegrationTests
        Lint
    wait
        ->
    Build
        ->
    Deploy
```

The `wait` keyword creates a barrier — execution continues only after all parallel branches complete.

### Conditional Workflow

```lucky
workflow DeployDecision
    Analyze
        ->
    if context.risk == "low"
        Deploy
    else
        RequestApproval
```

---

## 5. Goals — Defining Success

Goals declare what success means without prescribing implementation. Multiple workflows can satisfy a single goal.

### Simple Goal

```lucky
goal BuildWebsite
    success
        website.online
        website.tested
        website.documented
    workflow MainWorkflow
```

### Goal with Multiple Workflows

```lucky
goal GenerateReport
    workflow FastReport      # Quick, lower quality
    workflow ThoroughReport  # Slow, higher quality
```

The runtime selects the best workflow based on policy:

```lucky
policy ReportPolicy
    if context.priority == "quality"
        use ThoroughReport
    else
        use FastReport
```

---

## 6. Context — Ambient State

Context propagates automatically through the execution graph. No manual dependency injection.

### Declaring Context

```lucky
workflow BuildFeature
    context
        user: User
        repo: URI
        branch: String
        config: Config

    Analyze
        ->
    Implement
        ->
    Test
```

Every task in this workflow can access `context.user`, `context.repo`, etc. without explicit parameters.

### Task-Level Context

```lucky
task Analyze
    context
        analysis_depth: String = "deep"    # task-local context
    steps
        let repo = context.repo            # inherited from workflow
        let depth = context.analysis_depth  # local to this task
```

### Context in the Runtime

```lucky
# Pass context at execution time
lucky run main.lk --context '{"user":"alice","repo":"https://..."}'
```

---

## 7. Memory — Persistent Knowledge

Agents have persistent memory that survives across task invocations.

### Declaring Memory

```lucky
memory ProjectMemory
    scope project
    backend vector
    dimensions 1536
```

### Using Memory

```lucky
agent Planner
    memory ProjectMemory

    task Plan
        steps
            # Store knowledge
            ProjectMemory.remember("architecture_pattern", "microservices")

            # Retrieve by key
            let pattern = ProjectMemory.recall("architecture_pattern")

            # Semantic search
            let related = ProjectMemory.search("service communication", 5)

            # Forget outdated information
            ProjectMemory.forget("old_decision")
```

### Memory Scopes

| Scope | Lifetime |
|---|---|
| `local` | Duration of a single task |
| `session` | Duration of a user session |
| `project` | Lifetime of the project |
| `organization` | Shared across projects |
| `global` | Shared globally |

---

## 8. Tools — External Capabilities

Tools are capability interfaces to external systems.

### Built-in Tools

```lucky
# Filesystem
Filesystem.read("./config.toml")
Filesystem.write("./output.md", content)

# Git
Git.clone("https://github.com/org/repo.git")
Git.status()
Git.commit("feat: add new workflow")

# Shell
Shell.exec("cargo test")

# Browser
Browser.navigate("https://example.com")
Browser.extract("article.main-content")

# HTTP
let users = HTTP.get_json("/api/users")
HTTP.post_json("/api/tasks", { "title": "New Task" })
```

### Configuring Tools

```lucky
tool Git(
    workdir = "./repo",
    author_name = context.user.name,
    author_email = context.user.email,
)

tool Browser(
    headless = true,
    timeout = 30s,
)
```

---

## 9. Pipelines — Data Flow

Pipelines chain operations with the `|>` operator.

### Basic Pipeline

```lucky
files
    |> filter *.py
    |> summarize
    |> save report.md
```

### Pipeline with Lambda

```lucky
users
    |> filter fn u => u.age > 18
    |> map fn u => u.name
    |> sort
    |> take 10
```

### Query Expression (Alternative Syntax)

```lucky
users
    where age > 18
    where country == "US"
    select { name, email }
    order by name asc
    limit 10
```

### AI Pipeline

```lucky
Search.search("AI agent frameworks")
    |> extract
    |> rank relevance
    |> summarize
    |> save research.md
```

---

## 10. AI Models & Prompts

### Model Declaration

```lucky
model Claude(
    provider = "anthropic",
    version = "claude-sonnet-4-20250514",
    temperature = 0.7,
    max_tokens = 4096,
)

model GPT(
    provider = "openai",
    version = "gpt-4o",
)

model LocalLLM(
    provider = "ollama",
    version = "llama3.1",
)
```

### Model Selection

```lucky
use Claude           # Default for the module

agent Researcher
    use GPT          # Override for this agent

task QuickCheck
    use LocalLLM     # Override for this task
```

### Inline AI Calls

```lucky
let summary = ask Claude:
    Summarize the following text in 3 bullet points:
    {document_text}

let review = ask GPT:
    Review this code for security issues:
    ```python
    {code}
    ```
```

### Structured Prompts

```lucky
prompt CodeReviewer
    role
        You are a senior software engineer reviewing {language} code.
    rules
        - Report only actionable findings.
        - Cite specific line numbers.
        - Classify severity: low, medium, high, critical.
    context
        - Repository: {repo_name}
        - Branch: {branch}
    examples
        input:
            ```python
            query = f"SELECT * FROM users WHERE id = {user_id}"
            ```
        output:
            severity: high
            finding: SQL injection via string formatting
            recommendation: Use parameterized queries
    format
        Return JSON with fields: summary, findings[].
```

### Using Prompts

```lucky
agent Reviewer
    prompt CodeReviewer

    task ReviewCode
        steps
            let prompt_text = CodeReviewer.render({
                "language": "Python",
                "repo_name": "myapp",
                "branch": "main",
            })
            let review = Claude.complete(prompt_text)
            return review
```

---

## 11. Concurrency — Parallel & Swarm

### Parallel Execution

```lucky
parallel
    Researcher.search("topic A")
    Architect.design("component B")
    Security.audit("system C")
wait
```

### Await (Async within Tasks)

```lucky
task ProcessData
    steps
        let data = await Researcher.search("topic")
        let analysis = await Analyzer.analyze(data)
        return analysis
```

### Swarm Execution

Run many instances of an agent in parallel:

```lucky
swarm 50 Reviewer.review_patch(patches)
```

Each patch spawns an independent review instance. The runtime distributes work across available slots.

### Reactive Programming — When

```lucky
when
    main branch updates
    new PR opened
run
    ArchitectureReview
```

---

## 12. Error Handling — Attempt & Recover

Lucky uses explicit recovery policies instead of try/catch.

### Basic Recovery

```lucky
attempt
    deploy_to_production
recover
    retry 3 with backoff exponential(max: 5m)
recover
    fallback deploy_to_staging
recover
    human escalate "Deployment failed after 3 retries"
```

### Recovery Actions

| Action | Description |
|---|---|
| `retry N` | Re-execute up to N times |
| `retry with backoff linear` | Retry with increasing delay |
| `retry with backoff exponential` | Retry with exponential backoff |
| `fallback task` | Execute an alternative task |
| `human escalate "msg"` | Escalate to a human operator |
| `abort` | Terminate the workflow |
| `skip` | Skip the failed task and continue |

### Policy-Based Recovery

```lucky
policy ResilientPolicy
    retry 3 with backoff exponential(max: 10m)
    checkpoint before retry
    on_permanent_failure fallback
    on_transient_failure retry

task CriticalOperation
    policy ResilientPolicy
    steps
        ...
```

---

## 13. Permissions & Security

Lucky enforces capability security. Agents run with explicit, least-privilege permission sets.

### Declaring Permissions

```lucky
permissions
    allow
        filesystem.read
        git.clone
        git.commit
        browser.search
        http.get
    deny
        filesystem.delete
        filesystem.write(/etc/*)
        git.push(main)
        shell.exec
```

### Agent-Level Permissions

```lucky
agent RestrictedAgent
    permissions
        allow filesystem.read
        deny filesystem.write
```

Permissions are inherited from the project scope and can be further restricted by agents but never expanded.

### Permission Patterns

| Pattern | Matches |
|---|---|
| `filesystem.read` | Exact match |
| `filesystem.*` | All filesystem operations |
| `git.push(feature/*)` | Push to feature branches only |
| `http.*` | All HTTP methods |

---

## 14. Human Approval

Human judgment is a first-class language construct.

### Approval Gates

```lucky
approval
    before deploy
    before filesystem.delete(/production/*)
    before git.push(main)
```

Execution suspends until a human approves.

### Inline Human Queries

```lucky
let confirmed = ask human:
    Deploy version {version} to production?
    Changes: {changelog}
    Risk: medium

if confirmed
    deploy
else
    abort
```

### Approval with Timeout

```lucky
approval
    before deploy
    timeout 4h escalate to manager
```

If approval doesn't arrive within 4 hours, the request escalates.

---

## 15. Patterns & Best Practices

### Pattern: Research → Plan → Execute

```lucky
workflow StandardPipeline
    context
        topic: String
        output_format: String = "markdown"

    Researcher.Investigate(topic = context.topic)
        ->
    Planner.Decompose(goal = "Create {context.output_format} report")
        ->
    Coder.Generate(spec = context.plan)
        ->
    Reviewer.ReviewCode(code = context.code)
```

### Pattern: Agent with Multiple Strategies

```lucky
agent Analyzer
    model Claude

    task QuickAnalysis
        policy timeout 1m
        steps
            reason fast
            return ai.ask("Quick summary: {context.topic}")

    task DeepAnalysis
        policy timeout 10m
        steps
            reason deep
            let research = Search.search(context.topic, max_results = 20)
            let analysis = ai.summarize(research, style = "detailed")
            return analysis
```

### Pattern: Test-Driven Development

```lucky
# tests/security.test.lk
test "code review finds SQL injection" {
    let sql = "SELECT * FROM users WHERE id = " + user_input
    let review = Reviewer.ReviewCode(code = sql, focus_areas = ["security"])
    assert review.severity >= "high"
    assert review.category == "sql_injection"
}

test "all files have proper permissions" {
    let files = Filesystem.list("./src")
    for file in files
        assert filesystem.permissions(file) != "0777"
}
```

### Pattern: Pipeline + Map/Reduce

```lucky
task ProcessLogs
    input
        log_dir: String
    output
        summary: Map<String, Int>
    steps
        let results = Filesystem.glob(log_dir + "/**/*.log")
            |> map fn f => Filesystem.read(f)
            |> filter fn content => content.contains("ERROR")
            |> map fn content => extract_error_type(content)
            |> group_by fn err => err
            |> map fn (k, v) => { k: v.len() }
        return results
```

### Pattern: Multi-Agent Collaboration

```lucky
workflow SoftwareDevelopment
    context
        feature_spec: String
        language: String = "Rust"
        repo: URI

    parallel
        Researcher.Investigate(
            topic = "Best practices for {context.feature_spec} in {context.language}"
        )
        Architect.DesignSystem(
            requirements = context.feature_spec,
            constraints = { "language": context.language }
        )
    wait
        ->
    Coder.Generate(
        spec = context.architecture,
        language = context.language
    )
        ->
    parallel
        Reviewer.ReviewCode(code = context.code)
        Tester.GenerateTests(code = context.code)
    wait
        ->
    if context.review.approved and context.tests.passed
        Git.commit("feat: {context.feature_spec}")
    else
        Coder.FixBug(
            code = context.code,
            bug_description = context.review.findings
        )
```

---

## Where to Go Next

- [Language Reference Manual](../Lucky%20Language%20Reference%20Manual%20V0.1.md) — Complete syntax and semantics
- [Standard Library Reference](../Lucky%20Standard%20Library%20Specification%20V0.1.md) — All built-in types, tools, and APIs
- [Runtime Specification](../Lucky%20Runtime%20Specification%20V0.1.md) — Execution engine internals
- [IR Specification](../Lucky%20IR%20Specification%20V0.1.md) — Intermediate representation and optimization
- [Tool Protocol (LTP)](../Lucky%20Tool%20Protocol%20Specification%20V0.1.md) — Cross-platform execution protocol
- [Quickstart Guide](quickstart.md) — Fast setup reference
