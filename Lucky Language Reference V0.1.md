# Lucky Language Reference

**Version:** 0.1 Draft

**Status:** Language Specification

**Language:** Lucky

**Abbreviation:** LK

---

# Table of Contents

```
Part I      Introduction

Chapter 1   Overview
Chapter 2   Design Philosophy
Chapter 3   Language Concepts

----------------------------------------

Part II     Lexical Structure

Chapter 4   Source Files
Chapter 5   Unicode
Chapter 6   Tokens
Chapter 7   Keywords
Chapter 8   Comments
Chapter 9   Identifiers
Chapter 10  Literals

----------------------------------------

Part III    Grammar

Chapter 11  Program Structure
Chapter 12  Modules
Chapter 13  Imports
Chapter 14  Packages
Chapter 15  Grammar (Complete EBNF)

----------------------------------------

Part IV     Type System

Chapter 16  Primitive Types
Chapter 17  Composite Types
Chapter 18  Nullable Types
Chapter 19  Optional Types
Chapter 20  Union Types
Chapter 21  Generic Types
Chapter 22  AI Types
Chapter 23  Context Types
Chapter 24  Type Inference
Chapter 25  Type Compatibility
Chapter 26  Lifetime Rules

----------------------------------------

Part V      Expressions

Chapter 27  Literals
Chapter 28  Variables
Chapter 29  Assignment
Chapter 30  Operators
Chapter 31  Pipelines
Chapter 32  Pattern Matching
Chapter 33  Lambda Expressions
Chapter 34  Query Expressions

----------------------------------------

Part VI     Statements

Chapter 35  Blocks
Chapter 36  If
Chapter 37  Match
Chapter 38  Loop
Chapter 39  Parallel
Chapter 40  Await
Chapter 41  Return
Chapter 42  Break
Chapter 43  Continue

----------------------------------------

Part VII    AI Programming Model

Chapter 44  Goals
Chapter 45  Tasks
Chapter 46  Agents
Chapter 47  Workflows
Chapter 48  Models
Chapter 49  Prompts
Chapter 50  Memory
Chapter 51  Knowledge
Chapter 52  Context
Chapter 53  Tools
Chapter 54  Permissions
Chapter 55  Policies
Chapter 56  Human Approval

----------------------------------------

Part VIII   Runtime

Chapter 57  Execution Graph
Chapter 58  Scheduling
Chapter 59  Dependency Analysis
Chapter 60  Checkpoints
Chapter 61  Recovery
Chapter 62  Transactions
Chapter 63  State Management
Chapter 64  Cost Optimization

----------------------------------------

Part IX     Concurrency

Chapter 65  Task Parallelism
Chapter 66  Agent Parallelism
Chapter 67  Synchronization
Chapter 68  Streams
Chapter 69  Event System

----------------------------------------

Part X      Error Model

Chapter 70  Result Type
Chapter 71  Recovery Policies
Chapter 72  Retry
Chapter 73  Rollback
Chapter 74  Human Escalation

----------------------------------------

Part XI     Standard Library

Chapter 75  Collections
Chapter 76  File System
Chapter 77  Git
Chapter 78  Browser
Chapter 79  Shell
Chapter 80  HTTP
Chapter 81  AI
Chapter 82  Time
Chapter 83  Math

----------------------------------------

Appendix A Grammar
Appendix B Keywords
Appendix C Operators
Appendix D IR Mapping
Appendix E Memory Model
```

---



Lucky has this semantic hierarchy:

```
Goal

↓

Workflow

↓

Agent

↓

Task

↓

Operation

↓

Tool

↓

Runtime

↓

Backend
```

This is the first truly AI-native execution model.

---

# Semantic Hierarchy

Every Lucky program consists of only six semantic objects.

```
Goal

Workflow

Agent

Task

Operation

Artifact
```

Everything else is metadata.

---

# Example

Instead of

```python
main()

↓

foo()

↓

bar()
```

Lucky becomes

```
Goal

↓

Workflow

↓

Agent

↓

Task

↓

Tool
```

---

# Core Semantic Objects

## Goal

Highest abstraction.

Defines **what success means**.

Example

```lucky
goal BuildWebsite

success

website.online
```

Goals never contain implementation.

---

## Workflow

Defines

> How goals are achieved.

Example

```lucky
workflow BuildWebsite

Research

↓

Design

↓

Implement

↓

Review

↓

Deploy
```

---

## Agent

Defines

Who performs work.

```
agent SecurityReviewer

model Claude

memory Project

tools

Git

Browser
```

---

## Task

Smallest schedulable unit.

```
task AnalyzeAPI

input

repo

output

Report
```

Tasks are deterministic containers.

---

## Operation

Operations are built-in language primitives.

Examples

```
search

summarize

extract

clone

commit

deploy

reason

review

compare
```

These are not library calls.

They're language instructions.

---

## Artifact

Every execution produces artifacts.

```
Markdown

PDF

Patch

Image

Repository

Knowledge

Decision
```

Artifacts are immutable.

---

# A Radical Type System

Instead of

```
int

float

class
```

Lucky has two independent type dimensions.

```
Value Types

Behavior Types
```

Value

```
String

Int

Bool

List

Map
```

Behavior

```
Task

Agent

Workflow

Goal

Prompt

Tool
```

Therefore

```
Agent<Research>

Task<Review>

Workflow<CI>

Goal<Deploy>
```

are types.

Not keywords.

---

# Everything is a Graph

This is probably Lucky's biggest innovation.

Every source file compiles into a DAG.

Never bytecode.

Never machine code.

Example

```
Planner

↓

Coder

↓

Tester
```

becomes

```
Node

↓

Node

↓

Node
```

Parallel

```
Research

Security

Architecture
```

becomes

```
      Research

      /

Planner

      \

Security

      \

Architecture
```

Compiler constructs graph.

Runtime schedules graph.

---

# The Runtime Owns Control Flow

Traditional language

```
for

while

if
```

Lucky runtime

```
dependency

policy

priority

confidence

cost

approval
```

The runtime decides execution.

Not programmer.

---

# AI is Not a Library

This is another principle.

Python

```
import openai
```

Lucky

```
model Claude
```

LLM is as fundamental as

```
Int
```

---

# Execution States

Every task has exactly one lifecycle.

```
Created

↓

Ready

↓

Running

↓

Waiting

↓

Checkpointed

↓

Completed
```

or

```
↓

Failed

↓

Recovered

↓

Completed
```

or

```
↓

Cancelled
```

---

# Context Propagation

Lucky never passes 12 arguments.

Instead

```
Context

↓

Workflow

↓

Agent

↓

Task

↓

Operation
```

Context automatically flows downward.

---



Lucky could define an equivalent for AI systems:

```
Lucky Source (.lk)

↓

Lucky Parser

↓

Lucky AST

↓

Lucky Semantic Analyzer

↓

Lucky IR (.lir)

↓

Lucky Runtime

↓

Backend Adapter

↓

Claude Code

Codex CLI

OpenCode

Cursor

Dify

OpenHands

Local Agents
```

In other words, **Lucky should not primarily be a programming language. It should be an ecosystem standard.**

The language would simply be the most human-friendly way to author programs targeting **Lucky IR**, while the runtime and backend adapters make those programs portable across AI coding platforms. This is analogous to how many languages target LLVM or how many compilers target WebAssembly.
