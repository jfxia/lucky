# Lucky Programming Language Specification

Version 0.1 Draft

---

# Part I. Philosophy

## 1.1 Goal

Lucky is a programming language for autonomous AI systems.

Unlike traditional languages designed around CPU execution, Lucky is designed around **goal execution**.

A Lucky program specifies:

* goals
* workflows
* agents
* tools
* knowledge
* permissions
* execution policies

instead of algorithms.

---

## 1.2 Design Principles


**Lucky is a declarative workflow language with imperative islands.**

That means:

* 80% of Lucky code declares **what** should happen.
* Only small blocks describe **how**.
* The runtime, not the programmer, decides scheduling, concurrency, retries, checkpointing, context propagation, and LLM execution.

This makes Lucky fundamentally AI-native.


Lucky follows eight principles.

### (1) Intent First

Programs express intent before implementation.

Instead of

```
for(...)
```

Lucky prefers

```
select
summarize
review
search
```

---

### (2) AI Native

Large language models are part of the language.

Not libraries.

Not SDKs.

---

### (3) Deterministic Structure

Although AI is probabilistic, the program structure is deterministic.

Execution graph is always reproducible.

---

### (4) Context Everywhere

Context is automatically propagated.

No manual dependency injection.

---

### (5) Parallel by Default

Independent tasks execute simultaneously.

Sequential execution must be requested explicitly.

---

### (6) Human Approval

Human interaction is a language feature.

---

### (7) Recoverability

Every task is resumable.

---

### (8) Capability Security

Agents execute under explicit permissions.

---

# Part II. Program Structure

A Lucky source file, with the file suffix '**.lk**', is called a Module.

```
project MyProject

imports

definitions

workflows

tasks

agents
```

Compilation unit:

```
Project

 ├── Modules

 ├── Packages

 └── Runtime Manifest
```

---

# Part III. Lexical Structure

Whitespace is significant.

Indentation defines blocks.

Semicolons do not exist.

Comments

```
# single line

## documentation comment
```

Identifiers

```
Planner

user_name

repo

BuildWebsite
```

Unicode identifiers are allowed.

Reserved keywords cannot be identifiers.

---

# Part IV. Primitive Types

Lucky intentionally keeps the primitive type system small.

```
Bool

Int

Float

Decimal

String

Bytes

Time

Duration

UUID

URI

Version
```

Special values

```
null

unknown

error
```

Unlike null,

```
unknown
```

means "not yet computed."

---

# Part V. Collection Types

```
List<T>

Set<T>

Map<K,V>

Queue<T>

Graph<T>

Tree<T>

Stream<T>
```

All collections are immutable.

Mutations return new collections.

Example

```
users2 = users.add(user)
```

---

# Part VI. AI Types

Lucky introduces first-class AI types.

```
Agent

Task

Workflow

Goal

Prompt

Memory

Embedding

Knowledge

Context

Capability

Approval

Tool

Model

Reasoning

Observation

Artifact

Plan

Result
```

These cannot be represented naturally in existing languages.

---

# Part VII. Type System

Strongly typed.

Static type checking.

Gradual typing for AI outputs.

Example

```
task Search

returns List<Document>
```

LLM output

```
returns uncertain Report
```

The compiler knows

```
Report
```

is probabilistic.

---

### Confidence Types

Every uncertain value carries confidence.

```
Report?

```

is not optional.

It means

```
Probabilistic<Report>
```

Internally

```
value

confidence

reasoning

citations
```

---

# Part VIII. Variables

Immutable.

```
let

const
```

Example

```
let report = summarize docs
```

No reassignment.

Mutable state belongs inside Agents.

---

# Part IX. Agent

Agent is Lucky's central abstraction.

```
agent Researcher

memory

tools

permissions

prompt

policy

tasks
```

Agent owns:

* identity
* memory
* context
* reasoning strategy
* execution policy

---

# Part X. Task

