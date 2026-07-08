# Lucky Language Reference Manual
<img src="../../logo/logo128.png" alt="Lucky logo" width="64" align="right" />


**Version:** 0.1 Draft  
**Language:** Lucky (abbreviation: LK)  
**Status:** Language Specification  
**Copyright:** 2026 Jingfeng Xia  

---

# Part I &mdash; Introduction

---

## Chapter 1 &mdash; Overview

Lucky is a goal-oriented orchestration language for AI agents. Unlike traditional programming languages designed around CPU execution and deterministic algorithms, Lucky is designed around **goal execution** &mdash; the coordination of autonomous AI agents that reason, plan, and act under explicit human supervision.

A Lucky program does not describe a sequence of machine instructions. Instead it specifies:

* **goals** &mdash; what success means
* **workflows** &mdash; how goals are decomposed into executable steps
* **agents** &mdash; intelligent entities that own memory, tools, and reasoning strategies
* **tasks** &mdash; deterministic, schedulable units of computation
* **tools** &mdash; capability interfaces to external systems
* **context** &mdash; ambient execution state that propagates automatically
* **permissions** &mdash; capability-security boundaries
* **policies** &mdash; retry, timeout, checkpoint, and approval rules

The Lucky toolchain compiles source files (`.lk`) through a parser and semantic analyzer into a language-neutral intermediate representation called **Lucky IR**. The IR is a directed acyclic graph (DAG) where nodes represent tasks, tools, reasoning steps, conditions, approvals, and data flow. A runtime scheduler executes the DAG, managing concurrency, context propagation, checkpointing, retry, and cost optimization. Backend adapters map the IR onto concrete AI platforms &mdash; Claude Code, Codex CLI, OpenCode, or a standalone Lucky engine &mdash; so that the same Lucky program runs portably across environments.

Lucky occupies the abstraction layer between Python (general-purpose scripting) and natural language (unstructured human communication). It treats large language models as a language primitive, not as a library.

---

## Chapter 2 &mdash; Design Philosophy

Lucky is governed by eight core principles.

### 2.1 Intent First

Programs express **what** should be accomplished before **how** it should be accomplished. High-level operations such as `select`, `summarize`, `review`, and `search` are first-class language constructs. Imperative control flow is reserved for small, deterministic islands within a declarative whole.

### 2.2 AI Native

Large language models are part of the language. A program declares `model Claude` or `use GPT` directly &mdash; there is no SDK, no API wrapper, and no string-based prompt construction. The compiler understands model selection, prompt structure, and output confidence as type-level concerns.

### 2.3 Deterministic Structure

Although AI reasoning is probabilistic, the program structure itself is deterministic. The execution graph is always reproducible; only the leaf-level LLM invocations are non-deterministic. Checkpointing and replay guarantee that every Lucky execution can be reconstructed.

### 2.4 Context Everywhere

Variables are not manually threaded through call chains. Context &mdash; user identity, repository state, session history, configuration &mdash; propagates automatically from workflow scope downward into agents, tasks, and operations. This eliminates a major source of boilerplate and error.

### 2.5 Parallel by Default

Independent tasks execute concurrently unless the programmer explicitly requests sequential ordering. The runtime analyzes the execution DAG and schedules ready nodes simultaneously, bounded by resource limits and cost constraints.

### 2.6 Human Approval

Human judgment is a language feature. An `approval` block can gate any operation; execution suspends until a human sign-off arrives. This makes Lucky suitable for safety-critical workflows such as deployment, database migration, and destructive operations.

### 2.7 Recoverability

Every task is resumable. The runtime checkpoints task state automatically. On failure the system can retry, roll back, fall back to an alternative agent, or escalate to a human operator &mdash; all expressed as language-level recovery policies.

### 2.8 Capability Security

Agents execute under explicit, least-privilege permission sets. Permissions are lexical (inherited from enclosing scopes) and may be further restricted but never widened. A `deny` clause takes precedence over any `allow`.

### The Abstraction Ladder

```
Natural Language
		↑
   Lucky
		↑
   Python
		↑
	Go
		↑
 Rust / C++
		↑
	C
		↑
 Assembly
```

Lucky fills the gap between natural-language intent and executable software, providing a deterministic scaffold around probabilistic AI.

---

## Chapter 3 &mdash; Language Concepts

A Lucky program is built from exactly six semantic objects. Everything else is metadata.

```
Goal       →  "what success means"
Workflow   →  "how goals are achieved"
Agent      →  "who performs work"
Task       →  "the smallest schedulable unit"
Operation  →  "a built-in language primitive"
Artifact   →  "an immutable output of execution"
```

### Goal

The highest abstraction. A goal declares success criteria without implementation details. Multiple workflows may satisfy a single goal; the runtime selects among them according to policy.

```lucky
goal BuildWebsite
	success website.online
```

### Workflow

A directed acyclic graph of agents and tasks that together satisfy a goal. The workflow body uses indentation and arrows to express dependency order.

```lucky
workflow BuildWebsite
	Research
		->
	Design
		->
	Implement
		->
	Review
		->
	Deploy
```

### Agent

A stateful entity that owns memory, tools, prompts, reasoning strategy, and permissions. Agents are the central abstraction &mdash; the Lucky equivalent of a class.

```lucky
agent SecurityReviewer
	model Claude
	memory ProjectMemory
	tools
		Git
		Browser
```

### Task

The smallest schedulable, checkpointable unit of work. Tasks declare typed inputs and outputs, and may contain imperative steps.

```lucky
task AnalyzeRepo
	input repo: URI
	output report: Document
	steps
		clone repo
		read README
		understand architecture
		generate report
```

### Operation

Built-in language primitives that represent common AI-orchestration actions. Operations are not library calls; they are language instructions.

```
search   summarize   extract   clone   commit
deploy   reason      review    compare   rank
```

### Artifact

Every execution produces artifacts &mdash; immutable results such as documents, patches, images, repositories, knowledge bases, and decisions. Artifacts are versioned and addressable.

```
Markdown   PDF   Patch   Image   Repository   Knowledge   Decision
```

### The Semantic Hierarchy

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

---

# Part II &mdash; Lexical Structure

---

## Chapter 4 &mdash; Source Files

A Lucky source file uses the extension `.lk`. It must be valid UTF-8. Each file constitutes a **module**.

A module consists of:

* An optional `project` declaration (required for the root module of a project)
* Import declarations
* Type declarations
* Agent, task, workflow, and goal definitions
* Memory, tool, and model declarations

Files are organized in a conventional directory structure:

```
project/
	main.lk
	agents/
		coder.lk
		reviewer.lk
		planner.lk
	memory/
		permissions.lk
	tasks/
		build.lk
		deploy.lk
```

Line terminators are LF (U+000A) or CRLF (U+000D U+000A). A file must end with a line terminator. There is no line-continuation character; long expressions are broken across lines naturally via indentation rules.

---

## Chapter 5 &mdash; Unicode

Lucky source is Unicode 15.0 or later. The following Unicode categories are permitted within the source:

* **Identifiers**: Letters (L categories), numbers (N categories, but not as the first character), underscore (U+005F), and connecting punctuation (Pc category).
* **String literals**: Any Unicode scalar value except the unescaped quote delimiter and (for multi-line strings) the closing delimiter.
* **Comments**: Any Unicode scalar value.
* **Whitespace**: Space (U+0020), horizontal tab (U+0009), and newline sequences. Vertical tab (U+000B) and form feed (U+000C) are not whitespace in Lucky.

The byte order mark (BOM, U+FEFF) at the start of a file is permitted and ignored.

---

## Chapter 6 &mdash; Tokens

The lexer produces the following token classes:

| Token Class | Description | Examples |
|---|---|---|
| `IDENT` | Identifier | `Planner`, `user_name`, `repo` |
| `KEYWORD` | Reserved word | `agent`, `task`, `goal`, `let` |
| `INT_LIT` | Integer literal | `42`, `0xFF`, `1_000_000` |
| `FLOAT_LIT` | Floating-point literal | `3.14`, `1.0e9` |
| `STRING_LIT` | String literal | `"hello"`, `"""multi"""` |
| `BOOL_LIT` | Boolean literal | `true`, `false` |
| `NULL_LIT` | Null literal | `null` |
| `UNKNOWN_LIT` | Unknown literal | `unknown` |
| `OPERATOR` | Operator or punctuation | `->`, `\|>`, `+`, `==` |
| `COMMENT` | Comment | `# single`, `## doc` |
| `NEWLINE` | Logical newline | (end of logical line) |
| `INDENT` | Increase indentation | (leading whitespace increase) |
| `DEDENT` | Decrease indentation | (leading whitespace decrease) |
| `EOF` | End of file | |

Whitespace at the beginning of a line determines indentation level. The lexer emits `INDENT` and `DEDENT` tokens following Python-style indentation rules. The first non-blank, non-comment line establishes indentation level zero.

Blank lines (lines containing only whitespace or comments) do not affect indentation.

---

## Chapter 7 &mdash; Keywords

The following are reserved keywords and may not be used as identifiers:

**AI model keywords:**
```
agent      task       workflow   goal       prompt
memory     knowledge  context    tool       model
capability approval   permission policy
```

**Declaration keywords:**
```
project    import     use        type       let
const      return     recover
```

**Control flow keywords:**
```
if         else       match      loop       for
while      in         when       parallel   await
break      continue   then       run
```

**Value keywords:**
```
true       false      null       unknown    error
```

**Modifier keywords:**
```
input      output     steps      success    attempt
retry      fallback   human      abort      allow
deny       deep       fast       none       skip
```

**Pseudo-keywords (context-dependent, may be used as identifiers in some positions):**
```
select     search     summarize  extract    clone
commit     push       deploy     reason     review
compare    rank       filter     map        reduce
sort       group      join       save       load
```

---

## Chapter 8 &mdash; Comments

### Single-line comments

A `#` character outside a string literal begins a comment that extends to the end of the line.

```lucky
# This is a comment
let x = 10  # inline comment
```

### Documentation comments

A `##` prefix marks a documentation comment. Documentation comments attach to the immediately following declaration and are preserved in the AST for tooling (LSP, documentation generators).

```lucky
## SecurityReviewer audits code changes for vulnerabilities.
## It uses static analysis and LLM reasoning.
agent SecurityReviewer
	model Claude
```

### Block comments

Lucky has no block-comment syntax. Long documentation is written as consecutive `##` lines or multi-line string literals assigned to a `doc` section within an agent or task body.

---

## Chapter 9 &mdash; Identifiers

Identifiers follow the pattern:

```
Identifier = (Letter | "_") { Letter | Digit | "_" }
```

where `Letter` includes Unicode L and Pc categories.

### Naming conventions (by convention, not enforced)

| Entity | Convention | Example |
|---|---|---|
| Agents | PascalCase | `SecurityReviewer`, `CodeGenerator` |
| Tasks | PascalCase | `AnalyzeRepo`, `GenerateReport` |
| Workflows | PascalCase | `CIBuild`, `DeployProduction` |
| Goals | PascalCase | `BuildWebsite`, `AuditSecurity` |
| Variables | snake_case | `user_count`, `repo_url` |
| Constants | UPPER_SNAKE_CASE | `MAX_RETRIES`, `DEFAULT_MODEL` |
| Tools | PascalCase | `Git`, `Browser`, `Shell` |
| Models | PascalCase | `Claude`, `GPT`, `Gemini` |

### Qualified identifiers

A dot separates module path components and member access:

```lucky
company.security.Reviewer
agents.coder.generate
```

---

## Chapter 10 &mdash; Literals

### Boolean literals

```
true
false
```

### Integer literals

Decimal, hexadecimal (`0x` or `0X` prefix), and binary (`0b` or `0B` prefix) integer literals are supported. Underscores may be used as visual separators.

```lucky
42
0x2A
0b101010
1_000_000
```

The type of an integer literal is `Int` (a signed 64-bit integer) unless context demands a wider type.

### Float literals

```lucky
3.14
1.0e9
2.5e-3
0.5
.5         # error: leading dot required
```

The type is `Float` (IEEE 754 binary64).

### String literals

Single-line strings use double quotes:

```lucky
"hello world"
"line1\nline2"
"value = \{variable}"   # interpolation
```

Multi-line strings use triple quotes:

```lucky
"""
This is a multi-line string.
It preserves indentation relative
to the closing delimiter.
"""
```

Escape sequences: `\\`, `\"`, `\n`, `\r`, `\t`, `\{`, `\u{XXXXXX}`.

String interpolation embeds an expression between `\{` and `}`.

### Null literal

```lucky
null
```

Denotes the intentional absence of a value.

### Unknown literal

```lucky
unknown
```

Denotes "not yet computed" &mdash; distinct from `null`. Used for deferred AI outputs.

### Error literal

```lucky
error("reason")
```

Produces an `Error` value.

### List, Set, and Map literals

```lucky
[1, 2, 3]                   # List<Int>
{1, 2, 3}                   # Set<Int>
{"key": "value", "a": 1}    # Map<String, Any>
```

An empty `[]` or `{}` requires a type annotation unless inferable from context.

---

# Part III &mdash; Grammar

---

## Chapter 11 &mdash; Program Structure

A Lucky program is a project containing one or more modules. The root module declares the project name and optionally imports other modules.

```lucky
project MyProject

import github
import browser
import company.security

agent Planner
	model Claude

task BuildReport
	...

goal Deliver
	workflow MainWorkflow
```

### Compilation units

```
Project
 ├── Modules (.lk files)
 ├── Packages (imported capabilities)
 └── Runtime Manifest (lucky.toml)
```

The `lucky.toml` manifest (analogous to `Cargo.toml` or `pyproject.toml`) specifies project metadata, dependencies, and default model configuration:

```toml
[project]
name = "MyProject"
version = "0.1.0"

[models]
default = "claude-sonnet-4"

[dependencies]
github = "1.0"
browser = "2.0"
```

---

## Chapter 12 &mdash; Modules

A module is a single `.lk` file. Module names are derived from the file path relative to the project root, with `/` replaced by `.` and the `.lk` suffix removed.

```
project/
	agents/
		coder.lk          → module agents.coder
	tasks/
		build.lk          → module tasks.build
```

A module may contain any number of declarations. All top-level declarations in a module are private to that module unless explicitly exported.

### Export

Declarations prefixed with `pub` are exported and visible to importing modules:

```lucky
pub agent Reviewer
	...
```

If a module contains a `project` declaration, it is the root module of the project. All other modules in the project implicitly belong to that project.

---

## Chapter 13 &mdash; Imports

### Module imports

```lucky
import agents.coder
import tasks.build
```

This imports the module and makes its exported names available via qualified access:

```lucky
agents.coder.generate(...)
```

### Named imports

```lucky
import agents.coder { generate, review }
```

Imports specific names into the current scope.

### Wildcard imports

```lucky
import agents.coder.*
```

Imports all exported names. Discouraged for production code.

### Alias imports

```lucky
import agents.coder as Coder
```

### Package imports

```lucky
import github
import browser
import company.security
```

Packages expose capability bundles: agents, tasks, tools, and types. A package import brings all exported capabilities into scope.

---

## Chapter 14 &mdash; Packages

A Lucky package is a distributable capability bundle. Unlike traditional libraries (which expose functions and classes), a Lucky package exposes:

* **agents** &mdash; reusable agent definitions
* **tasks** &mdash; composable task templates
* **tools** &mdash; capability interfaces with backend implementations
* **types** &mdash; shared type definitions

Packages are published to a registry and declared in `lucky.toml` dependencies.

Standard capability packages:

```
browser          web navigation and scraping
database         SQL and vector database access
vision           image understanding and generation
speech           text-to-speech and speech-to-text
search           web search and retrieval
rag              retrieval-augmented generation
agent            agent orchestration primitives
github           GitHub API and git operations
```

### Package definition

A package is itself a Lucky project with a `lucky.toml` that specifies `[package]` metadata:

```toml
[package]
name = "github"
version = "1.0.0"
description = "GitHub API and git operation capabilities"

[exports]
agents = ["RepoAnalyzer", "PRReviewer"]
tasks = ["CloneRepo", "CreatePR"]
tools = ["GitCLI", "GitHubAPI"]
```

---

## Chapter 15 &mdash; Grammar (Complete EBNF)

The complete grammar of Lucky source files:

```ebnf
(* === Top Level === *)

program         = [ projectDecl ] { moduleItem } ;

projectDecl     = "project" Identifier NEWLINE ;

moduleItem      = pubDecl | importDecl | typeDecl | agentDecl
				| taskDecl | workflowDecl | goalDecl | memoryDecl
				| toolDecl | modelDecl | promptDecl | policyDecl
				| contextDecl | permissionDecl | approvalDecl
				;

pubDecl         = "pub" moduleItem ;

(* === Imports === *)

importDecl      = "import" qualifiedName [ importSelect ] [ aliasDecl ] NEWLINE ;

importSelect    = "{" identifierList "}" | ".*" ;

aliasDecl       = "as" Identifier ;

qualifiedName   = Identifier { "." Identifier } ;

(* === Declarations === *)

agentDecl       = "agent" Identifier NEWLINE block ;

taskDecl        = "task" Identifier NEWLINE taskBody ;

workflowDecl    = "workflow" Identifier NEWLINE block ;

goalDecl        = "goal" Identifier NEWLINE goalBody ;

memoryDecl      = "memory" Identifier NEWLINE block ;

toolDecl        = "tool" Identifier NEWLINE block ;

modelDecl       = "model" Identifier [ modelParams ] NEWLINE ;

promptDecl      = "prompt" Identifier NEWLINE block ;

policyDecl      = "policy" Identifier NEWLINE block ;

contextDecl     = "context" NEWLINE contextBody ;

permissionDecl  = permissionVerb "permissions" NEWLINE permissionBody ;

approvalDecl    = "approval" NEWLINE block ;

permissionVerb  = "allow" | "deny" ;

typeDecl        = "type" Identifier [ typeParams ] "=" typeExpr NEWLINE ;

(* === Bodies === *)

taskBody        = INDENT taskSection { taskSection } DEDENT ;

taskSection     = inputSection | outputSection | stepsSection
				| contextSection | policySection ;

inputSection    = "input" NEWLINE { typedIdent NEWLINE } ;

outputSection   = "output" NEWLINE { typedIdent NEWLINE } ;

stepsSection    = "steps" NEWLINE block ;

contextSection  = "context" NEWLINE { typedIdent NEWLINE } ;

policySection   = "policy" NEWLINE block ;

goalBody        = INDENT successClause { goalClause } DEDENT ;

successClause   = "success" NEWLINE { Identifier [ "." Identifier ] NEWLINE } ;

goalClause      = "workflow" Identifier NEWLINE ;

contextBody     = INDENT { typedIdent NEWLINE } DEDENT ;

permissionBody  = INDENT { permissionEntry NEWLINE } DEDENT ;

permissionEntry = Identifier { "." Identifier } ;

(* === Statements === *)

block           = INDENT { statement } DEDENT ;

statement       = letStmt | constStmt | assignStmt | callStmt
				| ifStmt | matchStmt | loopStmt | forStmt
				| parallelStmt | awaitStmt | whenStmt
				| pipelineStmt | returnStmt | breakStmt
				| continueStmt | attemptStmt | expression NEWLINE
				;

letStmt         = "let" Identifier [ ":" typeExpr ] "=" expression NEWLINE ;

constStmt       = "const" Identifier [ ":" typeExpr ] "=" expression NEWLINE ;

assignStmt      = assignTarget "=" expression NEWLINE ;

assignTarget    = Identifier | qualifiedName | indexExpr | fieldAccess ;

callStmt        = expression NEWLINE ;

returnStmt      = "return" [ expression ] NEWLINE ;

breakStmt       = "break" [ Identifier ] NEWLINE ;

continueStmt    = "continue" [ Identifier ] NEWLINE ;

(* === Control Flow === *)

ifStmt          = "if" expression [ "then" ] NEWLINE block
				{ "else" "if" expression [ "then" ] NEWLINE block }
				[ "else" NEWLINE block ] ;

matchStmt       = "match" expression NEWLINE
				INDENT { matchArm } DEDENT ;

matchArm        = pattern [ "if" expression ] "=>" NEWLINE block ;

loopStmt        = "loop" NEWLINE block ;

forStmt         = "for" pattern "in" expression NEWLINE block ;

parallelStmt    = "parallel" NEWLINE block [ "wait" NEWLINE ] ;

awaitStmt       = "await" expression NEWLINE ;

whenStmt        = "when" NEWLINE conditionBlock "run" NEWLINE block ;

conditionBlock  = INDENT { expression NEWLINE } DEDENT ;

pipelineStmt    = expression NEWLINE { "|>" expression NEWLINE } ;

(* === Error Handling === *)

attemptStmt     = "attempt" NEWLINE block
				{ "recover" NEWLINE recoveryBlock } ;

recoveryBlock   = INDENT { recoveryAction } DEDENT ;

recoveryAction  = "retry" [ expression ] [ "with" "backoff" backoffStrategy ] NEWLINE
				| "fallback" expression NEWLINE
				| "human" [ "escalate" expression ] NEWLINE
				| "abort" NEWLINE
				| "skip" NEWLINE
				;

(* === Expressions === *)

expression      = lambdaExpr ;

lambdaExpr      = [ "fn" paramList "=>" ] pipeExpr ;

pipeExpr        = logicalExpr { "|>" logicalExpr } ;

logicalExpr     = comparisonExpr { ("and" | "or") comparisonExpr } ;

comparisonExpr  = additiveExpr { ("==" | "!=" | "<" | ">" | "<=" | ">=") additiveExpr } ;

additiveExpr    = multiplicativeExpr { ("+" | "-") multiplicativeExpr } ;

multiplicativeExpr = unaryExpr { ("*" | "/" | "%") unaryExpr } ;

unaryExpr       = ( "-" | "not" ) unaryExpr | primaryExpr ;

primaryExpr     = literal | variable | callExpr | indexExpr
				| fieldAccess | parenExpr | listExpr | setExpr
				| mapExpr | queryExpr | interpolatedString
				| promptBlock | askExpr | reasonExpr
				| useExpr | confidenceExpr | approvalExpr
				| ifExpr
				;

ifExpr          = "if" expression "then" expression "else" expression ;

literal         = INT_LIT | FLOAT_LIT | STRING_LIT | BOOL_LIT
				| NULL_LIT | UNKNOWN_LIT ;

variable        = Identifier | qualifiedName ;

callExpr        = expression "(" [ argumentList ] ")" ;

argumentList    = argument { "," argument } [ "," ] ;

argument        = [ Identifier "=" ] expression ;

indexExpr       = expression "[" expression "]" ;

fieldAccess     = expression "." Identifier ;

parenExpr       = "(" expression ")" ;

listExpr        = "[" [ expression { "," expression } [ "," ] ] "]" ;

setExpr         = "{" [ expression { "," expression } [ "," ] ] "}" ;

mapExpr         = "{" [ mapEntry { "," mapEntry } [ "," ] ] "}" ;

mapEntry        = expression ":" expression ;

queryExpr       = querySource NEWLINE { queryOp NEWLINE } ;

querySource     = expression ;
queryOp         = "where" expression
				| "select" expression
				| "order" "by" expression [ "asc" | "desc" ]
				| "group" "by" expression
				| "limit" expression
				;

interpolatedString = STRING_LIT ;

promptBlock     = "prompt" [ Identifier ] NEWLINE block ;

askExpr         = "ask" Identifier ":" NEWLINE block ;

reasonExpr      = "reason" NEWLINE reasonMode NEWLINE ;

reasonMode      = "deep" | "fast" | "none" ;

useExpr         = "use" Identifier NEWLINE ;

confidenceExpr  = expression "confidence" comparisonOp expression ;

approvalExpr    = "ask" "human" ":" NEWLINE block ;

(* === Patterns === *)

pattern         = literal | variablePat | listPat | mapPat
				| constructorPat | wildcardPat ;

variablePat     = Identifier ;

wildcardPat     = "_" ;

listPat         = "[" [ pattern { "," pattern } [ "," ] ] "]" ;

mapPat          = "{" [ mapPatEntry { "," mapPatEntry } [ "," ] ] "}" ;

mapPatEntry     = pattern ":" pattern ;

constructorPat  = Identifier "(" [ pattern { "," pattern } ] ")" ;

(* === Types === *)

typedIdent      = Identifier ":" typeExpr ;

typeExpr        = unionType ;

unionType       = primaryType { "|" primaryType } ;

primaryType     = "Bool" | "Int" | "Float" | "Decimal" | "String"
				| "Bytes" | "Time" | "Duration" | "UUID" | "URI"
				| "Version" | "Agent" | "Task" | "Workflow" | "Goal"
				| "Prompt" | "Memory" | "Knowledge" | "Context"
				| "Tool" | "Model" | "Artifact" | "Result"
				| "Capability" | "Approval" | "Embedding"
				| "Observation" | "Plan" | "Reasoning"
				| "Any" | "Nothing" | "Error"
				| genericType | nullableType | optionalType
				| listType | setType | mapType | tupleType
				| Identifier [ typeArgs ]
				| "(" typeExpr ")" ;

genericType     = Identifier "<" typeArgList ">" ;

typeArgList     = typeArg { "," typeArg } [ "," ] ;

typeArg         = typeExpr ;

nullableType    = typeExpr "?" ;

optionalType    = typeExpr "!" ;

listType        = "List" "<" typeExpr ">" ;

setType         = "Set" "<" typeExpr ">" ;

mapType         = "Map" "<" typeExpr "," typeExpr ">" ;

tupleType       = "(" typeExpr { "," typeExpr } [ "," ] ")" ;

typeArgs        = "<" typeArgList ">" ;

typeParams      = "<" Identifier { "," Identifier } ">" ;

(* === Auxiliary === *)

paramList       = "(" [ typedIdent { "," typedIdent } [ "," ] ] ")" ;

argument        = [ Identifier "=" ] expression ;

identifierList  = Identifier { "," Identifier } [ "," ] ;

modelParams     = "(" parameterList ")" ;

parameterList   = parameter { "," parameter } [ "," ] ;

parameter       = Identifier "=" expression ;

(* === Helpers === *)

comparisonOp    = "==" | "!=" | "<" | ">" | "<=" | ">=" ;
```