Task replaces function.

```
task ReviewCode

input

repo

output

ReviewReport

steps

...
```

Tasks are pure unless explicitly declared stateful.

---

# Part XI. Workflow

Workflow orchestrates tasks.

```
workflow Build

Research

↓

Plan

↓

Implement

↓

Review
```

Execution graph is DAG.

Cycles require explicit loop syntax.

---

# Part XII. Goal

Goal is the entry point.

```
goal

BuildWebsite
```

Goals are declarative.

Multiple workflows may satisfy one goal.

Runtime chooses according to policy.

---

# Part XIII. Context

Context is implicit.

```
context

user

repo

memory

history
```

Tasks automatically inherit context.

---

# Part XIV. Memory

```
memory ProjectMemory
```

Scopes

```
local

session

project

organization

global
```

Memory may be vector-based or structured.

Programmer does not care.

---

# Part XV. Tool

```
tool Git

tool Browser

tool Shell
```

Tool invocation

```
Git.clone repo

Browser.search

Shell.exec
```

---

# Part XVI. Model

```
model Claude

model GPT

model Gemini

model Local
```

Selection

```
use Claude
```

or

```
Planner uses GPT
```

---

# Part XVII. Prompt

Prompt is structured.

```
prompt Reviewer

role

security reviewer

rules

...

examples

...
```

Prompt is not a string.

Compiler validates sections.

---

# Part XVIII. Execution Policies

```
policy

parallel

retry 3

timeout 30m

checkpoint

approval

cache

sandbox
```

---

# Part XIX. Concurrency

Parallel is default.

```
Research

Security

Architecture
```

Compiler executes simultaneously.

Sequential

```
Research

->

Plan

->

Code
```

---

# Part XX. Error Model

Lucky avoids exceptions.

Every task returns

```
Success

Failure

Cancelled

Skipped
```

Recovery

```
recover

retry

fallback

human

abort
```

---

# Part XXI. Permissions

```
allow

filesystem.read

git.push

browser.search
```

```
deny

filesystem.delete
```

Permission inheritance is lexical.

---

# Part XXII. Approval

```
approval

before deploy
```

Human approval suspends execution.

---

# Part XXIII. Import System

```
import github

import browser

import company.security
```

Packages expose

```
agents

tasks

tools

types
```

---

# Part XXIV. Execution Model

Lucky does **not** compile directly to machine code.

Instead:

```
Lucky Source

↓

Lexer

↓

Parser

↓

AST

↓

Semantic Analyzer

↓

Lucky IR

↓

Planner

↓

Execution DAG

↓

Runtime Scheduler

↓

Tool Calls

↓

LLM

↓

Artifacts
```

The runtime is responsible for:

* Dependency analysis.
* Automatic parallel scheduling.
* Context propagation.
* Checkpointing and resumability.
* Retry and fallback policies.
* Cost and latency optimization.

This separation lets the same Lucky program run on different backends (Claude Code, Codex CLI, OpenCode, or a standalone runtime) without changing source code.

# Part XXV. Formal Grammar (EBNF)

The language should have a compact, machine-readable grammar. A simplified core is:

```ebnf
program        = projectDecl { moduleItem } ;

projectDecl    = "project" Identifier ;

moduleItem     =
		importDecl
	| typeDecl
	| agentDecl
	| taskDecl
	| workflowDecl
	| goalDecl
	| memoryDecl
	| toolDecl
	| modelDecl
	;

agentDecl      = "agent" Identifier Block ;

taskDecl       = "task" Identifier Block ;

workflowDecl   = "workflow" Identifier Block ;

goalDecl       = "goal" Identifier Block ;

Block          = INDENT { Statement } DEDENT ;

Statement      =
		LetStmt
	| CallStmt
	| IfStmt
	| MatchStmt
	| ParallelStmt
	| PipelineStmt
	| ReturnStmt
	;
```


---