---

# Part IV &mdash; Type System

---

## Chapter 16 &mdash; Primitive Types

Lucky's primitive type system is intentionally small. Every primitive type maps to a well-defined representation:

| Type | Description | Example literal | Default value |
|---|---|---|---|
| `Bool` | Boolean | `true`, `false` | `false` |
| `Int` | Signed 64-bit integer | `42`, `-1` | `0` |
| `Float` | IEEE 754 binary64 | `3.14`, `1e9` | `0.0` |
| `Decimal` | Fixed-point decimal for currency and precise computation | `3.14d` | `0.0d` |
| `String` | UTF-8 string | `"hello"` | `""` |
| `Bytes` | Raw byte sequence | `0xDEADBEEF` | `[]` |
| `Time` | UTC timestamp with nanosecond precision | `2026-01-01T00:00:00Z` | epoch |
| `Duration` | Time interval | `30m`, `2h` | `0s` |
| `UUID` | Universally unique identifier | `uuid("...")` | nil UUID |
| `URI` | Uniform resource identifier | `uri("https://...")` | empty |
| `Version` | Semantic version | `version("1.2.3")` | `0.0.0` |

### Type annotations

```lucky
let count: Int = 0
let name: String = "Lucky"
let price: Decimal = 99.99d
let repo: URI = uri("https://github.com/lucky-lang/lucky")
```

### Type constructors

Each primitive type has a constructor function of the same name:

```lucky
Int("42")       # => 42
Float("3.14")   # => 3.14
String(42)      # => "42"
Bool(1)         # => true
```

---

## Chapter 17 &mdash; Composite Types

### List&lt;T&gt;

An ordered, immutable sequence of elements of type `T`.

```lucky
let nums: List<Int> = [1, 2, 3]
let first = nums[0]          # 1
let rest = nums[1..]         # [2, 3]
let bigger = nums.add(4)     # [1, 2, 3, 4]
let mapped = nums.map(fn x => x * 2)  # [2, 4, 6]
```

### Set&lt;T&gt;

An unordered collection of unique elements.

```lucky
let tags: Set<String> = {"rust", "ai", "agent"}
let has = tags.contains("ai")    # true
let merged = tags.union({"ml"})  # {"rust", "ai", "agent", "ml"}
```

### Map&lt;K, V&gt;

An immutable key-value store.

```lucky
let config: Map<String, Any> = {
	"model": "Claude",
	"temperature": 0.7,
}
let model = config["model"]       # "Claude"
let updated = config.insert("max_tokens", 4096)
```

### Queue&lt;T&gt;

A FIFO structure for stream processing.

```lucky
let q: Queue<Int> = Queue.empty()
let q2 = q.enqueue(1).enqueue(2)
let (val, q3) = q2.dequeue()  # val = 1
```

### Graph&lt;T&gt;

A directed or undirected graph. The `Graph` type is used extensively by the IR and execution engine.

```lucky
let g: Graph<String> = Graph.empty()
	.addNode("A")
	.addNode("B")
	.addEdge("A", "B")
```

### Tree&lt;T&gt;

A rooted tree structure for hierarchical data.

### Stream&lt;T&gt;

A lazy, potentially infinite sequence. Streams integrate with pipelines.

```lucky
let s: Stream<Int> = Stream.range(0, 100)
let result = s |> filter(fn x => x % 2 == 0) |> take(5)
```

### Immutability

All composite types are immutable. Methods that appear to mutate return new instances:

```lucky
let a = [1, 2]
let b = a.add(3)   # a == [1, 2], b == [1, 2, 3]
```

---

## Chapter 18 &mdash; Nullable Types

A postfix `?` after a type indicates that the value may be `null`.

```lucky
let maybeName: String? = null
let maybeCount: Int? = 42
```

Nullable types require explicit unwrapping before use. The `match` statement is the canonical way to handle nullable values:

```lucky
match maybeName
	null => "no name"
	name => "hello, \{name}"
```

The `.?` operator chains nullable field access:

```lucky
let city = user.?address.?city    # String? (null if any link is null)
```

The `?|` operator provides a default:

```lucky
let name = maybeName ?| "anonymous"
```

---

## Chapter 19 &mdash; Optional Types

A postfix `!` indicates an optional type. Unlike nullable types (which represent absence), optional types distinguish "not yet provided" from explicit `null`. This distinction is critical for AI outputs and configuration.

```lucky
let temperature: Float! = !        # not set
let temperature: Float! = 0.7      # explicitly set
let temperature: Float! = null     # error: null is not a valid optional value
```

---

## Chapter 20 &mdash; Union Types

A union type `A | B` represents a value that is either of type `A` or of type `B`.

```lucky
let result: Success | Failure = attemptSomething()

match result
	Success(data) => process(data)
	Failure(err)  => log(err)
```

Unions are untagged (structural) by default but may be discriminated by constructor names:

```lucky
type ApiResult = Success { data: String } | Failure { code: Int, message: String }
```

---

## Chapter 21 &mdash; Generic Types

Generics are declared with angle brackets:

```lucky
type Box<T> = { value: T }

type Pair<A, B> = { first: A, second: B }

type Result<Ok, Err> = Success { value: Ok } | Failure { error: Err }
```

Generic constraints use the `where` clause (for declarations) or inline bounds:

```lucky
task Process<T: Agent> where T has tool Browser
	input agent: T
	...
```

Generic type arguments are inferred at call sites when omitted:

```lucky
let box = Box { value: 42 }   # Box<Int> inferred
```

---

## Chapter 22 &mdash; AI Types

Lucky introduces first-class AI types that have no equivalent in traditional programming languages.

| Type | Description |
|---|---|
| `Agent` | A stateful AI entity with memory, tools, and reasoning |
| `Task` | A deterministic, schedulable unit of work |
| `Workflow` | A DAG of tasks and agents |
| `Goal` | A declaration of success criteria |
| `Prompt` | A structured prompt template with validated sections |
| `Memory` | Persistent agent state (vector or structured) |
| `Knowledge` | Structured domain knowledge (RAG-accessible) |
| `Context` | Ambient execution state |
| `Tool` | A capability interface to an external system |
| `Model` | An LLM backend descriptor |
| `Reasoning` | A reasoning strategy specification |
| `Observation` | A perceptual input from the environment |
| `Artifact` | An immutable execution output |
| `Plan` | A generated execution plan |
| `Result` | A task outcome with confidence |
| `Capability` | A named permission or tool right |
| `Approval` | A pending human decision |
| `Embedding` | A vector representation of text or data |

These types are parametric where applicable:

```lucky
Agent<Research>     # an agent specialized in research
Task<Review>        # a review task
Workflow<CI>        # a CI workflow
Goal<Deploy>        # a deployment goal
Prompt<CodeReview>  # a code-review prompt
```

AI types carry runtime metadata such as model binding, cost estimates, and confidence thresholds. The compiler can reason about these types to perform optimizations like model routing and caching.

---

## Chapter 23 &mdash; Context Types

A `Context` is an implicit, lexically-scoped key-value map that propagates from workflows into agents and tasks. Context entries are declared at the workflow or task level:

```lucky
context
	user: String
	repo: URI
	session: UUID
	history: List<Message>
```

Tasks automatically inherit context from their enclosing workflow. A task may shadow a context entry with a more specific type:

```lucky
task GenerateReport
	context
		repo: URI          # inherited, narrowed if desired
		template: String   # additional context for this task
```

Context is read-only within a task. Mutations to context are performed at the workflow level and create a new scope for downstream tasks.

---

## Chapter 24 &mdash; Type Inference

Lucky uses local type inference. The compiler infers types in the following positions:

* **Variable declarations**: `let x = 42` infers `x: Int`
* **Lambda parameters** (when unambiguous from context): `list.map(fn x => x * 2)`
* **Generic arguments** at call sites: `identity(42)` infers `identity<Int>(42)`
* **Pipeline intermediates**: the output type of each stage is inferred

Top-level declarations and task inputs/outputs require explicit type annotations:

```lucky
task Process
	input
		data: List<String>     # required
	output
		result: Map<String, Int>  # required
```

The inference algorithm is based on Hindley-Milner with extensions for union types and AI types.

---

## Chapter 25 &mdash; Type Compatibility

### Nominal vs structural

Named types (agents, tasks, workflows) use **nominal** compatibility: two types are compatible only if they refer to the same declaration.

Record and union types (those declared with `type ... =`) use **structural** compatibility: two types are compatible if their shapes match.

```lucky
# nominal: not compatible despite identical shape
agent Reviewer1
	...
agent Reviewer2
	...
let r: Reviewer1 = Reviewer2.new()   # error

# structural: compatible
type Point = { x: Int, y: Int }
type Coord = { x: Int, y: Int }
let p: Point = Coord { x: 1, y: 2 }  # ok
```

### Subtyping

Lucky has limited subtyping:

* `T` is a subtype of `T?` (non-nullable to nullable)
* `T` is a subtype of `T | U` (union introduction)
* `never` (the uninhabited type) is a subtype of all types

### Confidence subtyping

An AI output of type `Report` with confidence `0.9` is a subtype of `Report` with threshold `0.7`. The compiler tracks confidence as an overlay on the type lattice.

---

## Chapter 26 &mdash; Lifetime Rules

Lucky's ownership model is designed for an orchestration language &mdash; more permissive than Rust, more predictable than garbage-collected languages.

### Ownership

* **Primitives** are copy types (trivially copyable).
* **Composite types** (List, Map, etc.) are reference-counted immutable structures.
* **Agent state** is owned by the agent and persists across task invocations.
* **Artifacts** are owned by the workflow that produced them and are garbage-collected when the workflow completes (or archived if persistent).

### Borrowing

There are no explicit borrow annotations. Because all composite types are immutable and agents own their state, there is no need for borrow checking at the language level.

### Task lifetimes

A task's input values are valid for the duration of the task. A task's output values are valid after the task completes and remain valid until the owning workflow terminates.

### Context lifetime

Context values are valid from the point of declaration to the end of the enclosing scope (workflow, task, or block).

---

# Part V &mdash; Expressions

---

## Chapter 27 &mdash; Literals

Literal expressions are described in detail in Chapter 10. In expression context, every literal evaluates to a value of its corresponding type.

```lucky
42                  # Int
3.14                # Float
true                # Bool
"hello"             # String
null                # null (inhabits all nullable types)
unknown             # unknown (inhabits all optional types)
[1, 2, 3]           # List<Int>
{"a": 1}            # Map<String, Int>
```

Duration and time literals use postfix notation:

```lucky
30s                 # Duration (30 seconds)
5m                  # Duration (5 minutes)
2h                  # Duration (2 hours)
1d                  # Duration (1 day)
```

---

## Chapter 28 &mdash; Variables

### Immutable variables (`let`)

```lucky
let name = "Lucky"
let count: Int = 42
```

`let` bindings are immutable &mdash; they cannot be reassigned. This is the default and preferred binding form.

### Constants (`const`)

```lucky
const MAX_RETRIES = 3
const API_URL: URI = uri("https://api.example.com")
```

Constants must be evaluable at compile time. They can be used in type positions (e.g., array sizes) and pattern matching.

### Shadowing

A `let` or `const` in an inner scope may shadow an outer binding:

```lucky
let x = 1
if condition
	let x = 2   # shadows outer x within this block
```

### Mutable state

Mutable state is confined to agents. An agent's memory provides mutable storage with controlled access:

```lucky
agent Counter
	memory
		value: Int = 0

	task Increment
		steps
			memory.value = memory.value + 1
```

---

## Chapter 29 &mdash; Assignment

Assignment (`=`) is only valid for:

* **Agent memory fields**: `memory.field = expression`
* **Mutable collection wrappers**: `mut_list[index] = value` (where `mut_list` is a `MutList<T>` wrapper around an agent-owned list)

Variable bindings (`let`) are never reassignable. This simplifies data-flow analysis and checkpointing.

```lucky
agent StateManager
	memory
		counter: Int = 0
		cache: Map<String, String> = {}

	task Update
		steps
			memory.counter = memory.counter + 1
			memory.cache = memory.cache.insert("key", "value")
```

---

## Chapter 30 &mdash; Operators

### Arithmetic

| Operator | Description | Types |
|---|---|---|
| `+` | Addition / concatenation | Int, Float, Decimal, String, List |
| `-` | Subtraction | Int, Float, Decimal |
| `*` | Multiplication | Int, Float, Decimal |
| `/` | Division | Int→Float, Float, Decimal |
| `%` | Remainder | Int |

### Comparison

| Operator | Description |
|---|---|
| `==` | Structural equality |
| `!=` | Inequality |
| `<` | Less than |
| `>` | Greater than |
| `<=` | Less than or equal |
| `>=` | Greater than or equal |

Equality comparison (`==`) works across all types. For AI-output types, equality includes confidence-aware comparison.

### Logical

| Operator | Description |
|---|---|
| `and` | Logical AND (short-circuit) |
| `or` | Logical OR (short-circuit) |
| `not` | Logical NOT |

### Pipeline

| Operator | Description |
|---|---|
| `\|>` | Pipe output to next stage |

### Nullable operators

| Operator | Description |
|---|---|
| `.?` | Nullable field access |
| `?\|` | Null-coalescing default |
| `?[` | Nullable indexing |

### Precedence (lowest to highest)

```
|>
or
and
==  !=  <  >  <=  >=
+  -
*  /  %
not  - (unary)
.?  ?|  ?[
.  ()  []
```

---

## Chapter 31 &mdash; Pipelines

The pipeline operator `|>` threads the output of one expression as the input to the next. It is the primary data-flow mechanism in Lucky.

```lucky
files
	|> filter *.py
	|> summarize
	|> save report.md
```

Each stage after `|>` must be a function, task, or operation that accepts a single input or is curried:

```lucky
users
	|> where age > 18
	|> select name
	|> sort name asc
	|> take 10
```

Pipelines integrate with the web and AI operations natively:

```lucky
web.search "AI Agent Framework"
	|> extract
	|> rank relevance
	|> answer
```

The compiler infers intermediate types through the pipeline. A pipeline is desugared into nested function applications:

```lucky
x |> f |> g      →    g(f(x))
```

---

## Chapter 32 &mdash; Pattern Matching

Pattern matching is a core expression form in Lucky. The `match` expression dispatches on value structure.

### Simple patterns

```lucky
match value
	0 => "zero"
	1 => "one"
	n => "other: \{n}"
```

### Destructuring

```lucky
match point
	{ x: 0, y: 0 } => "origin"
	{ x, y }       => "x=\{x}, y=\{y}"
```

### List patterns

```lucky
match items
	[]        => "empty"
	[first]   => "single: \{first}"
	[first, ..rest] => "first=\{first}, rest=\{rest}"
```

### Guard clauses

```lucky
match result
	Success(data) if data.valid => process(data)
	Success(data)               => log("invalid data")
	Failure(err)                => report(err)
```

### Smart enum destructuring

```lucky
match apiResult
	Success { data, meta } => ...
	Failure { code, message } if code > 500 => ...
	Failure { message } => ...
```

### Exhaustiveness checking

The compiler verifies that `match` arms are exhaustive. For open unions (those not declared as a closed set), a wildcard arm `_` is required.

---

## Chapter 33 &mdash; Lambda Expressions

Lambda expressions (anonymous functions) use the `fn` keyword:

```lucky
fn x => x * 2
fn (x, y) => x + y
fn x => { let tmp = process(x); tmp * 2 }
```

Type annotations on parameters are optional when inferable:

```lucky
let double = fn (x: Int): Int => x * 2
```

Lambdas close over their lexical scope (immutable captures only):

```lucky
let factor = 10
let scale = fn x => x * factor
```

Lambdas are commonly used with collection methods and pipelines:

```lucky
numbers
	|> map(fn x => x * 2)
	|> filter(fn x => x > 10)
```

---

## Chapter 34 &mdash; Query Expressions

Query expressions provide a declarative syntax for filtering, transforming, and aggregating data. They are inspired by SQL and LINQ but designed for readability in a whitespace-significant language.

```lucky
users
	where age > 18
	where country == "US"
	select { name, email }
	order by name asc
```

Queries can be assigned:

```lucky
let adults = users
	where age >= 18
	select name

let stats = sales
	group by category
	select { category, total: sum(amount) }
```

Query operators:

| Operator | Description |
|---|---|
| `where` | Filter by predicate |
| `select` | Project/transform each element |
| `order by` | Sort (default ascending; add `desc` for descending) |
| `group by` | Group by key expression |
| `limit` | Take the first N results |
| `skip` | Skip the first N results |

Query expressions desugar into method calls on `Queryable<T>` and integrate with pipeline syntax.

---

# Part VI &mdash; Statements

---

## Chapter 35 &mdash; Blocks

A block is a sequence of statements at a common indentation level. Every block introduces a new lexical scope.

```lucky
let x = 1
let y =
	let inner = 2
	inner + 1       # last expression is the block's value
```

The value of a block is the value of its last expression. Empty blocks evaluate to `null`.

Blocks are used as bodies for control-flow constructs, task steps, and agent/workflow definitions. The indentation increase (`INDENT` token) and decrease (`DEDENT` token) delimit block boundaries.

---

## Chapter 36 &mdash; If

The `if` statement conditionally executes a block.

```lucky
if confidence > 0.9
	approve
else if confidence > 0.5
	request_human_review
else
	reject
```

`if` is an expression: it evaluates to the value of the executed block.

```lucky
let status = if passed then "ok" else "failed"
```

The `then` keyword is optional when the consequence is on a new indented line.

---

## Chapter 37 &mdash; Match

The `match` statement is the primary branching construct. Unlike `if`, it destructures values and checks exhaustiveness.

```lucky
match task_result
	Success { output: report } =>
		publish(report)
	Failure { error: err, recoverable: true } =>
		retry(err)
	Failure { error: err } =>
		escalate(err)
```

`match` is also an expression:

```lucky
let action = match state
	Ready    => "execute"
	Running  => "wait"
	Failed   => "recover"
	Complete => "archive"
```

---

## Chapter 38 &mdash; Loop

Lucky provides two looping constructs: `loop` and `for`.

### `loop` &mdash; infinite loop

```lucky
loop
	let msg = poll()
	if msg == null
		break
	process(msg)
```

### `for` &mdash; iteration

```lucky
for item in items
	process(item)

for i in 0..10
	log(i)
```

Loops are expressions that evaluate to `null` unless terminated by a `break` with a value:

```lucky
let found = for item in items
	if item.matches(criteria)
		break item
```

Cycles in the execution graph require explicit `loop` syntax. The compiler rejects implicit cycles in workflow definitions.

---

## Chapter 39 &mdash; Parallel

The `parallel` block declares that contained statements may execute concurrently.

```lucky
parallel
	Researcher.search("topic A")
	Architect.design("system B")
	Security.audit("component C")
wait
```

The `wait` keyword (optional) blocks until all parallel branches complete. Without `wait`, the parallel block fires-and-forgets; the workflow continues immediately.

Branches within `parallel` are independent by default. If one branch depends on another's output, use `await`:

```lucky
parallel
	let data = Researcher.search("topic")
	let report = await ReportWriter.write(data)   # depends on data
	Security.audit("topic")                        # independent
wait
```

The runtime schedules parallel branches onto available execution slots, respecting resource limits.

---

## Chapter 40 &mdash; Await

`await` suspends the current execution until the given expression completes.

```lucky
let result = await Agent.task(input)
```

`await` is required when:
* Calling a task from within another task
* Accessing the result of an asynchronous agent invocation
* Waiting for a human approval

`await` is not needed at the workflow level, where ordering is expressed through graph edges (`->`).

---

## Chapter 41 &mdash; Return

`return` exits the enclosing task or function with an optional value.

```lucky
task Compute
	input x: Int
	output result: Int
	steps
		if x < 0
			return 0
		return x * 2
```

In a task, `return` sets the task's output. Multiple `return` statements are permitted; all must produce values compatible with the declared output type.

In a lambda, `return` exits the lambda body.

---

## Chapter 42 &mdash; Break

`break` exits the innermost enclosing loop. An optional label allows breaking from an outer loop:

```lucky
outer: for batch in batches
	for item in batch
		if item.isDone()
			break outer
```

`break` may carry a value that becomes the loop's result:

```lucky
let result = for item in items
	if item.isTarget()
		break item
```

---

## Chapter 43 &mdash; Continue

`continue` skips the rest of the current loop iteration and proceeds to the next.

```lucky
for item in items
	if item.isSkipped()
		continue
	process(item)
```

A label may specify which enclosing loop to continue:

```lucky
outer: for batch in batches
	for item in batch
		if item.isTrivial()
			continue outer
```

---

# Part VII &mdash; AI Programming Model

---

## Chapter 44 &mdash; Goals

A `goal` is the highest-level declaration in a Lucky program. It defines **what success means** without prescribing implementation.

```lucky
goal BuildWebsite
	success
		website.online
		website.tested
		website.documented
	workflow MainWorkflow
```

### Success criteria

Success criteria are predicate expressions that the runtime verifies at the conclusion of execution:

```lucky
goal DeployService
	success
		service.healthy
		service.latency < 100ms
		service.uptime > 99.9
```

### Multiple workflows

A goal may list multiple workflows. The runtime selects among them based on context, cost, or explicit policy:

```lucky
goal GenerateReport
	workflow FastReport      # quick, lower quality
	workflow ThoroughReport  # slow, higher quality
```

The runtime chooses `ThoroughReport` when `context.priority == "quality"` and `FastReport` otherwise, unless overridden by a policy.

### Goal lifecycle

```
Created → Planning → Executing → Verifying → Completed
								↓
								Failed (retry or escalate)
```

---

## Chapter 45 &mdash; Tasks

A `task` replaces the traditional function. It is the smallest schedulable, checkpointable unit of work.

```lucky
task ReviewCode
	input
		repo: URI
		files: List<Path>
	output
		report: ReviewReport
	context
		coding_standards: Document
	policy
		retry 2
		timeout 10m
	steps
		clone repo
		for file in files
			review file
		generate report
```

### Task purity

By default, tasks are pure (no side effects beyond their declared outputs). A task may be explicitly declared stateful to access agent memory:

```lucky
task UpdateCache(stateful)
	input key: String, value: String
	steps
		memory.cache = memory.cache.insert(key, value)
```

### Task lifecycle

```
Created → Ready → Running → Checkpointed → Completed
							↓
						Waiting (on dependency or approval)
							↓
						Failed → Recovering → Completed
								↓
								Cancelled
```

### Task composition

Tasks compose through workflows, not through direct calls. If task A needs task B's output, the workflow graph declares that edge:

```lucky
workflow Build
	Analyze
		->
	Generate  # receives Analyze.output implicitly via context
```

Within a task, imperative code may call sub-tasks via `await`:

```lucky
let analysis = await Analyze.run(repo)
```

---

## Chapter 46 &mdash; Agents

An `agent` is the central abstraction in Lucky &mdash; the equivalent of a class in object-oriented languages, but designed around AI-native capabilities.

```lucky
agent SecurityReviewer
	model Claude
	memory
		findings: List<Finding> = []
		patterns: Knowledge = Knowledge.empty()
	tools
		Git
		Browser
		Shell
	permissions
		allow filesystem.read
		deny filesystem.write
	policy
		retry 3
		timeout 30m
	prompt ReviewerPrompt
```

### Agent components

| Component | Purpose |
|---|---|
| `model` | The LLM backend this agent uses |
| `memory` | Persistent state (vector + structured) |
| `tools` | Capability interfaces available to the agent |
| `permissions` | Security boundaries |
| `policy` | Execution behavior (retry, timeout, etc.) |
| `prompt` | The agent's system prompt template |

### Agent methods

Agents expose tasks as methods:

```lucky
SecurityReviewer.review(repo)
SecurityReviewer.explain(finding)
```

### Agent instantiation

```lucky
let reviewer = SecurityReviewer.new(
	model = GPT,
	memory = ProjectMemory,
)
```

Agent instances can be configured per-use with overrides.

### Swarm instantiation

Multiple instances of an agent can run in parallel:

```lucky
swarm 20 Reviewer.review_batch(files)
```

---

## Chapter 47 &mdash; Workflows

A `workflow` orchestrates agents and tasks into a directed acyclic graph.

### Sequential workflow

```lucky
workflow BuildAndDeploy
	Research
		->
	Design
		->
	Implement
		->
	Review
		->
	Test
		->
	Deploy
```

Arrows (`->`) indicate sequential dependency. Each node waits for all upstream nodes to complete before executing.

### Parallel workflow

Nodes at the same indentation level, without arrows between them, execute in parallel:

```lucky
workflow SecurityAudit
	StaticAnalysis
	DependencyScan
	SecretDetection
	ComplianceCheck
```

All four branches start simultaneously. The workflow completes when all branches finish.

### Mixed workflow

```lucky
workflow FullPipeline
	Research
		->
	parallel
		Design
		SecurityReview
	wait
		->
	Implement
		->
	Test
		->
	Deploy
```

### Conditional workflow

```lucky
workflow ConditionalBuild
	Analyze
		->
	if result.needsBuild
		Build
			->
		Deploy
	else
		Skip
```

### Workflow as value

A workflow is a value of type `Workflow<T>` and can be passed to other workflows:

```lucky
workflow SubProcess
	StepA -> StepB

workflow MainProcess
	PreProcess
		->
	SubProcess      # embedded
		->
	PostProcess
```

---

## Chapter 48 &mdash; Models

A `model` declaration names an LLM backend and configures its parameters.

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

model Local(
	provider = "ollama",
	version = "llama3",
)
```

### Model selection

The `use` directive sets the default model for the current scope:

```lucky
use Claude

# all agent invocations in this scope use Claude

agent Researcher
	use GPT     # override for this agent only
```

### Model routing

The runtime may route requests to different models based on task complexity, cost constraints, or confidence requirements. A policy can express routing rules:

```lucky
policy ModelRouter
	if task.complexity > 0.8
		use Claude
	else if task.cost_budget < 0.01
		use Local
	else
		use GPT
```

---

## Chapter 49 &mdash; Prompts

A `prompt` is a structured template &mdash; not a raw string. The compiler validates its sections.

```lucky
prompt CodeReviewer
	role
		You are a senior software engineer reviewing code
		for correctness, security, and performance.
	rules
		- Report only actionable findings.
		- Cite specific line numbers.
		- Classify severity as low, medium, or high.
	context
		- Language: {language}
		- Framework: {framework}
	examples
		input:
			```python
			def foo():
				return eval(user_input)
			```
		output:
			severity: high
			finding: Use of eval() with user input is a security risk.
			recommendation: Use ast.literal_eval() or a safe parser.
	format
		Return a structured report with sections:
		summary, findings[], recommendations[].
```

### Prompt sections

| Section | Required | Description |
|---|---|---|
| `role` | Yes | The system persona |
| `rules` | No | Behavioral constraints |
| `context` | No | Dynamic context injected at runtime |
| `examples` | No | Few-shot examples |
| `format` | No | Output format specification |

### Prompt invocation

```lucky
ask Claude:
	Review this code for security issues.
```

Inline prompt blocks (shorthand):

```lucky
let summary = ask GPT:
	Summarize the following document in 3 bullet points:
	{document}
```

---

## Chapter 50 &mdash; Memory

Memory provides persistent, queryable storage for agents. Lucky abstracts over the underlying storage backend (in-memory, vector database, relational database).

```lucky
memory ProjectMemory
	scope project
	backend vector
	dimensions 1536
```

### Memory scopes

| Scope | Lifetime |
|---|---|
| `local` | Duration of a single task |
| `session` | Duration of a user session |
| `project` | Lifetime of the project |
| `organization` | Shared across projects in an org |
| `global` | Shared globally (carefully scoped) |

### Memory operations

```lucky
agent Planner
	memory ProjectMemory

	task Plan
		steps
			memory.remember("architecture", architectureDoc)
			let docs = memory.recall("how to structure APIs")
			let related = memory.similar(architectureDoc, limit = 5)
			memory.forget(oldDecision)
```

### Memory as context

Memory entries propagate into task context automatically:

```lucky
task GenerateCode
	context
		coding_style: Style    # inherited from agent memory
```

---

## Chapter 51 &mdash; Knowledge

Knowledge represents structured domain information that agents can query. Unlike memory (which is agent-specific and experiential), knowledge is shared reference material.

```lucky
knowledge CompanyDocs
	source "./docs/**/*.md"
	source "./wiki/**/*.md"
	source uri("https://docs.internal.company.com")
	chunk_size 1024
	chunk_overlap 128
```

### Knowledge operations

```lucky
let relevant = CompanyDocs.search("deployment process", top_k = 5)
let answer = CompanyDocs.ask("What is our SLA for production outages?")
```

Knowledge is RAG-accessible by default. The runtime manages embedding, chunking, and retrieval transparently.

---

## Chapter 52 &mdash; Context

Context is implicit, lexically-scoped execution state that propagates from workflows into agents and tasks. It eliminates manual dependency injection.

```lucky
workflow MainFlow
	context
		user: User
		repo: URI
		config: Config
		history: List<Message>

	Research
		->
	Plan
		->
	Execute
```

Every task within `MainFlow` can access `context.user`, `context.repo`, etc. without explicit parameter passing.

### Context layering

Context is layered: a workflow's context is visible to all its agents; an agent's context is visible to all its tasks. A task can add context entries visible only to its own steps:

```lucky
task Analyze
	context
		analysis_mode: String = "fast"   # scoped to this task
	steps
		# can access context.user (inherited) and context.analysis_mode (local)
```

### Context at runtime

The runtime snapshot of context is part of every checkpoint. Resuming from a checkpoint restores the full context.

---

## Chapter 53 &mdash; Tools

A `tool` is a capability interface to an external system. Tools are declared, configured, and invoked through the language.

```lucky
tool Git(
	workdir = "./repo",
)

tool Browser(
	headless = true,
	timeout = 30s,
)

tool Shell(
	allowed_commands = ["ls", "cat", "grep", "find"],
)

tool HTTP(
	base_url = "https://api.example.com",
	auth = "bearer",
)
```

### Tool invocation

```lucky
Git.clone(repo)
Git.commit("fix: resolve null pointer")
Git.push()

Browser.navigate("https://example.com")
Browser.screenshot()
Browser.extract("article")

Shell.exec("cargo build")

HTTP.get("/users")
HTTP.post("/tasks", body)
```

### Tool contracts

Tools have typed inputs and outputs that the compiler checks:

```lucky
tool Database
	fn query(sql: String): List<Row>
	fn execute(sql: String): Int
```

### Custom tools

Users can define custom tools as tasks and register them:

```lucky
task CustomAnalyzer
	input file: Path
	output metrics: Metrics
	...

tool Analysis
	uses CustomAnalyzer
```

---

## Chapter 54 &mdash; Permissions

Permissions enforce capability security. Every agent runs with an explicit, least-privilege permission set.

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

### Permission inheritance

Permissions are lexical: an agent inherits permissions from its enclosing scope (workflow or project). An agent may further restrict but never expand its inherited permissions.

```lucky
project MyProject
	permissions
		allow filesystem.read

	agent Reader
		# inherits filesystem.read

	agent RestrictedReader
		permissions
			deny filesystem.read(/secrets/*)
		# has filesystem.read EXCEPT for /secrets/*
```

### Permission checking

Permission violations are caught at compile time when statically analyzable and at runtime otherwise. A denied operation produces a `PermissionError` that can be handled by recovery policies.

---

## Chapter 55 &mdash; Policies

Policies configure execution behavior. They govern retry, timeout, checkpointing, caching, sandboxing, and model routing.

```lucky
policy ProductionPolicy
	retry 3 with backoff exponential
	timeout 1h
	checkpoint after each task
	cache ttl 24h
	sandbox enabled
	model Claude
	cost_limit 50.00 USD
```

### Policy attachment

Policies attach to goals, workflows, agents, or individual tasks:

```lucky
goal BuildWebsite
	policy FastPolicy

agent Reviewer
	policy ThoroughPolicy

task CriticalDeploy
	policy ProductionPolicy
```

### Policy resolution

When multiple policies apply, the most specific wins: task-level overrides agent-level, which overrides workflow-level, which overrides goal-level. Conflicting settings at the same level produce a compile-time error.

---

## Chapter 56 &mdash; Human Approval

Human approval is a first-class language construct that gates execution on human judgment.

### Approval gates

```lucky
approval
	before deploy
	before filesystem.delete(/production/*)
	before git.push(main)
```

When execution reaches an approval gate, the runtime suspends and notifies the designated human approver. Execution resumes after approval (or terminates after rejection).

### Inline human queries

```lucky
ask human:
	Delete production database "orders_db"? This action is irreversible.
	[yes / no]
```

The `ask human` construct presents a structured question. The human response is typed and can be used in control flow:

```lucky
let confirmed = ask human:
	Deploy version {version} to production?
	Changes: {changelog}

if confirmed
	deploy
else
	abort
```

### Approval timeout

```lucky
approval
	before deploy
	timeout 4h escalate to manager
```

If approval does not arrive within the timeout, the runtime escalates according to the specified escalation path.

---

# Part VIII &mdash; Runtime

---

## Chapter 57 &mdash; Execution Graph

The central data structure of the Lucky runtime is the **Execution DAG** &mdash; a directed acyclic graph where nodes are computation units and edges are dependencies.

### Graph construction

The compiler translates workflow definitions into a DAG:

```lucky
workflow Build
	Research
		->
	Plan
		->
	parallel
		Implement
		Test
	wait
		->
	Deploy
```

becomes:

```
[Research] → [Plan] → [Implement] → [Deploy]
						↘
						[Test] ↗
```

### Node types

| Node Type | IR Kind | Description |
|---|---|---|
| `GoalNode` | `goal` | Root node declaring success criteria |
| `WorkflowNode` | `workflow` | Named subgraph of agents and tasks |
| `TaskNode` | `task` | Executes a task with given inputs |
| `AgentInvokeNode` | `agent_invoke` | Invokes an agent method on a model backend |
| `ToolNode` | `tool` | Calls an external tool |
| `LLMCallNode` | `llm_call` | Direct LLM prompt invocation |
| `DecisionNode` | `decision` | Branches on a condition |
| `MatchNode` | `match` | Multi-way pattern-match branch |
| `ParallelNode` | `parallel` | Fan-out to multiple children |
| `JoinNode` | `join` | Fan-in from multiple parents; synchronization barrier |
| `LoopNode` | `loop` | Bounded iterative execution |
| `PipelineNode` | `pipeline` | Data-flow pipeline of operations |
| `AttemptNode` | `attempt` | Error handling with recovery chain |
| `ApprovalNode` | `approval` | Suspends for human approval |

### Graph properties

* **Acyclic** &mdash; the compiler rejects cycles (loops are explicit, unrolled, or bounded)
* **Typed edges** &mdash; edges carry type information for data flow
* **Weighted** &mdash; nodes have estimated cost and duration weights for scheduling

### Graph serialization

The execution graph is serialized as Lucky IR (`.lir`), a portable JSON-based format:

```json
{
  "version": "0.1",
  "nodes": [...],
  "edges": [...],
  "context": {...},
  "policies": {...}
}
```

---

## Chapter 58 &mdash; Scheduling

The runtime scheduler traverses the execution DAG and dispatches ready nodes to available execution slots.

### Scheduling algorithm

1. Mark all nodes with no incoming edges as **ready**.
2. From the ready set, select nodes according to priority and resource availability.
3. Dispatch selected nodes to the execution engine (LLM backend or local executor).
4. When a node completes, mark its outgoing edges as satisfied.
5. If a newly unblocked node has all incoming edges satisfied, add it to the ready set.
6. Repeat until all nodes complete or a terminal failure occurs.

### Priority

Nodes carry a priority derived from:

* Explicit `priority` policy setting
* Dependency chain depth (nodes on the critical path get higher priority)
* Cost budget allocation

### Resource constraints

The scheduler respects:

* **Concurrency limit**: maximum simultaneous LLM calls
* **Rate limit**: calls per minute per model provider
* **Cost budget**: maximum spend per workflow
* **Memory limit**: maximum context window utilization

### Backend dispatch

The scheduler delegates node execution to the appropriate backend adapter (Claude Code, Codex CLI, OpenCode, or the standalone Lucky engine). The adapter is selected per-node based on the model declaration and available backends.

---

## Chapter 59 &mdash; Dependency Analysis

The compiler performs static dependency analysis to determine execution order and parallelism opportunities.

### Data dependencies

A task B depends on task A if B reads a value that A produces. The compiler tracks this through context propagation and explicit `await` calls:

```lucky
task A
	output data: Data

task B
	input source: Data     # depends on A.output.data
```

### Resource dependencies

Tasks may declare exclusive resource requirements:

```lucky
task Deploy
	resource database: exclusive
```

Tasks requiring the same exclusive resource serialize; tasks requiring shared resources may run concurrently.

### Dependency graph optimization

The compiler performs these optimizations on the dependency graph:

* **Dead node elimination**: remove nodes whose outputs are unused
* **Diamond resolution**: detect and preserve shared dependencies
* **Parallelization**: identify independent subgraphs for concurrent scheduling

---

## Chapter 60 &mdash; Checkpoints

The runtime checkpoints task state at configurable intervals, enabling resumption after interruption.

### What is checkpointed

* **Task inputs and outputs** (immutable values)
* **Agent memory** (mutable state snapshot)
* **Context** (ambient state)
* **Execution position** (which step within a task)

### Checkpoint policy

```lucky
policy
	checkpoint after each task       # fine-grained
	checkpoint after each workflow   # coarse-grained
	checkpoint interval 5m           # time-based
```

### Checkpoint storage

Checkpoints are stored in a configurable backend (local filesystem, cloud storage, or database). Each checkpoint is identified by a UUID and linked to its parent for recovery chains.

---

## Chapter 61 &mdash; Recovery

When a task fails, the runtime consults the recovery policy to determine the next action.

### Recovery actions

```lucky
attempt
	deploy
recover
	retry
recover
	fallback deploy_staging
recover
	human escalate "Deployment failed after 3 retries"
```

| Action | Description |
|---|---|
| `retry` | Re-execute the task (with optional count and backoff) |
| `fallback` | Execute an alternative task |
| `human` | Escalate to a human operator |
| `abort` | Terminate the workflow |
| `skip` | Skip the failed task and continue |

### Recovery chains

Multiple recovery blocks define a chain tried in order:

```lucky
attempt
	risky_operation
recover
	retry 3
recover
	fallback safe_alternative
recover
	human
```

### Partial recovery

For tasks with multiple steps, recovery can resume from the last checkpoint rather than restarting:

```lucky
policy
	checkpoint after each step
	recovery resume_from_checkpoint
```

---

## Chapter 62 &mdash; Transactions

Lucky provides transactional semantics for sequences of operations that must succeed or fail atomically.

```lucky
transaction
	Git.commit("step 1")
	Database.migrate()
	Config.update()
```

If any step fails, all preceding steps are rolled back (using their declared inverse operations).

### Compensating actions

Tasks may declare compensating actions for rollback:

```lucky
task DeployService
	input version: Version
	output endpoint: URI
	rollback
		undeploy version
	steps
		...
```

The runtime calls `rollback` tasks in reverse order when a transaction fails.

### Transaction isolation

Transactions run with snapshot isolation: they see a consistent snapshot of agent memory and context at transaction start.

---

## Chapter 63 &mdash; State Management

The runtime manages state across five levels:

| Level | Scope | Mutability | Persistence |
|---|---|---|---|
| **Task-local** | Within a task | Immutable bindings + mutable agent memory | None (recomputed) |
| **Agent memory** | Within an agent | Mutable (controlled) | Checkpointed |
| **Context** | Within a scope | Read-only | Checkpointed |
| **Workflow state** | Within a workflow | Transitional (DAG progress) | Checkpointed |
| **Project state** | Within a project | Configuration | Configuration file |

### State transitions

State transitions follow a deterministic state machine. The runtime tracks the current state of every task, agent, and workflow. This enables:

* **Observability**: query current state of any component
* **Debugging**: replay from any checkpoint
* **Auditability**: record of all state transitions

---

## Chapter 64 &mdash; Cost Optimization

The runtime optimizes for cost across multiple dimensions.

### Optimization strategies

* **Model routing**: simple queries go to cheaper/faster models
* **Caching**: identical prompts reuse cached responses
* **Batching**: multiple queries to the same model are batched where provider APIs support it
* **Speculative execution**: cheap checks run before expensive LLM calls
* **Confidence gating**: low-confidence outputs trigger additional computation only when necessary

### Cost policy

```lucky
policy
	cost_limit 10.00 USD per workflow
	prefer_cheapest_model true
	cache ttl 1h
	max_tokens_per_call 4096
```

### Cost tracking

The runtime reports cost in real time and can halt execution when the budget is exceeded:

```lucky
policy
	cost_limit 5.00 USD
	on_budget_exceeded abort
```

---

# Part IX &mdash; Concurrency

---

## Chapter 65 &mdash; Task Parallelism

Independent tasks within a workflow execute concurrently by default.

```lucky
workflow BuildReport
	FetchData           # starts immediately
	ComputeStats        # starts immediately (parallel with FetchData)
	GenerateCharts      # starts immediately
	wait                # optional explicit barrier
		->
	AssembleReport      # waits for all above
```

### Parallelism limits

The maximum concurrent tasks are bounded by:

* `max_concurrency` in the runtime configuration
* Model provider rate limits
* Resource declarations (`resource X: exclusive`)

### Fan-out

```lucky
swarm 50 Analyzer.process_file(files)
```

Each file spawns an independent task instance. The runtime schedules them across available slots.

---

## Chapter 66 &mdash; Agent Parallelism

Multiple agents can operate concurrently on independent work.

```lucky
parallel
	Researcher.investigate("topic A")
	Architect.design("component B")
	SecurityReviewer.audit("system C")
wait
```

### Agent-to-agent communication

Agents communicate through shared memory, context, or explicit message passing:

```lucky
agent Producer
	task Generate
		output data: Data
		steps
			context.channel.send(data)

agent Consumer
	task Process
		steps
			let data = context.channel.receive()
			process(data)
```

### Agent swarms

Swarm execution instantiates many copies of an agent:

```lucky
swarm 20 Reviewer.review_patch(patches)
```

The runtime distributes work across instances and collects results.

---

## Chapter 67 &mdash; Synchronization

### Barriers

The `wait` keyword acts as a barrier, blocking until all preceding parallel branches complete:

```lucky
parallel
	branch_a
	branch_b
wait
	# both complete here
```

### Channels

Typed channels enable communication between concurrent tasks:

```lucky
let chan: Channel<Message> = Channel.new(buffer = 10)

parallel
	chan.send(msg)
	let received = chan.receive()
wait
```

### Mutex (agent memory only)

Agent memory fields support atomic compare-and-swap for safe concurrent updates:

```lucky
memory.counter.compare_and_swap(expected: 0, new: 1)
```

### Deadlock detection

The runtime detects cycles in the wait-for graph and reports deadlocks at runtime (statically when possible).

---

## Chapter 68 &mdash; Streams

Streams are lazy, potentially unbounded sequences that integrate with pipelines and concurrent processing.

```lucky
let stream: Stream<Event> = EventSource.subscribe("deployments")

stream
	|> filter(fn e => e.status == "failed")
	|> map(fn e => e.service)
	|> for_each(fn service => alert(service))
```

### Stream sources

* **Event subscriptions**: `EventSource.subscribe("topic")`
* **File watchers**: `Filesystem.watch("./src")`
* **Polling**: `Stream.poll(interval = 5s, fn => fetch())`
* **Agent outputs**: `Agent.stream(task, inputs)`

### Stream operations

| Operation | Description |
|---|---|
| `map(f)` | Transform each element |
| `filter(p)` | Keep elements matching predicate |
| `take(n)` | Take first n elements |
| `skip(n)` | Skip first n elements |
| `batch(n)` | Group into batches of n |
| `window(d)` | Group into time windows of duration d |
| `merge(s)` | Merge with another stream |
| `for_each(f)` | Apply f to each element |

---

## Chapter 69 &mdash; Event System

Lucky has a built-in pub/sub event system for reactive orchestration.

### Event subscription

```lucky
when
	README changes
	main branch updates
	new PR opened
run
	ArchitectureReview
```

The `when` block declares event conditions. When all conditions are met, the `run` block executes.

### Event types

* **File events**: `path changes`, `path created`, `path deleted`
* **Git events**: `branch updates`, `pr opened`, `pr merged`
* **Time events**: `cron("0 9 * * 1")`, `every 1h`
* **Custom events**: `event("deployment.complete")`

### Event-driven workflows

```lucky
workflow CIListener
	when
		event("push") on main
	run
		Test -> Build -> Deploy
```

---

# Part X &mdash; Error Model

---

## Chapter 70 &mdash; Result Type

Lucky avoids exceptions. Every fallible operation returns a `Result` type with a default error parameter:

```lucky
type Result<T, E = Error> = Success { value: T } | Failure { error: E }

type Error
	code: Int
	message: String
	recoverable: Bool
	cause: Error?
```

### Creating results

```lucky
Success(42)
Failure(error("file not found", recoverable: true))
```

### Handling results

```lucky
match result
	Success(data) => process(data)
	Failure(err) if err.recoverable => retry(err)
	Failure(err) => escalate(err)
```

### Result chaining

```lucky
let outcome = fetch(url)
	.and_then(fn data => parse(data))
	.or_else(fn err => fallback(err))
```

### Task return status

Every task implicitly returns one of:

```lucky
Success   # task completed normally
Failure   # task failed
Cancelled # task was cancelled
Skipped   # task was skipped by policy
```

---

## Chapter 71 &mdash; Recovery Policies

Recovery policies define how the runtime responds to failures.

```lucky
policy ResilientPolicy
	retry 3 with backoff exponential(max: 10m)
	on_permanent_failure fallback
	on_transient_failure retry
	checkpoint before retry
```

### Failure classification

| Category | Description | Default Action |
|---|---|---|
| `transient` | Temporary (network, rate limit) | Retry |
| `permanent` | Unrecoverable (validation, permission) | Fallback or escalate |
| `timeout` | Exceeded time limit | Retry or fallback |
| `cancelled` | Explicit cancellation | Skip |
| `budget` | Cost limit exceeded | Abort |

### Policy composition

Policies compose through inheritance. A task inherits its agent's policy, which inherits its workflow's policy.

---

## Chapter 72 &mdash; Retry

The `retry` recovery action re-executes a failed task.

```lucky
attempt
	fetch_from_api
recover
	retry 3 with backoff exponential(max: 5m)
```

### Retry strategies

| Strategy | Description |
|---|---|
| `retry N` | Retry up to N times |
| `retry with backoff linear` | Linear delay: 1s, 2s, 3s, ... |
| `retry with backoff exponential` | Exponential delay: 1s, 2s, 4s, 8s, ... |
| `retry with jitter` | Add random jitter to delay |
| `retry until condition` | Retry until predicate holds or max reached |
| `retry with circuit_breaker` | Stop retrying if failure rate exceeds threshold |

### Retry scope

```lucky
# retry a single task
task FlakyTask
	policy retry 5

# retry an entire block
attempt
	step1
	step2
	step3
recover
	retry 2
```

---

## Chapter 73 &mdash; Rollback

When a sequence of operations fails partway through, the rollback mechanism reverses completed steps.

```lucky
transaction
	Git.commit("migration")
	Database.migrate("v2")
	Config.update("v2")

# if Database.migrate fails, Git.commit is rolled back
```

### Declaring rollback actions

```lucky
task DeployVersion
	input version: Version
	rollback
		undeploy version
	steps
		...

task CreateBranch
	input name: String
	rollback
		Git.delete_branch(name)
	steps
		...
```

### Automatic rollback

The runtime tracks which tasks completed within a transaction and calls their rollback handlers in reverse order on failure. If a rollback itself fails, the runtime escalates to human.

---

## Chapter 74 &mdash; Human Escalation

When automated recovery is exhausted, the runtime escalates to a human operator.

```lucky
attempt
	critical_operation
recover
	retry 3
recover
	human escalate "Critical operation failed after 3 retries:
					{error.message}"
```

### Escalation context

The human receives:

* The original error
* Recovery attempts and their outcomes
* The checkpointed state at the point of failure
* Suggested remediation actions

### Human response

The human can:

* **Retry**: re-execute with modified parameters
* **Skip**: bypass the failed task
* **Abort**: terminate the workflow
* **Override**: manually provide the expected output and continue
* **Delegate**: assign to a different agent or model

---

# Part XI &mdash; Standard Library

---

## Chapter 75 &mdash; Collections

The `collections` package provides immutable collection types and operations.

```lucky
import collections

let list = [1, 2, 3, 4, 5]
let mapped = list.map(fn x => x * 2)
let filtered = list.filter(fn x => x > 2)
let reduced = list.reduce(0, fn acc, x => acc + x)
let sorted = list.sort()
let grouped = list.group_by(fn x => x % 2)

let set = {1, 2, 3}
let union = set.union({3, 4, 5})
let intersection = set.intersection({2, 3, 4})

let map = {"a": 1, "b": 2}
let keys = map.keys()
let values = map.values()
let merged = map.merge({"b": 3, "c": 4})
```

### Conversion

```lucky
list.to_set()
set.to_list()
list.to_map(fn x => (x.key, x.value))
map.to_list()
```

---

## Chapter 76 &mdash; File System

The `filesystem` package provides safe, permission-gated file access.

```lucky
import filesystem

let content = filesystem.read("./path/to/file.md")
let lines = filesystem.read_lines("./path/to/file.txt")
filesystem.write("./output.md", content)
filesystem.exists("./path")
filesystem.list("./directory")
filesystem.walk("./project")
	|> filter(fn f => f.extension == ".lk")
	|> for_each(fn f => process(f))
```

### Globbing

```lucky
let py_files = filesystem.glob("./**/*.py")
let all_source = filesystem.glob("./src/**/*.{lk,py,ts}")
```

### Permission awareness

File system operations are gated by the agent's permissions. An agent without `filesystem.read` cannot call `filesystem.read()`.

---

## Chapter 77 &mdash; Git

The `git` package provides native version control operations.

```lucky
import git

git.clone("https://github.com/org/repo.git")
git.status()
git.diff()
git.add("*.lk")
git.commit("feat: add new workflow")
git.push()
git.pull()
git.branch("feature/x")
git.checkout("main")
git.merge("feature/x")
git.log(count = 10)
git.blame("src/main.lk")
```

### PR operations

```lucky
git.create_pr(
	title = "Add security review workflow",
	base = "main",
	head = "feature/security-review",
	body = "...",
)
git.list_prs(state = "open")
git.review_pr(pr_number, approval = "approved")
```

---

## Chapter 78 &mdash; Browser

The `browser` package provides web navigation and scraping.

```lucky
import browser

browser.navigate("https://example.com")
browser.click("#button")
browser.type("#input", "query text")
browser.screenshot()
browser.extract("article")
browser.extract_all("a.link")
let html = browser.content()
let title = browser.title()
browser.pdf("./page.pdf")
```

### Headless and headed modes

```lucky
browser.configure(headless = true, timeout = 30s)
```

### Authentication

```lucky
browser.login(
	url = "https://app.example.com/login",
	username = context.credentials.user,
	password = context.credentials.pass,
)
```

---

## Chapter 79 &mdash; Shell

The `shell` package provides controlled command execution.

```lucky
import shell

let result = shell.exec("cargo build")
let output = shell.exec("ls -la", capture = true)
shell.exec("npm test", timeout = 5m)
```

### Command safety

```lucky
policy
	shell.allowed_commands = ["ls", "cat", "grep", "find", "cargo", "npm"]
	shell.denied_patterns = ["rm -rf", "sudo", "chmod 777"]
```

### Working directory

```lucky
shell.cd("./project")
shell.exec("make")
```

---

## Chapter 80 &mdash; HTTP

The `http` package provides HTTP client functionality.

```lucky
import http

let response = http.get("https://api.example.com/users")
let response = http.post(
	"https://api.example.com/tasks",
	body = { "title": "New Task" },
	headers = { "Authorization": "Bearer \{token}" },
)
let response = http.put("https://api.example.com/tasks/1", body)
let response = http.delete("https://api.example.com/tasks/1")
```

### Response handling

```lucky
let status = response.status        # Int
let body = response.json()          # Any
let text = response.text()          # String
let headers = response.headers      # Map<String, String>
```

### Retry and timeout

```lucky
let response = http.get(
	url,
	retry = 3,
	timeout = 30s,
	backoff = exponential,
)
```

---

## Chapter 81 &mdash; AI

The `ai` package provides direct AI operations beyond agent-based workflows.

```lucky
import ai

let answer = ai.ask("What is the capital of France?")
let summary = ai.summarize(document, max_words = 100)
let translation = ai.translate(text, to = "Japanese")
let keywords = ai.extract_keywords(article, count = 10)
let sentiment = ai.sentiment(review)
let embedding = ai.embed(text)
let classification = ai.classify(ticket, categories = ["bug", "feature", "question"])
```

### Model selection

```lucky
ai.use(Claude)
let answer = ai.ask(question)
```

### Confidence

```lucky
let result = ai.ask(difficult_question, min_confidence = 0.8)
let response = match result
	Answer { text, confidence } if confidence >= 0.9 => text
	Answer { text } => "(low confidence) \{text}"
```

### RAG

```lucky
let answer = ai.rag(
	query = "How do I deploy?",
	knowledge = CompanyDocs,
	top_k = 5,
)
```

---

## Chapter 82 &mdash; Time

The `time` package provides temporal operations.

```lucky
import time

let now = time.now()
let epoch = time.epoch()
let tomorrow = now.add(1d)
let yesterday = now.subtract(1d)
let diff: Duration = later - earlier
let formatted = time.format(now, "%Y-%m-%d %H:%M:%S")
let parsed = time.parse("2026-01-01", "%Y-%m-%d")

time.sleep(5s)
time.measure(fn => expensive_operation())
```

### Scheduling

```lucky
time.schedule("0 9 * * 1-5", fn => daily_report())
time.every(1h, fn => health_check())
```

---

## Chapter 83 &mdash; Math

The `math` package provides mathematical operations.

```lucky
import math

math.abs(-5)               # 5
math.max(1, 2, 3)          # 3
math.min(1, 2, 3)          # 1
math.round(3.7)            # 4
math.floor(3.7)            # 3
math.ceil(3.2)             # 4
math.sqrt(16)              # 4.0
math.pow(2, 10)            # 1024
math.log(100, 10)          # 2.0
math.sin(math.pi / 2)      # 1.0
math.random()              # Float in [0, 1)
math.random_int(1, 100)    # Int in [1, 100]
math.clamp(x, 0, 100)

# Statistics
math.mean(values)
math.median(values)
math.stdev(values)
math.percentile(values, 95)
```

---

# Appendix A &mdash; Grammar

The complete formal grammar in Extended Backus-Naur Form. See Chapter 15 for the full EBNF specification.

---

# Appendix B &mdash; Keywords

The complete list of reserved keywords:

```
agent       allow       and          approval    ask
attempt     await       break        capability  const
context     continue    deep         deny        else
error       fallback    false        fast        fn
for         goal        human        if          import
in          input       knowledge    let         loop
match       memory      model        none        not
null        or          output       parallel    permission
policy      project     prompt       recover     retry
return      run         select       skip        steps
success     swarm       task         then        tool
true        unknown     use          wait        when
where       workflow
```

Pseudo-keywords (context-dependent):

```
abort       asc         backoff      browser     cache
clone       commit      compare      deploy      desc
extract     filter      generate     group       join
limit       map         order        push        rank
reason      reduce      review       rollback    save
search      sort        summarize
```

---

# Appendix C &mdash; Operators

| Precedence | Operator | Associativity | Description |
|---|---|---|---|
| 1 | `()`, `[]`, `.` | Left | Call, index, member access |
| 2 | `.?`, `?\|`, `?[` | Left | Nullable operators |
| 3 | `-` (unary), `not` | Right | Negation, logical NOT |
| 4 | `*`, `/`, `%` | Left | Multiplication, division, remainder |
| 5 | `+`, `-` | Left | Addition, subtraction |
| 6 | `==`, `!=`, `<`, `>`, `<=`, `>=` | Left | Comparison |
| 7 | `and` | Left | Logical AND |
| 8 | `or` | Left | Logical OR |
| 9 | `\|>` | Left | Pipeline |
| 10 | `->` | &mdash; | Workflow dependency (statement-level) |

---

# Appendix D &mdash; IR Mapping

The Lucky Intermediate Representation maps source constructs to graph nodes.

### Mapping table

| Source Construct | IR Node | Description |
|---|---|---|
| `goal G` | `GoalNode { id, criteria, workflows[] }` | Entry point with success predicates |
| `workflow W` | `Subgraph { nodes[], edges[] }` | Named subgraph of the execution DAG |
| `agent A` | `AgentDef { id, model, memory, tools, policy }` | Agent entity definition |
| `task T` | `TaskDef { id, inputs[], outputs[], steps[] }` | Task definition |
| `A.task()` | `InvokeNode:agent_invoke { agent: A, task: T, inputs }` | Task invocation |
| `A -> B` | `Edge { from: A, to: B }` | Sequential dependency |
| `parallel { ... }` | `ParallelNode { branches[] }` + `JoinNode` | Fork/join pattern |
| `if C { A } else { B }` | `DecisionNode { condition: C, then: A, else: B }` | Conditional branch |
| `match x { ... }` | `DecisionNode` with multiple branches | Multi-way branch |
| `loop { ... }` | `LoopNode { body, max_iterations }` | Bounded loop |
| `approval before X` | `ApprovalNode { gate: X }` | Human approval gate |
| `attempt { ... } recover { ... }` | `AttemptNode { body, recovery[] }` | Error handling |
| `ask human: "..."` | `HumanQueryNode { question }` | Interactive human query |
| `T \|> U` | `PipelineNode { stages[] }` | Pipeline composition |
| `let x = E` | `LetNode { name, value }` | Immutable binding |

### IR format

The IR is serialized as JSON (`.lir` files) with the following top-level structure:

```json
{
  "version": "0.1",
  "project": { "name": "...", "version": "..." },
  "modules": [ ... ],
  "graph": {
	"nodes": [ ... ],
	"edges": [ ... ]
  },
  "agents": [ ... ],
  "policies": [ ... ],
  "context": { ... },
  "symbols": { ... }
}
```

---

# Appendix E &mdash; Memory Model

### Memory architecture

```
┌─────────────────────────────────────────────┐
│                  Workflow                    │
│  ┌──────────────┐  ┌──────────────┐         │
│  │   Context    │  │   Policies   │         │
│  │  (read-only) │  │              │         │
│  └──────────────┘  └──────────────┘         │
│  ┌──────────────────────────────────────┐   │
│  │             Agent Memory             │   │
│  │  ┌────────┐ ┌────────┐ ┌────────┐   │   │
│  │  │ Vector │ │Struct'd│ │ Cache  │   │   │
│  │  │ Store  │ │ Store  │ │        │   │   │
│  │  └────────┘ └────────┘ └────────┘   │   │
│  └──────────────────────────────────────┘   │
│  ┌────────┐ ┌────────┐ ┌────────┐          │
│  │ Task 1 │ │ Task 2 │ │ Task 3 │  ...     │
│  │ local  │ │ local  │ │ local  │          │
│  │ state  │ │ state  │ │ state  │          │
│  └────────┘ └────────┘ └────────┘          │
│  ┌──────────────────────────────────────┐   │
│  │           Checkpoint Storage         │   │
│  └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

### Allocation

* **Immutable values** (primitives, collections): allocated on a shared heap with reference counting. No cycle collection is needed because all data structures are acyclic (DAG).
* **Agent memory**: allocated per-agent, persists across task invocations within a session.
* **Task-local state**: stack-allocated for the duration of a task. Cleared on task completion unless checkpointed.
* **Context**: allocated once per scope, shared across all tasks in that scope.

### Concurrency

* Concurrent tasks share **immutable values** freely (reference-counted pointers).
* Agent memory is accessed under internal locks managed by the runtime.
* Channels use bounded buffers with backpressure.

### Checkpoint persistence

Checkpoints are written to a configurable store (file system, S3, database). The checkpoint format is:

```json
{
  "checkpoint_id": "uuid",
  "parent_id": "uuid | null",
  "timestamp": "ISO8601",
  "task_states": {
	"task_id": {
		"status": "running | completed | failed",
		"inputs": { ... },
		"outputs": { ... },
		"position": { "step": 3 }
	}
  },
  "agent_memories": {
	"agent_id": {
		"fields": { ... }
	}
  },
  "context": { ... },
  "graph_progress": {
	"completed_nodes": [ ... ],
	"active_nodes": [ ... ]
  }
}
```

### Garbage collection

The runtime uses reference counting for immutable values. Agent memory is explicitly managed &mdash; entries are removed via `memory.forget()`. Checkpoints older than the retention policy are purged automatically. Completed workflow artifacts may be archived or deleted based on project configuration.

---

*End of Lucky Language Reference Manual, Version 0.1*
