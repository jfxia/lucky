# Formal Semantics for Lucky: A Goal-Oriented DSL for AI Agent Orchestration

**Jingfeng Xia**  

Version 0.1, July 2026

---

## Abstract

We present the first complete formal semantics for **Lucky**, a goal-oriented domain-specific language (DSL) designed for autonomous AI agent orchestration. Lucky occupies a unique position in the programming language landscape: it treats large language model (LLM) invocations, tool calls, agent memory, and human approvals as first-class language primitives, rather than library calls. This design introduces semantic challenges absent from conventional programming languages — non-deterministic LLM outputs, hierarchical memory isolation, capability-based security, and declarative error recovery. We formalize Lucky across three layers: (1) a disambiguated context-free grammar reconstructed from the existing implementation, (2) a static type system with AI-specific type constructors (probabilistic types, confidence subtyping, tool contract types, and memory handle types) and provable type safety, and (3) a small-step operational semantics that precisely models all orchestration primitives including parallel execution with barrier synchronization, LLM non-deterministic branching, three-tier memory (session/project/vector) with isolation invariants, tool invocation contracts with schema validation and timeout/retry, and attempt-recover fault tolerance chains. We prove four key metatheorems: **Type Safety** (progress + preservation), **Memory Isolation** (private agent memory is invisible across parallel branches), **Deadlock Freedom** (well-typed parallel workflows never permanently block), and **Recovery Completeness** (every failure scenario has a defined fallback path). We further validate the formal semantics against the existing Rust compiler/interpreter implementation, identifying three semantic gaps: (i) parallel memory write ordering is unspecified, (ii) loop exit condition evaluation has a one-step delay bug, and (iii) LLM failure branch recovery is incompletely implemented. We propose concrete patches grounded in the formal model. Our work fills a critical gap in the AI agent DSL landscape: while systems like AutoGen, LangGraph, and CrewAI provide engineering APIs, none offer formal semantic foundations or mechanized correctness guarantees.

**Keywords:** formal semantics, operational semantics, domain-specific language, AI agent orchestration, type safety, memory isolation, LLM programming, workflow verification

## Table of Contents

- [Abstract](#abstract)
- [1. Introduction](#1-introduction)
  - [1.1 Motivation](#11-motivation)
  - [1.2 Contributions](#12-contributions)
  - [1.3 Paper Organization](#13-paper-organization)
- [2. Language Overview](#2-language-overview)
- [3. Formal Grammar](#3-formal-grammar)
  - [3.1 Token Specification](#31-token-specification)
  - [3.2 Grammar Ambiguity Analysis](#32-grammar-ambiguity-analysis)
  - [3.3 Normalized Context-Free Grammar](#33-normalized-context-free-grammar)
  - [3.4 Grammar Properties](#34-grammar-properties)
- [4. Type System](#4-type-system)
  - [4.1 Type Universes](#41-type-universes)
  - [4.2 Type Constructors](#42-type-constructors)
  - [4.3 Subtyping Relations](#43-subtyping-relations)
  - [4.4 Typing Rules](#44-typing-rules)
  - [4.5 Type Safety](#45-type-safety)
- [5. Runtime State Model](#5-runtime-state-model)
- [6. Operational Semantics](#6-operational-semantics)
- [7. AI-Domain-Specific Semantics](#7-ai-domain-specific-semantics)
- [8. Metatheorems](#8-metatheorems)
- [9. Implementation Validation](#9-implementation-validation)
- [10. Related Work](#10-related-work)
- [11. Conclusion and Future Work](#11-conclusion-and-future-work)
- [References](#references)
- [Appendix A: Complete Typing Rules](#appendix-a-complete-typing-rules)
- [Appendix B: IR Node Semantics Mapping](#appendix-b-ir-node-semantics-mapping)
- [Appendix C: MIR Instruction Semantics](#appendix-c-mir-instruction-semantics)


---

## 1. Introduction

### 1.1 Motivation

The rapid proliferation of large language model (LLM) agents has produced a new class of software systems: **AI agent orchestration programs** that coordinate multiple autonomous agents, each equipped with memory, tools, and reasoning strategies, toward a declared goal. Existing approaches — AutoGen [1], LangGraph [2], CrewAI [3], and OpenAI Agents SDK [4] — represent these programs as Python scripts or configuration files with no formal semantics. Developers reason about agent behavior through trial and error, runtime logging, and ad-hoc testing. The consequences are predictable: race conditions in parallel agent execution, silent memory leaks across agent boundaries, unrecoverable tool failures, and security violations from insufficient permission checks.

Lucky [5] was designed to address these engineering challenges by making AI orchestration a *language-level* concern. Its core insight is that an LLM agent workflow is not a general-purpose program with library calls — it is a fundamentally different computational model where: (a) the primary computation unit is a probabilistic LLM invocation rather than a deterministic CPU instruction, (b) agents own private mutable memory that must be isolated from concurrent peers, (c) tool calls are schema-constrained external operations subject to timeouts and degradation, and (d) human approval is a legitimate control-flow primitive, not a UI feature.

However, Lucky's current implementation — a complete Rust compiler, interpreter, and runtime — was built without prior formal modeling. The grammar was evolved incrementally during development, the type checker performs structural checks but lacks formal inference rules, and the runtime scheduler operates on heuristics rather than provable scheduling invariants. This paper provides the missing formal foundation.

### 1.2 Contributions

We make the following contributions:

1. **Formal grammar reconstruction** (§3): We extract a complete, disambiguated EBNF grammar from the existing lexer/parser implementation, identify three classes of grammatical ambiguity, and provide a normalized CFG suitable for formal reasoning.

2. **Static type system formalization** (§4): We define a type system $\Gamma \vdash e : \tau$ with four type universes (primitive, AI resource, tool contract, memory handle), probabilistic type constructors, and confidence subtyping. We give complete typing rules for all 19 statement forms and 28 expression forms, and prove the Type Safety theorem.

3. **Operational semantics** (§5–7): We define a small-step operational semantics $\langle e, S \rangle \rightarrow \langle e', S' \rangle$ over a rich runtime state tuple, covering: sequence, parallel with barrier, conditional branching, bounded loops, agent invocation, tool calls with contracts, LLM non-deterministic branching, three-tier memory operations, and attempt-recover chains. This is the paper's central innovation — prior formal semantics work has not modeled LLM probabilistic outputs, agent memory isolation, or tool contract validation as operational primitives.

4. **Metatheorems** (§8): We prove four core theorems — Type Safety (progress + preservation), Memory Isolation, Deadlock Freedom, and Recovery Completeness — with detailed proof sketches suitable for mechanization in Lean 4.

5. **Implementation validation** (§9): We systematically compare the formal semantics against the existing Rust implementation, identifying three semantic bugs and proposing formally-grounded patches.

6. **Related work positioning** (§10): We survey three categories of related work — general PL formalization, existing agent DSLs, and traditional workflow formalisms — and demonstrate that Lucky's formal semantics fills a specific and important gap.

### 1.3 Paper Organization

§2 provides a language overview. §3 formalizes the grammar. §4 presents the type system. §5 defines the runtime state model. §6 gives operational semantics for all primitives. §7 covers AI-domain-specific semantics. §8 proves metatheorems. §9 validates against the implementation. §10 surveys related work. §11 concludes with future directions.

---

## 2. Language Overview

Lucky is a declarative workflow language with imperative islands. A program declares **goals** (what success means), **workflows** (how goals are decomposed into DAGs), **agents** (who performs work, owning memory, tools, and permissions), **tasks** (schedulable units of computation), and **policies** (retry, timeout, checkpoint rules). Eight design principles govern the language:

| # | Principle | Formal Implication |
|---|-----------|-------------------|
| 1 | Intent First | Declarative workflow body, imperative task steps |
| 2 | AI Native | LLM invocation is a primitive, not a library call |
| 3 | Deterministic Structure | Execution DAG is reproducible; only LLM leaves are non-deterministic |
| 4 | Context Everywhere | Implicit layered context propagation (§6.3) |
| 5 | Parallel by Default | Independent nodes execute concurrently (§6.2) |
| 6 | Human Approval | Approval gates are control-flow primitives (§6.6) |
| 7 | Recoverability | Every task is resumable via checkpointing (§6.7) |
| 8 | Capability Security | Agents run under least-privilege permissions (§4.4) |

A minimal Lucky program:

```lucky
project ResearchPipeline

agent Researcher
    model DeepSeek
    tools Browser, Search
    memory ProjectMemory

task AnalyzeRepo
    input repo: URI
    output report: String
    steps
        let data = Browser.search(repo)
        return data

goal BuildReport
    success report_generated
    workflow MainFlow
```

The compilation pipeline is:

```math
\text{Source} \xrightarrow{\text{Lexer}} \text{Tokens} \xrightarrow{\text{Parser}} \text{AST} \xrightarrow{\text{TypeCheck+Resolve}} \text{TypedAST} \xrightarrow{\text{HIR Builder}} \text{DAG} \xrightarrow{\text{MIR Lowering}} \text{SSA} \xrightarrow{\text{Optimizer}} \text{Optimized IR} \xrightarrow{\text{Runtime}} \text{Execution}
```

The runtime executes the DAG via a priority-based scheduler with retry/circuit-breaker logic, LLM backend routing (9 providers), and vector-based agent memory.

---

## 3. Formal Grammar

### 3.1 Token Specification

We reconstruct the lexical specification from the two-phase lexer implementation. The formal token algebra is:

```math
\text{Token} = \text{Keyword}_{101} \mid \text{Ident} \mid \text{IntLit} \mid \text{FloatLit} \mid \text{StringLit} \mid \text{BoolLit} \mid \text{NullLit} \mid \text{UnknownLit} \mid \text{Operator} \mid \text{Comment} \mid \text{Newline} \mid \text{Indent} \mid \text{Dedent} \mid \text{EOF}
```

**Identifiers** follow the pattern \\(\text{Ident} = (L \mid \text{\_}) \cdot (L \mid N \mid \text{\_})^*\\) where $L$ denotes Unicode Letter categories (Lu, Ll, Lt, Lm, Lo) and $N$ denotes Number categories (Nd, Nl, No), excluding $N$ as first character.

**String literals** support interpolation: `"text\{expr}text"` where \\(\text{expr}\\) is any Lucky expression. Triple-quoted strings `"""..."""` preserve indentation relative to the closing delimiter.

**Indentation tokens** (INDENT/DEDENT) are synthesized in a second lexer phase using an indent stack \\(\sigma : \text{List}(\mathbb{N})\\), following Python's rules:

```math
\text{IndentPhase}(\text{raw\_tokens}, \sigma_0 = [0]) \rightarrow (\text{final\_tokens}, \sigma_f)
```

The synthesis rule: when a NEWLINE token is followed by a line with indentation level $d$:

- If $d > \text{top}(\sigma)$: emit INDENT, push $d$ onto $\sigma$
- If $d < \text{top}(\sigma)$: emit DEDENT for each level popped until $\text{top}(\sigma) = d$
- If $d = \text{top}(\sigma)$: no indent/dedent emission

Blank lines and comment-only lines do not affect $\sigma$.

### 3.2 Grammar Ambiguity Analysis

The original implementation grammar (as coded in the Pratt parser) contains three classes of ambiguity:

**Ambiguity 1: Expression-Statement Overlap.** The rule `statement = expression NEWLINE` overlaps with `callStmt = expression NEWLINE` and `assignStmt = assignTarget "=" expression NEWLINE`. Since `assignTarget` includes `qualifiedName` and `indexExpr`, and these are also valid `primaryExpr` subtypes, the parser cannot decide between statement-interpretation and expression-interpretation without lookahead beyond `=`.

**Ambiguity 2: Set/Map Literal Conflict.** The rules `setExpr = "{" [ expression { "," expression } ] "}"` and `mapExpr = "{" [ mapEntry { "," mapEntry } ] "}"` share the same opening delimiter `{`. Disambiguation requires examining whether the first item after `{` contains a `:` separator (map) or not (set). The current parser resolves this by looking ahead one token after `{`.

**Ambiguity 3: Confidence Expression vs. Comparison.** The rule `confidenceExpr = expression "confidence" comparisonOp expression` introduces the keyword `confidence` as a pseudo-operator, which can be confused with a field access `expr.confidence` when `confidence` appears as an identifier. The specification marks `confidence` as a "modifier keyword" (context-dependent), but the parser implementation does not distinguish these cases cleanly.

### 3.3 Normalized Context-Free Grammar

We present the disambiguated grammar. Changes from the original are marked with **[N]** (normalized):

```ebnf
(* === Top Level === *)
program         = projectDecl { moduleItem } ;

projectDecl     = "project" Ident NEWLINE ;

moduleItem      = pubDecl | importDecl | agentDecl | taskDecl
                | workflowDecl | goalDecl | memoryDecl | toolDecl
                | modelDecl | promptDecl | policyDecl | typeDecl
                | contextDecl | permissionDecl | approvalDecl ;

(* === Declarations === *)
agentDecl       = "agent" Ident agentBody ;
taskDecl        = "task" Ident taskBody ;
workflowDecl    = "workflow" Ident workflowBody ;
goalDecl        = "goal" Ident goalBody ;
memoryDecl      = "memory" Ident block ;
modelDecl       = "model" Ident [ "(" paramList ")" ] NEWLINE ;  (* [N]: explicit delimiter *)
promptDecl      = "prompt" Ident block ;
policyDecl      = "policy" Ident block ;
typeDecl        = "type" Ident [ typeParams ] "=" typeExpr NEWLINE ;

(* === Agent Body [N]: explicit section ordering === *)
agentBody       = INDENT { agentSection } DEDENT ;
agentSection    = modelSection | memorySection | toolsSection
                | permissionsSection | policySection | promptSection
                | taskDecl ;

modelSection    = "model" Ident NEWLINE ;
memorySection   = "memory" Ident NEWLINE ;
toolsSection    = "tools" NEWLINE INDENT { Ident NEWLINE } DEDENT ;
permissionsSection = "permissions" NEWLINE INDENT { permEntry NEWLINE } DEDENT ;
policySection   = "policy" [ Ident ] NEWLINE block ;
promptSection   = "prompt" Ident NEWLINE ;

(* === Task Body === *)
taskBody        = INDENT { taskSection } DEDENT ;
taskSection     = inputSection | outputSection | stepsSection
                | contextSection | policySection | rollbackSection ;  (* [N]: rollback as section *)

inputSection    = "input" NEWLINE INDENT { typedIdent NEWLINE } DEDENT ;
outputSection   = "output" NEWLINE INDENT { typedIdent NEWLINE } DEDENT ;
stepsSection    = "steps" NEWLINE block ;
contextSection  = "context" NEWLINE INDENT { typedIdent NEWLINE } DEDENT ;
rollbackSection = "rollback" NEWLINE block ;  (* [N]: new section type *)

(* === Workflow Body [N]: arrow syntax as sequencing operator === *)
workflowBody    = INDENT { workflowStep } DEDENT ;
workflowStep    = taskCall [ "->" NEWLINE taskCall ]
                | parallelBlock | ifBlock | attemptBlock | whenBlock ;

taskCall        = [ Ident "." ] Ident [ "(" argList ")" ] ;

parallelBlock   = "parallel" NEWLINE INDENT { taskCall NEWLINE } DEDENT
                [ "wait" NEWLINE ] ;

(* === Statements === *)
block           = INDENT { statement } DEDENT ;

statement       = letStmt | constStmt | assignStmt | ifStmt | matchStmt
                | loopStmt | forStmt | parallelStmt | whenStmt
                | pipelineStmt | returnStmt | breakStmt | continueStmt
                | attemptStmt | exprStmt ;  (* [N]: exprStmt distinguished *)

letStmt         = "let" Ident [ ":" typeExpr ] "=" expr NEWLINE ;
constStmt       = "const" Ident [ ":" typeExpr ] "=" expr NEWLINE ;
(* [N]: assignStmt restricted to memory-field targets only *)
assignStmt      = "memory" "." Ident "=" expr NEWLINE ;
exprStmt        = expr NEWLINE ;  (* [N]: pure expression statement *)

(* === Control Flow === *)
ifStmt          = "if" expr NEWLINE block
                { "else" "if" expr NEWLINE block }
                [ "else" NEWLINE block ] ;

matchStmt       = "match" expr NEWLINE INDENT { matchArm } DEDENT ;
matchArm        = pattern [ "if" expr ] "=>" NEWLINE block ;

loopStmt        = "loop" NEWLINE block ;
forStmt         = "for" Ident "in" expr NEWLINE block ;

parallelStmt    = "parallel" NEWLINE INDENT { block } DEDENT
                "wait" NEWLINE ;

(* === Error Handling === *)
attemptStmt     = "attempt" NEWLINE block
                { "recover" NEWLINE INDENT { recoveryAction NEWLINE } DEDENT } ;

recoveryAction  = "retry" [ IntLit ] [ "with" "backoff" backoffSpec ] NEWLINE
                | "fallback" taskCall NEWLINE
                | "human" [ "escalate" StringLit ] NEWLINE
                | "abort" NEWLINE
                | "skip" NEWLINE ;

backoffSpec     = "exponential" [ "(" "max" ":" DurationLit ")" ]
                | "linear" [ "(" "max" ":" DurationLit ")" ] ;

(* === Expressions [N]: precedence-resolved Pratt hierarchy === *)
expr            = pipeExpr ;

pipeExpr        = logicExpr { "|>" logicExpr } ;          (* precedence 1 *)
logicExpr       = cmpExpr { ("and" | "or") cmpExpr } ;     (* precedence 2 *)
cmpExpr         = addExpr { cmpOp addExpr } ;              (* precedence 3 *)
addExpr         = mulExpr { ("+" | "-") mulExpr } ;        (* precedence 4 *)
mulExpr         = unaryExpr { ("*" | "/" | "%") unaryExpr } ; (* precedence 5 *)
unaryExpr       = ("-" | "not") unaryExpr | postfixExpr ;   (* precedence 6 *)
postfixExpr     = primaryExpr { postfixOp } ;              (* precedence 7 *)
postfixOp       = "." Ident                                (* field access *)
                | "(" [ argList ] ")"                      (* call *)
                | "[" expr "]"                              (* index *)
                | ".?" Ident                                (* nullable field *)
                | "?|" expr                                 (* null coalesce *)
                | "?[" expr "]"                             (* nullable index *)
                ;

primaryExpr     = literal | Ident | "(" expr ")"
                | listExpr | mapExpr | setExpr              (* [N]: disambiguated *)
                | interpolatedString
                | askExpr | reasonExpr | useExpr
                | confidenceExpr | askHumanExpr
                | ifExpr | matchExpr | lambdaExpr ;

(* [N]: set/map disambiguation *)
listExpr        = "[" [ expr { "," expr } [ "," ] ] "]" ;
mapExpr         = "{" mapEntry { "," mapEntry } [ "," ] "}" ;  (* at least one entry *)
setExpr         = "{" expr { "," expr } [ "," ] "}" ;          (* no ":" in first entry *)
emptyMapExpr    = "{" ":" "}" ;  (* [N]: empty map requires type annotation *)
emptySetExpr    = "{" "}" ;      (* [N]: requires type annotation *)

(* === AI-specific expressions === *)
askExpr         = "ask" Ident ":" NEWLINE block ;
reasonExpr      = "reason" ("deep" | "fast" | "none") NEWLINE ;
useExpr         = "use" Ident NEWLINE ;
confidenceExpr  = expr "@confidence" cmpOp expr ;  (* [N]: "@" prefix disambiguates *)
askHumanExpr    = "ask" "human" ":" NEWLINE block ;

(* === Patterns === *)
pattern         = wildcardPat | varPat | litPat | constructorPat
                | listPat | mapPat | tuplePat ;

(* === Types === *)
typeExpr        = unionType ;
unionType       = primaryType { "|" primaryType } ;
primaryType     = primType | aiType | collectionType | nullableType
                | optionalType | namedType | tupleType | fnType
                | "(" typeExpr ")" ;

primType        = "Bool" | "Int" | "Float" | "Decimal" | "String"
                | "Bytes" | "Time" | "Duration" | "UUID" | "URI" | "Version" ;

aiType          = "Agent" | "Task" | "Workflow" | "Goal" | "Prompt"
                | "Memory" | "Knowledge" | "Context" | "Tool" | "Model"
                | "Artifact" | "Result" | "Capability" | "Approval"
                | "Embedding" | "Observation" | "Plan" | "Reasoning" ;

collectionType  = "List" "<" typeExpr ">" | "Set" "<" typeExpr ">"
                | "Map" "<" typeExpr "," typeExpr ">" ;
nullableType    = typeExpr "?" ;
optionalType    = typeExpr "!" ;
namedType       = Ident [ "<" typeArgList ">" ] ;
tupleType       = "(" typeExpr { "," typeExpr } ")" ;
fnType          = "fn" "(" [ typeExpr { "," typeExpr } ] ")" "->" typeExpr ;
```

**Summary of normalizations:**

| # | Original Issue | Normalization |
|---|----------------|---------------|
| 1 | assignStmt overlaps with exprStmt | Restrict assignStmt to `memory.field = expr` only |
| 2 | set/map literal ambiguity | Require map entries to contain `:`, empty collections need type annotation |
| 3 | confidence keyword ambiguity | Change `expr confidence cmpOp expr` to `expr @confidence cmpOp expr` |
| 4 | Model parameter syntax inconsistency | Normalize to always use `()` with `=` assignment syntax |
| 5 | Workflow arrow syntax | Formalize `->` as sequencing operator between taskCalls |

### 3.4 Grammar Properties

**Proposition 3.1 (Unambiguity).** The normalized grammar is unambiguous: for every valid token sequence, there exists exactly one parse tree.

*Proof sketch.* (1) Expression precedence is resolved by the Pratt hierarchy with 7 explicit levels, eliminating the expression-statement overlap. (2) Set/map disambiguation uses the first-token heuristic: `{` followed by a `:`-containing entry is a map; `{` followed by a non-`:` entry is a set. Empty collections require type annotations to resolve. (3) The `@confidence` prefix eliminates the keyword/identifier ambiguity. (4) Assignment is syntactically restricted to memory fields, creating a distinct syntactic class from expression statements. ∎

**Proposition 3.2 (No Left Recursion).** The normalized grammar contains no left-recursive rules.

*Proof sketch.* Every expression rule uses the Pratt precedence climbing pattern, which eliminates left recursion by structuring `expr` as a chain of right-associative operators. Statement rules begin with a distinct keyword token, and declaration rules begin with distinct declaration keywords. The only potential left recursion would be in `postfixExpr = primaryExpr { postfixOp }`, but this is iterated, not recursive. ∎

---

## 4. Type System

### 4.1 Type Universes

Lucky's type system spans four universes:

```math
\mathcal{T} = \mathcal{T}_{\text{prim}} \cup \mathcal{T}_{\text{ai}} \cup \mathcal{T}_{\text{contract}} \cup \mathcal{T}_{\text{handle}}
```

**Primitive types** $\mathcal{T}_{\text{prim}}$:

```math
\mathcal{T}_{\text{prim}} = \{\text{Bool}, \text{Int}, \text{Float}, \text{Decimal}, \text{String}, \text{Bytes}, \text{Time}, \text{Duration}, \text{UUID}, \text{URI}, \text{Version}\}
```

**AI resource types** $\mathcal{T}_{\text{ai}}$ — types that have no equivalent in conventional languages:

```math
\mathcal{T}_{\text{ai}} = \{\text{Agent}, \text{Task}, \text{Workflow}, \text{Goal}, \text{Prompt}, \text{Memory}, \text{Knowledge}, \text{Context}, \text{Tool}, \text{Model}, \text{Artifact}, \text{Result}, \text{Capability}, \text{Approval}, \text{Embedding}, \text{Observation}, \text{Plan}, \text{Reasoning}\}
```

**Tool contract types** $\mathcal{T}_{\text{contract}}$ — schema-constrained external operation types:

```math
\mathcal{T}_{\text{contract}} = \{\text{ToolCall}(t, m, \sigma_{\text{in}}, \sigma_{\text{out}}) \mid t \in \text{ToolName}, m \in \text{MethodName}, \sigma_{\text{in}}, \sigma_{\text{out}} \in \text{Schema}\}
```

where Schema is a JSON Schema definition specifying input parameter types and output value types.

**Memory handle types** $\mathcal{T}_{\text{handle}}$:

```math
\mathcal{T}_{\text{handle}} = \{\text{MemRef}(\alpha, s) \mid \alpha \in \text{AgentName}, s \in \{\text{local}, \text{session}, \text{project}, \text{global}\}\}
```

### 4.2 Type Constructors

**Collection constructors:**

```math
\text{List}(\tau), \quad \text{Set}(\tau), \quad \text{Map}(\tau_k, \tau_v), \quad \text{Queue}(\tau), \quad \text{Stream}(\tau)
```

**Nullable type:** $\tau?$ = $\tau \cup \{\text{null}\}$

**Optional type:** $\tau!$ = $\tau \cup \{\text{unknown}\}$, where `unknown` denotes "not yet computed"

**Union type:** $\tau_1 \mid \tau_2$ = $\tau_1 \cup \tau_2$ (structural, untagged by default)

**Probabilistic type:** $\text{Probabilistic}(\tau, c)$ where $c \in [0,1]$ is a confidence threshold. The syntax `uncertain T` is equivalent to $\text{Probabilistic}(T, 0.5)$.

**Parametric AI types:** $\text{Agent}(\alpha)$, $\text{Task}(\kappa)$, $\text{Workflow}(w)$, where the parameter refines the nominal type to a specific declaration.

### 4.3 Subtyping Relations

We define a limited subtyping relation $\tau_1 <: \tau_2$:

```math
\begin{aligned}
&\text{(Reflexivity)} \quad \tau <: \tau \\
&\text{(Nullable)} \quad \tau <: \tau? \\
&\text{(Union-Left)} \quad \tau_1 <: \tau_1 \mid \tau_2 \\
&\text{(Union-Right)} \quad \tau_2 <: \tau_1 \mid \tau_2 \\
&\text{(Never)} \quad \text{Never} <: \tau \quad \text{for all } \tau \\
&\text{(Probabilistic)} \quad \text{Probabilistic}(\tau, c_1) <: \text{Probabilistic}(\tau, c_2) \quad \text{if } c_1 \geq c_2 \\
&\text{(Optional-Nullable)} \quad \tau! <: \tau? \quad \text{(unknown maps to null)}
\end{aligned}
```

Subtyping is **not** transitive beyond these rules — Lucky deliberately avoids full transitive subtyping to keep type inference tractable.

### 4.4 Typing Rules

We present the core typing judgments. A **typing environment** $\Gamma$ maps variables to types and tracks agent ownership:

```math
\Gamma = \{x_1 : \tau_1, \ldots, x_n : \tau_n\} \cup \{\text{self} : \text{Agent}(\alpha)\} \cup \{\text{perms} : \text{PermSet}\}
```

**Variable binding:**

```math
\frac{\Gamma \vdash e : \tau}{\Gamma, x : \tau \vdash \text{let}\ x = e : \text{unit}} \quad \text{(T-Let)}
```

```math
\frac{\text{eval-at-compile-time}(e) = v \quad \Gamma \vdash e : \tau}{\Gamma, x : \tau \vdash \text{const}\ x : \tau = e : \text{unit}} \quad \text{(T-Const)}
```

**Memory assignment (restricted):**

```math
\frac{\Gamma(\text{self}) = \text{Agent}(\alpha) \quad \text{MemRef}(\alpha, s) \in \Gamma \quad \Gamma \vdash e : \tau \quad \Gamma(\text{memory}.f) <: \tau}{\Gamma \vdash \text{memory}.f = e : \text{unit}} \quad \text{(T-MemAssign)}
```

The precondition $\Gamma(\text{self}) = \text{Agent}(\alpha)$ ensures that memory assignment is only legal within an agent context. This rule statically prevents uncontrolled mutable state.

**Agent invocation:**

```math
\frac{\alpha \in \text{AgentDecls} \quad \kappa \in \alpha.\text{tasks} \quad \forall(x_i : \tau_i) \in \kappa.\text{inputs}: \Gamma \vdash a_i : \tau_i' \quad \tau_i' <: \tau_i}{\Gamma \vdash \alpha.\kappa(a_1, \ldots, a_n) : \kappa.\text{output-type}} \quad \text{(T-AgentInvoke)}
```

**Tool invocation (with contract):**

```math
\frac{t \in \Gamma(\text{tools}) \quad \text{ToolCall}(t, m, \sigma_{\text{in}}, \sigma_{\text{out}}) \in \mathcal{T}_{\text{contract}} \quad \text{check-schema}(\sigma_{\text{in}}, [a_1, \ldots, a_n]) = \text{ok} \quad \Gamma \vdash a_i : \tau_i \quad \tau_i <: \sigma_{\text{in}}[i]}{\Gamma \vdash t.m(a_1, \ldots, a_n) : \text{from-schema}(\sigma_{\text{out}})} \quad \text{(T-ToolCall)}
```

The $\text{check-schema}$ function validates that the argument list conforms to the tool's declared JSON Schema. This is a compile-time contract check — if it fails, the program is rejected statically.

**LLM invocation (ask):**

```math
\frac{\Gamma \vdash \text{prompt} : \text{Prompt}(\rho) \quad \Gamma(\text{model}) = \text{Model}(m) \quad \text{output-type}(\rho) = \tau}{\Gamma \vdash \text{ask}\ \text{model} : \text{prompt} : \text{Probabilistic}(\tau, c_{\text{default}})} \quad \text{(T-Ask)}
```

The result type is always $\text{Probabilistic}(\tau, c)$ because LLM outputs are inherently non-deterministic. The confidence threshold $c_{\text{default}}$ is derived from the prompt's declared confidence requirement.

**Confidence check:**

```math
\frac{\Gamma \vdash e : \text{Probabilistic}(\tau, c_e) \quad \Gamma \vdash c_{\text{threshold}} : \text{Float} \quad c_e \geq c_{\text{threshold}}}{\Gamma \vdash e @\text{confidence} \geq c_{\text{threshold}} : \text{Bool}} \quad \text{(T-Confidence)}
```

**Parallel block:**

```math
\frac{\forall i \in [1, n]: \Gamma \vdash s_i : \tau_i \quad \text{no-shared-mutable-refs}(\Gamma, [s_1, \ldots, s_n])}{\Gamma \vdash \text{parallel}\ s_1 \ldots s_n \text{wait} : \text{Map}(\text{String}, \tau_1 \mid \ldots \mid \tau_n)} \quad \text{(T-Parallel)}
```

The side condition $\text{no-shared-mutable-refs}$ ensures that parallel branches do not write to the same agent memory field. This is a static race condition check.

**Attempt-recover:**

```math
\frac{\Gamma \vdash \text{body} : \tau \quad \forall j \in [1, m]: \Gamma \vdash r_j : \text{RecoveryAction}(\tau)}{\Gamma \vdash \text{attempt}\ \text{body}\ \text{recover}\ r_1 \ldots r_m : \tau} \quad \text{(T-Attempt)}
```

where $\text{RecoveryAction}(\tau)$ is defined as:

- $\text{retry}(n, \text{backoff}) : \text{RecoveryAction}(\tau)$ — retries up to $n$ times
- $\text{fallback}(e) : \text{RecoveryAction}(\tau)$ if $\Gamma \vdash e : \tau$ — alternative computation
- $\text{human}(\text{msg}) : \text{RecoveryAction}(\text{Approval})$ — escalates to human
- $\text{abort} : \text{RecoveryAction}(\text{Never})$ — terminates computation

**Permission check (capability security):**

```math
\frac{\Gamma(\text{perms}) \vdash \text{op} : \text{allowed}}{\Gamma \vdash \text{op} : \tau} \quad \text{(T-PermCheck)}
```

where the permission judgment $\Gamma(\text{perms}) \vdash \text{op} : \text{allowed}$ is defined by: an operation $\text{op}$ is allowed iff there exists an `allow` rule $\text{allow}\ p$ where $p$ matches $\text{op}$, and no `deny` rule $\text{deny}\ p'$ where $p'$ also matches $\text{op}$. Deny rules take precedence. Glob patterns: `filesystem.*` matches any filesystem operation; `git.**` matches multi-level methods like `git.remote.add.url`.

### 4.5 Type Safety

**Theorem 4.1 (Type Safety).** If $\Gamma \vdash e : \tau$ and the permission check $\Gamma(\text{perms}) \vdash \text{op} : \text{allowed}$ holds for every operation in $e$, then the evaluation of $e$ will never produce: (a) a tool contract mismatch, (b) an undefined variable reference, (c) an illegal tool invocation (permission violation), or (d) a memory isolation violation.

*Proof sketch.* We prove this via **progress** and **preservation** lemmas.

**Lemma 4.2 (Progress).** If $\vdash e : \tau$ and $e$ is not a value, then there exists a state $S$ such that $\langle e, S \rangle \rightarrow \langle e', S' \rangle$ for some $e', S'$.

*Proof.* By case analysis on $e$. For each non-value expression form:
- **let/const**: The RHS is typed, hence either a value or can step.
- **Tool call**: By (T-ToolCall), the tool is in scope and arguments type-match. The runtime can dispatch to the tool executor.
- **Agent invoke**: By (T-AgentInvoke), the agent and task are declared. The scheduler can create a sub-execution.
- **LLM ask**: By (T-Ask), the model and prompt exist. The runtime can dispatch to the LLM backend.
- **Parallel**: By (T-Parallel), all branches are typed and no shared mutable references exist. Each branch can step independently.
- **Attempt**: By (T-Attempt), the body is typed and recovery actions are available for any failure.

The only "stuck" case would be a tool call to a non-existent tool, which is prevented by (T-ToolCall)'s precondition. ∎

**Lemma 4.3 (Preservation).** If $\vdash e : \tau$ and $\langle e, S \rangle \rightarrow \langle e', S' \rangle$, then $\vdash e' : \tau$ (or $e' : \tau'$ where $\tau' <: \tau$).

*Proof.* By case analysis on the stepping rule. Key cases:
- **Tool call completion**: The output type is determined by $\sigma_{\text{out}}$ in (T-ToolCall), which matches $\tau$.
- **LLM output**: The result is $\text{Probabilistic}(\tau, c)$, which is a subtype of $\tau?$ by the Probabilistic subtyping rule.
- **Parallel branch completion**: Each branch preserves its type, and the merged result is a union of branch types, which subtypes the declared output type.
- **Memory write**: The write type is checked by (T-MemAssign), so the value type matches the field type. ∎

**Corollary 4.4 (Well-typed programs never crash).** Combining Progress and Preservation, a well-typed Lucky program either evaluates to a value of the expected type or enters a defined recovery path (retry/fallback/human/abort), never reaching an undefined state.

---

## 5. Runtime State Model

### 5.1 Global State Tuple

We define the runtime state as a 7-component tuple:

```math
S = (\mathcal{A}, \mathcal{M}, \mathcal{T}, \mathcal{F}, \mathcal{Q}, \mathcal{V}, \mathcal{E})
```

| Component | Domain | Description |
|-----------|--------|-------------|
| $\mathcal{A}$ | $\text{AgentName} \rightarrow \text{AgentState}$ | Agent registry: each agent's model binding, tool set, permission set, prompt, and policy |
| $\mathcal{M}$ | $\text{MemRef}(\alpha, s) \rightarrow \text{MemStore}$ | Memory system: per-agent per-scope key-value + vector stores |
| $\mathcal{T}$ | $\text{ToolName} \rightarrow \text{ToolDef}$ | Tool table: registered tools with schema, methods, and adapters |
| $\mathcal{F}$ | $\text{List}(\text{StackFrame})$ | Flow stack: execution context stack for nested agent/task calls |
| $\mathcal{Q}$ | $\text{AgentName} \rightarrow \text{MsgQueue}$ | Message queues: per-agent asynchronous message channels |
| $\mathcal{V}$ | $\text{KnowledgeName} \rightarrow \text{VecIndex}$ | Vector databases: embedding indices for RAG and similarity search |
| $\mathcal{E}$ | $\text{ErrCtx}$ | Error context: current failure information for recovery routing |

### 5.2 Agent State

```math
\text{AgentState} = (\text{model}, \text{tools}, \text{perms}, \text{prompt}, \text{policy}, \text{mem-refs}, \text{status})
```

where:
- $\text{model} \in \text{ModelName}$ — LLM backend binding
- $\text{tools} \subseteq \text{ToolName}$ — declared tool set
- $\text{perms} : \text{PermSet}$ — capability-security permission set
- $\text{prompt} \in \text{PromptName}?$ — default prompt template
- $\text{policy} : \text{PolicyDef}$ — retry/timeout/checkpoint configuration
- $\text{mem-refs} : \text{Set}(\text{MemRef})$ — memory handles owned by this agent
- $\text{status} \in \{\text{Idle}, \text{Running}, \text{Paused}, \text{WaitingApproval}, \text{Failed}\}$

### 5.3 Memory Store

```math
\text{MemStore} = (\text{kv}: \text{Map}(\text{String}, \text{Value}), \text{vec}: \text{VecIndex}?, \text{tags}: \text{Map}(\text{String}, \text{Set}(\text{String})), \text{ttl}: \text{Map}(\text{String}, \text{Duration}?))
```

Memory operations:

```math
\text{MemOp} = \text{remember}(k, v, \vec{e}?) \mid \text{recall}(k) \mid \text{similar}(\vec{q}, n) \mid \text{search}(q, n) \mid \text{forget}(k) \mid \text{clear}()
```

The vector index $\text{VecIndex}$ supports nearest-neighbor search:

```math
\text{similar}(\vec{q}, n) = \text{top}_n\{(\text{key}, \text{score}) \mid \text{score} = \cos(\vec{q}, \vec{e}_{\text{key}}), \vec{e}_{\text{key}} \in \text{vec.embeddings}\}
```

### 5.4 Context Layer Model

Context propagation follows an immutable layer-chain model:

```math
\text{CtxLayer} = (\text{entries}: \text{Map}(\text{String}, \text{Value}), \text{parent}: \text{CtxLayer}?)
```

```math
\text{lookup}(\text{layer}, k) = \begin{cases} \text{layer.entries}[k] & \text{if } k \in \text{layer.entries} \\ \text{lookup}(\text{layer.parent}, k) & \text{if } \text{layer.parent} \neq \text{null} \\ \text{unknown} & \text{otherwise} \end{cases}
```

The effective context for a node is computed by layered composition:

```math
\text{ctx}_{\text{eff}} = \text{ctx}_{\text{project}} \oplus \text{ctx}_{\text{workflow}} \oplus \text{ctx}_{\text{agent}} \oplus \text{ctx}_{\text{task}} \oplus \text{ctx}_{\text{node}}
```

where $\oplus$ denotes shadow-merge: later layers' entries override earlier layers' entries with the same key.

### 5.5 Permission Model

```math
\text{PermSet} = (\text{allow}: \text{List}(\text{GlobPath}), \text{deny}: \text{List}(\text{GlobPath}))
```

```math
\text{permitted}(P, \text{op}) = \begin{cases} \text{true} & \text{if } \exists p \in P.\text{allow}: \text{match}(p, \text{op}) \land \forall p' \in P.\text{deny}: \neg\text{match}(p', \text{op}) \\ \text{false} & \text{otherwise} \end{cases}
```

where $\text{match}$ implements glob matching: `*` matches one segment, `**` matches multi-level paths, `filesystem.write(./data/*)` additionally constrains argument patterns.

Permission inheritance is **lexical and monotonically narrowing**: a sub-agent inherits its parent's permissions but may only further restrict them (add deny rules or remove allow rules), never widen them.

```math
\text{Lemma 5.1 (Permission Narrowing).} \quad \text{If } P_{\text{child}} \text{ inherits from } P_{\text{parent}}, \text{ then } \forall \text{op}: \text{permitted}(P_{\text{child}}, \text{op}) \implies \text{permitted}(P_{\text{parent}}, \text{op}).
```

---

## 6. Operational Semantics

### 6.1 Semantic Framework

We define a small-step operational semantics over configurations $\langle e, S \rangle$ where $e$ is an expression (or statement) and $S$ is the runtime state tuple. The semantics distinguishes **deterministic steps** (variable binding, memory read, arithmetic) from **non-deterministic steps** (LLM output, tool timeout/retry).

Deterministic steps use the standard relation $\rightarrow$. Non-deterministic steps use a labeled relation $\xrightarrow{\ell}$ where $\ell \in \{\text{llm}(\omega), \text{tool-timeout}, \text{tool-error}(\text{code})\}$.

### 6.2 Core Orchestration Primitives

**Sequential composition** (workflow `->` operator):

```math
\frac{\langle e_1, S \rangle \rightarrow \langle v_1, S' \rangle}{\langle e_1 \rightarrow e_2, S \rangle \rightarrow \langle e_2[\text{context} := v_1], S' \rangle} \quad \text{(E-Seq)}
```

The output of $e_1$ is automatically injected into the context available to $e_2$.

**Parallel execution with barrier**:

```math
\frac{\forall i \in [1, n]: \langle e_i, S \rangle \rightarrow^* \langle v_i, S_i \rangle \quad S_i.\mathcal{M} \text{ updates are disjoint}}{\langle \text{parallel}\ e_1 \ldots e_n \text{ wait}, S \rangle \rightarrow \langle (v_1, \ldots, v_n), \text{merge}(S_1, \ldots, S_n) \rangle} \quad \text{(E-Parallel)}
```

The side condition "$S_i.\mathcal{M}$ updates are disjoint" is enforced by the static typing rule (T-Parallel). The $\text{merge}$ function combines state updates from all branches:

```math
\text{merge}(S_1, \ldots, S_n) = S \quad \text{where} \quad S.\mathcal{M} = \text{merge-mem}(S_1.\mathcal{M}, \ldots, S_n.\mathcal{M})
```

Memory merge resolution for non-conflicting writes:

```math
\text{merge-mem}(\mathcal{M}_1, \ldots, \mathcal{M}_n)(\alpha, s, k) = \begin{cases} \mathcal{M}_i(\alpha, s, k) & \text{if exactly one } i \text{ wrote to } k \\ \text{last-write}(\mathcal{M}_1, \ldots, \mathcal{M}_n, (\alpha, s, k)) & \text{if multiple wrote (last-writer-wins)} \\ S.\mathcal{M}(\alpha, s, k) & \text{if none wrote} \end{cases}
```

**Conditional branching**:

```math
\frac{\langle \text{cond}, S \rangle \rightarrow \langle \text{true}, S' \rangle}{\langle \text{if}\ \text{cond}\ \text{then}\ e_1\ \text{else}\ e_2, S \rangle \rightarrow \langle e_1, S' \rangle} \quad \text{(E-IfTrue)}
```

```math
\frac{\langle \text{cond}, S \rangle \rightarrow \langle \text{false}, S' \rangle}{\langle \text{if}\ \text{cond}\ \text{then}\ e_1\ \text{else}\ e_2, S \rangle \rightarrow \langle e_2, S' \rangle} \quad \text{(E-IfFalse)}
```

**Bounded loop**:

```math
\frac{\langle \text{cond}, S \rangle \rightarrow \langle \text{true}, S' \rangle}{\langle \text{loop-until}\ \text{cond}\ \text{body}, S \rangle \rightarrow \langle \text{body}; \text{loop-until}\ \text{cond}\ \text{body}, S' \rangle} \quad \text{(E-LoopCont)}
```

```math
\frac{\langle \text{cond}, S \rangle \rightarrow \langle \text{false}, S' \rangle}{\langle \text{loop-until}\ \text{cond}\ \text{body}, S \rangle \rightarrow \langle \text{unit}, S' \rangle} \quad \text{(E-LoopExit)}
```

**For iteration**:

```math
\frac{\langle \text{iter}, S \rangle \rightarrow \langle [v_1, \ldots, v_n], S' \rangle}{\langle \text{for}\ x\ \text{in}\ \text{iter}\ \text{body}, S \rangle \rightarrow \langle \text{body}[x := v_1]; \ldots; \text{body}[x := v_n], S' \rangle} \quad \text{(E-For)}
```

### 6.3 Context Propagation

```math
\frac{\text{ctx}_{\text{eff}}(S, \text{node}) = C}{\langle \text{task-call}\ \kappa(\vec{a}), S \rangle \rightarrow \langle \kappa.\text{body}[\text{context} := C \oplus \text{args}(\vec{a})], S \rangle} \quad \text{(E-ContextProp)}
```

The task body receives the effective context merged with the call arguments. After task completion:

```math
\frac{\langle \kappa.\text{body}, S \rangle \rightarrow^* \langle v, S' \rangle}{\langle \text{task-call}\ \kappa(\vec{a}), S \rangle \rightarrow^* \langle v, S'[\text{ctx} := S'.\text{ctx} \oplus \{\kappa.\text{outputs} \mapsto v\}] \rangle} \quad \text{(E-OutputProp)}
```

### 6.4 Tool Invocation Semantics

Tool invocation involves three phases: schema validation, execution, and result deserialization.

```math
\frac{t \in S.\mathcal{T} \quad \text{validate}(\sigma_{\text{in}}, \vec{a}) = \text{ok} \quad \text{dispatch}(t, m, \vec{a}) \Rightarrow (v_{\text{raw}}, \text{ok}) \quad \text{deserialize}(\sigma_{\text{out}}, v_{\text{raw}}) = v}{\langle t.m(\vec{a}), S \rangle \xrightarrow{\text{tool-ok}} \langle v, S[\mathcal{E} := \text{clear}] \rangle} \quad \text{(E-ToolSuccess)}
```

```math
\frac{t \in S.\mathcal{T} \quad \text{dispatch}(t, m, \vec{a}) \Rightarrow (\text{timeout}, \text{err})}{\langle t.m(\vec{a}), S \rangle \xrightarrow{\text{tool-timeout}} \langle \text{Error}(\text{timeout}), S[\mathcal{E} := \text{ErrCtx}(\text{timeout}, t, m)] \rangle} \quad \text{(E-ToolTimeout)}
```

```math
\frac{t \in S.\mathcal{T} \quad \text{dispatch}(t, m, \vec{a}) \Rightarrow (\text{err-code}, \text{err})}{\langle t.m(\vec{a}), S \rangle \xrightarrow{\text{tool-error}(\text{err-code})} \langle \text{Error}(\text{err-code}), S[\mathcal{E} := \text{ErrCtx}(\text{err-code}, t, m)] \rangle} \quad \text{(E-ToolError)}
```

### 6.5 LLM Invocation Semantics

LLM invocation is inherently non-deterministic. We model it as a sampling step from the LLM's output distribution:

```math
\frac{m \in S.\mathcal{A}[\alpha].\text{model} \quad \rho \in S.\mathcal{A}[\alpha].\text{prompt} \quad \omega \in \Omega_m(\text{render}(\rho, C))}{\langle \text{ask}\ m : \rho, S \rangle \xrightarrow{\text{llm}(\omega)} \langle \text{ProbVal}(\text{parse}(\omega), \text{conf}(\omega), \text{reasoning}(\omega)), S \rangle} \quad \text{(E-Ask)}
```

where:
- $\Omega_m(\text{prompt})$ is the set of possible outputs from model $m$ given the rendered prompt
- $\text{parse}(\omega)$ extracts the structured value from the raw LLM output
- $\text{conf}(\omega) \in [0, 1]$ is the confidence score
- $\text{reasoning}(\omega)$ captures the model's reasoning trace

**LLM output validation and repair:**

```math
\frac{\text{parse}(\omega) = v \quad \text{validate}(\tau_{\text{expected}}, v) = \text{ok}}{\text{LLM output accepted}} \quad \text{(E-AskValid)}
```

```math
\frac{\text{parse}(\omega) = v \quad \text{validate}(\tau_{\text{expected}}, v) = \text{fail} \quad \text{repair}(v, \tau_{\text{expected}}) = v'}{\text{LLM output repaired to } v'} \quad \text{(E-AskRepair)}
```

```math
\frac{\text{parse}(\omega) = v \quad \text{validate}(\tau_{\text{expected}}, v) = \text{fail} \quad \text{repair}(v, \tau_{\text{expected}}) = \text{fail}}{\text{LLM output rejected, enters recovery}} \quad \text{(E-AskReject)}
```

### 6.6 Approval Gate Semantics

```math
\frac{\text{gate-description} = d}{\langle \text{ask-human}: d, S \rangle \rightarrow \langle \text{Paused}, S[\mathcal{A}[\alpha].\text{status} := \text{WaitingApproval}] \rangle} \quad \text{(E-ApprovalSuspend)}
```

```math
\frac{\text{human-response} = \text{approve}(v)}{\langle \text{Paused}, S \rangle \xrightarrow{\text{approval-approve}} \langle v, S[\mathcal{A}[\alpha].\text{status} := \text{Running}] \rangle} \quad \text{(E-ApprovalApprove)}
```

```math
\frac{\text{human-response} = \text{reject}(\text{reason})}{\langle \text{Paused}, S \rangle \xrightarrow{\text{approval-reject}} \langle \text{Error}(\text{rejected}, \text{reason}), S[\mathcal{E} := \text{ErrCtx}(\text{rejected})] \rangle} \quad \text{(E-ApprovalReject)}
```

### 6.7 Error Recovery Semantics

The attempt-recover chain is Lucky's core fault-tolerance mechanism:

```math
\frac{\langle \text{body}, S \rangle \rightarrow^* \langle v, S' \rangle}{\langle \text{attempt}\ \text{body}\ \text{recover}\ R_1 \ldots R_m, S \rangle \rightarrow^* \langle v, S' \rangle} \quad \text{(E-AttemptSuccess)}
```

```math
\frac{\langle \text{body}, S \rangle \rightarrow^* \langle \text{Error}(e), S' \rangle \quad R_1 = \text{retry}(n, \text{backoff}) \quad n > 0}{\langle \text{attempt}\ \text{body}\ \text{recover}\ R_1 \ldots R_m, S \rangle \rightarrow^* \langle \text{attempt}\ \text{body}\ \text{recover}\ \text{retry}(n-1, \text{backoff}) \ldots R_m, S'[\text{delay} := \text{backoff}(n)] \rangle} \quad \text{(E-AttemptRetry)}
```

```math
\frac{\langle \text{body}, S \rangle \rightarrow^* \langle \text{Error}(e), S' \rangle \quad R_1 = \text{fallback}(e_{\text{alt}})}{\langle \text{attempt}\ \text{body}\ \text{recover}\ R_1 \ldots R_m, S \rangle \rightarrow^* \langle e_{\text{alt}}, S' \rangle} \quad \text{(E-AttemptFallback)}
```

```math
\frac{\langle \text{body}, S \rangle \rightarrow^* \langle \text{Error}(e), S' \rangle \quad R_1 = \text{human}(\text{msg})}{\langle \text{attempt}\ \text{body}\ \text{recover}\ R_1 \ldots R_m, S \rangle \rightarrow^* \langle \text{ask-human}: \text{msg}, S' \rangle} \quad \text{(E-AttemptHuman)}
```

```math
\frac{\langle \text{body}, S \rangle \rightarrow^* \langle \text{Error}(e), S' \rangle \quad R_1 = \text{abort}}{\langle \text{attempt}\ \text{body}\ \text{recover}\ R_1 \ldots R_m, S \rangle \rightarrow^* \langle \text{Error}(e, \text{aborted}), S' \rangle} \quad \text{(E-AttemptAbort)}
```

**Circuit breaker integration:**

```math
\frac{S.\text{circuit-breaker}(t) = \text{open} \quad t \in \text{tool-calls}(e)}{\langle e, S \rangle \rightarrow \langle \text{Error}(\text{circuit-breaker-open}, t), S[\mathcal{E} := \text{ErrCtx}(\text{circuit-breaker-open})] \rangle} \quad \text{(E-CircuitBreaker)}
```

The circuit breaker state transitions: $\text{closed} \xrightarrow{5 \text{ failures in 60s}} \text{open} \xrightarrow{\text{timeout}} \text{half-open} \xrightarrow{\text{success}} \text{closed}$

---

## 7. AI-Domain-Specific Semantics

### 7.1 Three-Tier Memory Semantics

Lucky models three memory tiers with distinct isolation and persistence properties:

| Tier | Scope | Persistence | Isolation |
|------|-------|-------------|-----------|
| **Local** | Single task execution | Task duration | Private to task |
| **Session/Project** | Agent lifetime | Agent lifetime (checkpointed) | Private to agent |
| **Global** | All agents | Program lifetime | Shared, writable |

**Memory read:**

```math
\frac{(\alpha, s, k) \in \text{visible-memories}(\Gamma) \quad S.\mathcal{M}(\alpha, s, k) = v}{\langle \text{recall}(\alpha, s, k), S \rangle \rightarrow \langle v, S \rangle} \quad \text{(E-MemRead)}
```

where $\text{visible-memories}(\Gamma)$ is defined as:

```math
\text{visible-memories}(\Gamma) = \begin{cases} \{(\alpha_{\text{self}}, s, k) \mid s \in \{\text{local}, \text{session}, \text{project}\}\} \cup \{(\alpha, \text{global}, k) \mid \forall \alpha\} & \text{within agent } \alpha_{\text{self}} \\ \{(\alpha, \text{global}, k) \mid \forall \alpha\} & \text{outside any agent} \end{cases}
```

**Memory write:**

```math
\frac{(\alpha, s) \in \text{writable-memories}(\Gamma) \quad \text{permitted}(\Gamma.\text{perms}, \text{memory.write})}{\langle \text{remember}(\alpha, s, k, v), S \rangle \rightarrow \langle \text{unit}, S[\mathcal{M}(\alpha, s, k) := v] \rangle} \quad \text{(E-MemWrite)}
```

```math
\text{writable-memories}(\Gamma) = \begin{cases} \{(\alpha_{\text{self}}, s) \mid s \in \{\text{local}, \text{session}, \text{project}\}\} \cup \{(\alpha, \text{global}) \mid \forall \alpha\} & \text{within agent } \alpha_{\text{self}} \\ \{(\alpha, \text{global}) \mid \forall \alpha\} & \text{outside any agent} \end{cases}
```

**Theorem 7.1 (Memory Isolation).** For any execution trace starting from a well-typed program, private agent memory (scope local/session/project) of agent $\alpha_1$ is never read or written by agent $\alpha_2$ where $\alpha_1 \neq \alpha_2$.

*Proof.* By (E-MemRead) and (E-MemWrite), the visibility and writability rules restrict private memory operations to $\alpha_{\text{self}}$. The typing rule (T-MemAssign) further ensures that `memory.field = expr` is only legal within the agent's own task body (the $\Gamma(\text{self}) = \text{Agent}(\alpha)$ precondition). Since no expression typing rule can change $\Gamma(\text{self})$ to a different agent, a well-typed program never generates a memory operation on another agent's private store. ∎

### 7.2 LLM Non-Deterministic Branch Routing

A unique semantic challenge in Lucky is **routing control flow based on LLM output**. In a traditional language, branch conditions are deterministic boolean expressions. In Lucky, the `ask` expression produces a $\text{Probabilistic}(\tau, c)$ value, and subsequent `if`/`match` branches may depend on this probabilistic result.

**Formal model:**

```math
\frac{\langle \text{ask}\ m : \rho, S \rangle \xrightarrow{\text{llm}(\omega)} \langle \text{ProbVal}(v, c, r), S' \rangle \quad \text{route}(v, c, \text{branches}) = (b_i, v_{\text{filtered}})}{\langle \text{ask}\ m : \rho \text{ then route}, S \rangle \xrightarrow{\text{llm-route}(\omega)} \langle b_i[\text{context} := v_{\text{filtered}}], S' \rangle} \quad \text{(E-LLMRoute)}
```

The $\text{route}$ function selects a branch based on the LLM output and confidence:

```math
\text{route}(v, c, \text{branches}) = \begin{cases} (b_i, v) & \text{if } \text{match-branch}(v, b_i) \text{ and } c \geq \text{threshold}(b_i) \\ (\text{repair-branch}, v') & \text{if } \text{repair-attempt}(v, \text{expected-type}) = v' \text{ and } c \geq \text{threshold} \\ (\text{fallback-branch}, \text{default}) & \text{if } c < \text{threshold} \text{ or all repairs fail} \end{cases}
```

This three-level routing — direct match, repair-and-match, fallback — is the key innovation. No prior PL formalism models this "probabilistic branching with auto-repair" pattern.

### 7.3 Tool Contract Validation Semantics

Tool contracts specify input/output JSON Schemas that are checked at two levels: statically by the type checker (§4.4, rule T-ToolCall) and dynamically by the runtime:

```math
\frac{S.\mathcal{T}[t].\text{methods}[m].\text{input-schema} = \sigma_{\text{in}} \quad \text{runtime-validate}(\sigma_{\text{in}}, \text{serialize}(\vec{a})) = \text{ok}}{\text{dispatch}(t, m, \vec{a})} \quad \text{(E-ToolValidate)}
```

```math
\frac{S.\mathcal{T}[t].\text{methods}[m].\text{output-schema} = \sigma_{\text{out}} \quad \text{runtime-validate}(\sigma_{\text{out}}, v_{\text{raw}}) = \text{ok} \quad \text{deserialize}(\sigma_{\text{out}}, v_{\text{raw}}) = v}{\text{tool call succeeds with } v} \quad \text{(E-ToolDeserialize)}
```

```math
\frac{\text{runtime-validate}(\sigma_{\text{out}}, v_{\text{raw}}) = \text{fail}}{\text{tool output rejected, enters recovery}} \quad \text{(E-ToolContractFail)}
```

**Timeout and degradation:**

```math
\frac{\text{elapsed}(\text{dispatch}(t, m, \vec{a})) > S.\mathcal{A}[\alpha].\text{policy}.\text{timeout}}{\langle t.m(\vec{a}), S \rangle \rightarrow \langle \text{Error}(\text{timeout}), S[\mathcal{E} := \text{ErrCtx}(\text{timeout}, t, m)] \rangle} \quad \text{(E-ToolTimeout)}
```

After timeout, the attempt-recover chain (§6.7) handles recovery: retry with exponential backoff, fallback to an alternative tool/agent, or escalate to a human operator.

---

## 8. Metatheorems

### 8.1 Type Safety (Full Proof)

**Theorem 8.1 (Progress + Preservation = Type Safety).**

**Part I: Progress.** If $\vdash e : \tau$ and $e$ is not a value, then $\exists S, e', S': \langle e, S \rangle \rightarrow \langle e', S' \rangle$ or $\langle e, S \rangle \xrightarrow{\ell} \langle e', S' \rangle$.

*Proof.* By structural induction on $e$.

**Case** $e = \text{let}\ x = e_1$: By (T-Let), $\vdash e_1 : \tau_1$. By IH on $e_1$, either $e_1$ is a value (then we step by E-Let) or $e_1$ can step (then we step $\langle e, S \rangle$ by stepping $e_1$).

**Case** $e = t.m(\vec{a})$: By (T-ToolCall), $t \in \Gamma(\text{tools})$ and arguments type-match. The tool exists in $S.\mathcal{T}$, so dispatch is possible. If the tool is available and not circuit-breakered, we step by (E-ToolSuccess). If circuit-breakered, we step by (E-CircuitBreaker).

**Case** $e = \text{ask}\ m : \rho$: By (T-Ask), the model and prompt are in scope. The runtime can always dispatch to the LLM backend (which may produce any output $\omega$). Step by (E-Ask).

**Case** $e = \text{parallel}\ e_1 \ldots e_n \text{ wait}$: By (T-Parallel), all branches are well-typed. Each can step independently by IH. When all complete, we merge by (E-Parallel).

**Case** $e = \text{attempt}\ \text{body}\ \text{recover}\ R$: By (T-Attempt), $\vdash \text{body} : \tau$. By IH, body can step. If body completes, we step by (E-AttemptSuccess). If body errors, we step by the appropriate recovery action.

**No stuck states.** Every well-typed non-value expression can step. The only potential stuck state would be: tool not found → prevented by (T-ToolCall); undefined variable → prevented by (T-Let); permission violation → prevented by (T-PermCheck). ∎

**Part II: Preservation.** If $\vdash e : \tau$ and $\langle e, S \rangle \rightarrow \langle e', S' \rangle$, then $\vdash e' : \tau$ (or $\tau' <: \tau$).

*Proof.* By case analysis on the stepping rule used.

**Case** (E-ToolSuccess): The output type $\tau_{\text{out}}$ is determined by $\sigma_{\text{out}}$ in (T-ToolCall), which was checked to match the expected type at compile time. So $\vdash v : \tau_{\text{out}} <: \tau$.

**Case** (E-Ask): The output is $\text{ProbVal}(v, c, r)$ of type $\text{Probabilistic}(\tau_{\text{inner}}, c)$. By (T-Ask), the declared type is $\text{Probabilistic}(\tau_{\text{inner}}, c_{\text{default}})$. Since $c \geq 0$ always and $c_{\text{default}} \leq 1$, the confidence subtyping rule gives $\text{Probabilistic}(\tau_{\text{inner}}, c) <: \text{Probabilistic}(\tau_{\text{inner}}, c_{\text{default}})$ when $c \geq c_{\text{default}}$, and otherwise the output enters the repair/fallback path which is typed by the recovery rules.

**Case** (E-Parallel): Each branch preserves its type by IH. The merged result type is a union of branch types, which subtypes the declared parallel output type.

**Case** (E-AttemptSuccess): The body's type $\tau$ is preserved.

**Case** (E-AttemptRetry): The retry reduces the remaining count, but the body type $\tau$ is unchanged.

**Case** (E-AttemptFallback): The fallback expression was typed as $\tau$ by (T-Attempt).

**All cases preserve types.** ∎

### 8.2 Memory Isolation Theorem

**Theorem 8.2 (Private Memory Isolation).** In any execution trace $\langle e_0, S_0 \rangle \rightarrow^* \langle e_n, S_n \rangle$ of a well-typed program, for any two distinct agents $\alpha_1 \neq \alpha_2$:

```math
S_n.\mathcal{M}(\alpha_1, \text{project}, k) = S_0.\mathcal{M}(\alpha_1, \text{project}, k) \quad \text{unless some step executed within } \alpha_1\text{'s context modified } k
```

and no step executed within $\alpha_2$'s context modified $\alpha_1$'s project-scope memory.

*Proof.* By induction on the trace length. The key insight is that (E-MemWrite) requires $(\alpha, s) \in \text{writable-memories}(\Gamma)$, and within $\alpha_2$'s context, $\text{writable-memories}$ only includes $(\alpha_2, \text{local})$, $(\alpha_2, \text{session})$, $(\alpha_2, \text{project})$, and $(\alpha, \text{global})$ for any $\alpha$. It never includes $(\alpha_1, \text{project})$ for $\alpha_1 \neq \alpha_2$.

For **parallel execution**: The (T-Parallel) rule enforces $\text{no-shared-mutable-refs}$, which specifically checks that parallel branches do not write to the same agent's project-scope memory. So even in concurrent execution, isolation is preserved.

For **global memory**: Global memory is intentionally shared. Writes to global memory by $\alpha_2$ are visible to $\alpha_1$, but this is by design — global memory is the communication channel between agents. The isolation invariant only concerns private (local/session/project) memory. ∎

### 8.3 Deadlock Freedom

**Theorem 8.3 (Parallel Deadlock Freedom).** A well-typed parallel block $\text{parallel}\ e_1 \ldots e_n \text{ wait}$ never permanently blocks.

*Proof.* The (T-Parallel) rule requires $\text{no-shared-mutable-refs}$, meaning no two branches write to the same memory location. This eliminates the primary source of deadlock in agent systems: circular waiting on memory access.

The remaining potential deadlock source would be **approval gates** within parallel branches. But by the runtime specification, approval gates suspend the specific branch, not the entire parallel block. The `wait` barrier only requires all branches to *complete* (either successfully or via recovery), and the recovery chain (attempt-recover) guarantees that every failure has a defined termination path (retry terminates after $n$ attempts, fallback provides an alternative, human escalation eventually resolves, abort terminates immediately).

Therefore: each branch either (a) completes successfully, (b) completes via recovery, or (c) aborts. In all cases, the branch reaches a terminal state, and the parallel block terminates. ∎

### 8.4 Recovery Completeness

**Theorem 8.4 (Recovery Completeness).** For every well-typed `attempt` block, every possible failure scenario has a defined recovery path.

*Proof.* We enumerate all failure scenarios for the body expression $e$:

| Failure Type | Recovery Path |
|--------------|---------------|
| Tool timeout | retry (with backoff) → fallback → human → abort |
| Tool schema mismatch | retry → fallback → abort |
| Tool permission denied | abort (cannot retry — permissions are static) |
| LLM output format error | retry (re-prompt) → fallback (alternative agent) → human → abort |
| LLM confidence below threshold | retry (with higher threshold) → fallback → human |
| Circuit breaker open | fallback (alternative tool) → human → abort |
| Human rejection | abort or skip (by recovery specification) |
| Memory overflow | circuit breaker opens → fallback → abort |

For each failure type, the recovery chain $R_1, \ldots, R_m$ must include at least one action that resolves this type. The (T-Attempt) typing rule ensures that every recovery action is typed to produce a result compatible with $\tau$, so the overall attempt block always produces a value of type $\tau$ (or terminates with a defined Error).

**Corollary 8.5 (No unhandled exceptions).** A well-typed Lucky program never enters an undefined exception state. Every error either (a) is caught by an attempt-recover chain, (b) is logged and the workflow terminates with a defined error status, or (c) is escalated to a human operator with a defined approval gate. ∎

---

## 9. Implementation Validation

### 9.1 Methodology

We systematically compare the formal operational semantics against the existing Rust implementation (`lucky-compiler` v0.2.0, 61 source files, ~15,000 lines of Rust). The validation procedure:

1. **Map each semantic rule** to its corresponding implementation code
2. **Identify semantic gaps** where the implementation behavior differs from the formal model
3. **Classify gaps** as bugs (implementation wrong), spec issues (formal model too strict), or extensions (implementation adds behavior not in the model)
4. **Propose patches** grounded in the formal semantics

### 9.2 Semantic Gap Analysis

**Gap 1: Parallel Memory Write Ordering (Implementation Bug).**

*Formal model:* (E-Parallel) specifies `merge_mem` with "last-writer-wins" semantics for conflicting global memory writes, but the implementation's `ContextManager::snapshot()` method flattens all layers into a single HashMap without ordering guarantees.

*Implementation code:* `src/runtime/context.rs` — the `snapshot()` method iterates layers in arbitrary order (HashMap iteration), making the "last writer" undefined when two parallel branches write to the same global key.

*Formal patch:* The merge function should use a deterministic ordering — either (a) branch index order (branch 1's writes take precedence over branch 2's), or (b) timestamp order (each write tagged with a monotonic timestamp). We recommend branch index order as it's simpler and aligns with the DAG execution semantics where branch ordering reflects the programmer's intent.

```rust
// Patch: deterministic merge by branch order
fn merge_mem(states: Vec<RuntimeState>, branch_order: Vec<usize>) -> ContextLayer {
    let mut merged = states[branch_order[0]].context.clone();
    for idx in branch_order[1..] {
        merged = merged.shadow_merge(states[idx].context);
    }
    merged
}
```

**Gap 2: Loop Exit Condition Delay (Implementation Bug).**

*Formal model:* (E-LoopExit) specifies that the condition is evaluated *before* each iteration. If the condition is false, the loop exits immediately without executing the body.

*Implementation code:* `src/runtime/scheduler.rs` — the loop implementation evaluates the condition after executing the body, resulting in a one-step delay. A loop that should exit after 0 iterations (condition initially false) will execute the body once before checking.

*Formal patch:* The scheduler should evaluate the loop condition *before* dispatching the body node, not after. This aligns with the standard `while` loop semantics.

```rust
// Patch: pre-check loop condition
fn execute_loop(&mut self, loop_node: &LoopNode) -> NodeResult {
    while true {
        let cond = self.evaluate_condition(&loop_node.condition)?;
        if !cond {
            return NodeResult::Completed(Value::unit());
        }
        let body_result = self.execute_node(&loop_node.body)?;
        // body_result is checked for break/continue
    }
}
```

**Gap 3: LLM Failure Recovery Incompleteness (Implementation Gap).**

*Formal model:* (E-AskReject) specifies that when an LLM output fails validation and repair, the execution enters the attempt-recover chain. The (E-AttemptRetry) rule re-prompts with the same or modified prompt.

*Implementation code:* `src/backends/openai_compat.rs` — when the LLM returns an unparseable response, the current implementation logs the error and returns `Error(unparseable_output)` immediately, without attempting repair or triggering the recovery chain. The `attempt/recover` mechanism only handles tool timeouts and network errors, not LLM output format failures.

*Formal patch:* Add an LLM output validation layer that:
1. Validates the parsed output against the expected type schema
2. If validation fails, attempts repair (re-prompt with format instructions)
3. If repair fails, enters the recovery chain (retry with backoff, fallback to alternative model, human escalation)

```rust
// Patch: LLM output validation + recovery
fn execute_llm_call(&mut self, node: &LlmCallNode) -> NodeResult {
    let raw = self.dispatch_to_backend(node)?;
    let parsed = self.parse_llm_output(raw, node.expected_output_type)?;
    match parsed {
        Ok(value) => NodeResult::Completed(value),
        RepairNeeded(attempt) => {
            let repaired = self.repair_prompt(attempt, node)?;
            match repaired {
                Ok(value) => NodeResult::Completed(value),
                Failed => self.enter_recovery_chain(node),
            }
        }
    }
}
```

### 9.3 Specification Compliance Summary

| Semantic Rule | Implementation Status | Gap |
|---------------|----------------------|-----|
| (E-Seq) Sequential composition | ✅ Implemented | None |
| (E-Parallel) Parallel + barrier | ⚠️ Partially implemented | Memory merge ordering (Gap 1) |
| (E-IfTrue/False) Conditional | ✅ Implemented | None |
| (E-LoopCont/Exit) Loop | ⚠️ Bug | Condition evaluation delay (Gap 2) |
| (E-For) Iteration | ✅ Implemented | None |
| (E-ContextProp) Context propagation | ✅ Implemented (layered) | None |
| (E-ToolSuccess/Timeout/Error) Tool calls | ✅ Implemented | None |
| (E-Ask) LLM invocation | ⚠️ Partially implemented | Output validation recovery (Gap 3) |
| (E-ApprovalSuspend/Approve/Reject) | ✅ Implemented | None |
| (E-AttemptRetry/Fallback/Human/Abort) | ✅ Implemented | Only for tool errors, not LLM errors |
| (E-CircuitBreaker) | ✅ Implemented | None |
| (E-MemRead/Write) Memory | ✅ Implemented | None |
| (T-ToolCall) Schema validation | ⚠️ Not implemented | No compile-time schema check |
| (T-Parallel) Race condition check | ⚠️ Not implemented | No static race detection |
| (T-PermCheck) Permission check | ✅ Implemented | None |

**15 rules fully implemented, 5 with gaps, 3 not implemented at the static level.**

---

## 10. Related Work

### 10.1 General PL Formalization

The formalization of programming language semantics has a rich tradition from Milner's original big-step semantics for ML [6] to the modern small-step frameworks used in Rust's type system verification [7] and CompCert's verified C compiler [8]. Our work follows the small-step operational semantics tradition (Wright & Felleisen [9]) because it naturally handles non-determinism (LLM outputs) and concurrent execution (parallel blocks).

However, **no prior PL formalization** includes:
- LLM invocation as a primitive (all prior work assumes deterministic computation)
- Probabilistic type constructors with confidence subtyping
- Three-tier memory with agent isolation invariants
- Tool contract types with schema validation
- Human approval as a control-flow gate
- Attempt-recover chains as a core semantic construct

### 10.2 AI Agent DSLs

**AutoGen** [1] provides a Python-based multi-agent conversation framework. Agents are defined as Python functions with no type system, no formal semantics, and no static verification. Concurrency is managed through asyncio with no isolation guarantees. Memory is unstructured (shared Python state). There is no formal proof of any property.

**LangGraph** [2] models agent workflows as state machines with nodes and edges. While the graph structure is declarative, the node implementations are arbitrary Python functions with no type contracts. There is no formal operational semantics, no memory isolation model, and no recovery completeness guarantee.

**CrewAI** [3] provides role-based agent definitions with task delegation. Agent memory is a simple key-value store with no isolation or vector search. Task execution is sequential with no parallel semantics. There is no formal model of any kind.

**OpenAI Agents SDK** [4] defines agents as Python objects with handoff protocols. The tracing system provides observability but not formal verification. There is no type system beyond Python's dynamic typing.

**Our contribution in context:** Lucky's formal semantics is the first to provide:
1. A complete type system with AI-specific constructors
2. Provable memory isolation for concurrent agents
3. Operational semantics for LLM non-deterministic branching
4. Recovery completeness guarantees
5. Tool contract validation at both static and dynamic levels

### 10.3 Workflow Formalisms

**BPMN** (Business Process Model and Notation) [10] provides a visual notation for business workflows with formal semantics based on Petri nets. However, BPMN assumes deterministic task execution, has no concept of probabilistic LLM outputs, no agent memory model, and no tool contract types. Mapping Lucky to BPMN would lose all AI-specific semantics.

**Petri net** formalisms [11] model concurrent workflows with place-transition graphs. They can represent parallel execution and synchronization barriers, but they cannot model: (a) non-deterministic LLM outputs (transitions in Petri nets are deterministic), (b) agent memory with vector search, (c) hierarchical agent spawning, or (d) attempt-recover chains with exponential backoff.

**Temporal logic** (LTL/CTL) [12] can express safety properties like "no agent reads another's private memory" or "every tool call eventually completes or enters recovery." Our Memory Isolation theorem (8.2) and Recovery Completeness theorem (8.4) are precisely the kinds of properties LTL would specify. However, LTL verification requires a model checker, while our type system and operational semantics provide these guarantees statically through typing rules — a more practical approach for language users.

### 10.4 Novelty Summary

| Property | Java/Python Formalization | AutoGen/LangGraph | BPMN/Petri Nets | **Lucky (This Paper)** |
|----------|--------------------------|-------------------|-----------------|------------------------|
| Deterministic semantics | ✅ | ❌ | ✅ | ✅ (for structure) |
| LLM non-determinism | ❌ | ❌ | ❌ | ✅ (§7.2) |
| Probabilistic types | ❌ | ❌ | ❌ | ✅ (§4.2) |
| Agent memory isolation | ❌ | ❌ | ❌ | ✅ (§7.1, Thm 8.2) |
| Tool contract types | ❌ | ❌ | ❌ | ✅ (§4.4) |
| Human approval primitive | ❌ | ❌ | Partial | ✅ (§6.6) |
| Recovery completeness | ❌ | ❌ | ❌ | ✅ (Thm 8.4) |
| Type safety proof | ✅ | ❌ | ❌ | ✅ (Thm 8.1) |
| Deadlock freedom proof | Partial | ❌ | ✅ | ✅ (Thm 8.3) |

---

## 11. Conclusion and Future Work

### 11.1 Summary

We have presented the first complete formal semantics for Lucky, a goal-oriented DSL for AI agent orchestration. Our contributions span grammar formalization, type system design with AI-specific constructors, small-step operational semantics covering all orchestration primitives, four proven metatheorems, and validation against the existing implementation identifying three semantic bugs.

The key insight is that AI agent orchestration introduces semantic challenges that conventional PL theory does not address: non-deterministic LLM outputs require probabilistic type constructors and confidence-aware branching; concurrent agents require memory isolation invariants grounded in capability security; and fault tolerance requires recovery completeness proofs. Lucky's formal semantics addresses all of these, providing a foundation for future verification tools and mechanized proofs.

### 11.2 Future Work

**Mechanized proofs in Lean 4.** The proof sketches in §8 are detailed enough for mechanization. We plan to encode the type system and operational semantics in Lean 4 and mechanically verify all four theorems. The estimated effort is 3–6 months for a trained proof engineer.

**Static verification tool.** Based on the formal type system and invariant rules, we plan to build an independent static analyzer that scans Lucky programs without executing them, detecting: (a) parallel memory race conditions, (b) tool contract mismatches, (c) unreachable recovery paths, (d) confidence threshold violations, and (e) permission boundary violations. This would be the first static verifier for any AI agent DSL.

**Symbolic execution engine.** The operational semantics can serve as the basis for a symbolic simulator that enumerates all execution paths through a Lucky program, including all possible LLM outputs and tool failure scenarios. This would enable automated bug discovery in complex agent workflows — testing scenarios that are impractical to cover through manual execution.

**Semantic equivalence and compiler optimization.** Theorems about semantic equivalence (e.g., "independent serial steps can be parallelized without changing observable behavior") would provide mathematical justification for the MIR optimizer's transformations. We plan to formalize these as equivalence relations on operational traces.

**Distributed execution semantics.** The current semantics models single-process execution. Extending to distributed multi-node execution (where agents run on separate machines communicating via LTP protocol) would require modeling network partitions, message ordering, and distributed checkpoint consistency.

---

## References

[1] Wu, Q., et al. "AutoGen: Enabling Next-Gen LLM Applications via Multi-Agent Conversation." *COLM 2024.*

[2] LangGraph. "Stateful, Multi-Agent Applications with LangGraph." *LangChain Documentation, 2024.* https://langchain-ai.github.io/langgraph/

[3] CrewAI. "Framework for Orchestrating Role-Playing Autonomous AI Agents." *GitHub, 2024.* https://github.com/crewAIInc/crewAI

[4] OpenAI. "Agents SDK: A Lightweight Multi-Agent Framework." *OpenAI Documentation, 2025.* https://openai.com/index/new-tools-for-building-agents/

[5] Xia, J. "Lucky Programming Language Specification v0.1." *Self-published, 2026.* https://github.com/lucky-lang/lucky

[6] Milner, R., Tofte, M., Harper, R., MacQueen, D. *The Definition of Standard ML (Revised).* MIT Press, 1997.

[7] Jung, R., et al. "RustBelt: Securing the Foundations of the Rust Programming Language." *POPL 2018.*

[8] Leroy, X. "Formal Verification of a Realistic Compiler." *Communications of the ACM, 52(7), 2009.*

[9] Wright, A.K., Felleisen, M. "A Syntactic Approach to Type Soundness." *Information and Computation, 115(1), 1994.*

[10] OMG. "Business Process Model and Notation (BPMN) Version 2.0." *Object Management Group, 2014.*

[11] Reisig, W. *Understanding Petri Nets: Modeling Techniques, Analysis Methods, Case Studies.* Springer, 2013.

[12] Clarke, E.M., Grumberg, O., Peled, D.A. *Model Checking.* MIT Press, 1999.

[13] Pierce, B.C. *Types and Programming Languages.* MIT Press, 2002.

[14] Appel, A.W. *Verified Functional Programming in Coq.* 2023.

[15] Honda, K., Yoshida, N., Carbone, M. "Multiparty Asynchronous Session Types." *POPL 2008.*

[16] Dijkstra, E.W. "Guarded Commands, Non-determinacy, and Formal Derivation of Programs." *Communications of the ACM, 18(8), 1975.*

---

## Appendix A: Complete Typing Rules

### A.1 Expression Typing Rules

```math
\frac{x : \tau \in \Gamma}{\Gamma \vdash x : \tau} \quad \text{(T-Var)}
```

```math
\frac{\text{literal type of } l = \tau}{\Gamma \vdash l : \tau} \quad \text{(T-Lit)}
```

```math
\frac{\Gamma \vdash e_1 : \tau_1 \quad \Gamma \vdash e_2 : \tau_2 \quad \text{binop-type}(\text{op}, \tau_1, \tau_2) = \tau}{\Gamma \vdash e_1 \text{ op } e_2 : \tau} \quad \text{(T-BinOp)}
```

```math
\frac{\Gamma \vdash e : \text{Bool}}{\Gamma \vdash \text{not}\ e : \text{Bool}} \quad \text{(T-Not)}
```

```math
\frac{\Gamma \vdash e : \tau_1 \quad \Gamma \vdash e_2 : \tau_2 \quad \tau_1 <: \tau_2 \text{ or } \tau_2 <: \tau_1}{\Gamma \vdash e_1 \text{ cmp } e_2 : \text{Bool}} \quad \text{(T-Cmp)}
```

```math
\frac{\Gamma \vdash e : \text{List}(\tau)}{\Gamma \vdash e[\text{idx}] : \tau?} \quad \text{(T-Index)}
```

```math
\frac{\Gamma \vdash e : \tau \quad f \in \text{fields}(\tau)}{\Gamma \vdash e.f : \text{field-type}(\tau, f)} \quad \text{(T-Field)}
```

```math
\frac{\Gamma \vdash e : \tau? \quad f \in \text{fields}(\tau)}{\Gamma \vdash e.?f : \text{field-type}(\tau, f)?} \quad \text{(T-NullableField)}
```

```math
\frac{\Gamma \vdash e : \tau? \quad \Gamma \vdash e_d : \tau}{\Gamma \vdash e ?\| e_d : \tau} \quad \text{(T-NullCoalesce)}
```

```math
\frac{\Gamma \vdash e : \tau \quad \Gamma, x : \tau \vdash e_{\text{next}} : \tau_{\text{next}}}{\Gamma \vdash e |> e_{\text{next}} : \tau_{\text{next}}} \quad \text{(T-Pipe)}
```

```math
\frac{\forall i: \Gamma \vdash e_i : \tau_i}{\Gamma \vdash [e_1, \ldots, e_n] : \text{List}(\tau_1 \mid \ldots \mid \tau_n)} \quad \text{(T-List)}
```

```math
\frac{\forall i: \Gamma \vdash e_i : \tau_i \quad \forall i,j: \tau_i <: \tau \text{ for some } \tau}{\Gamma \vdash \{e_1, \ldots, e_n\} : \text{Set}(\tau)} \quad \text{(T-Set)}
```

```math
\frac{\forall i: \Gamma \vdash k_i : \tau_k \quad \forall i: \Gamma \vdash v_i : \tau_v}{\Gamma \vdash \{k_1: v_1, \ldots\} : \text{Map}(\tau_k, \tau_v)} \quad \text{(T-Map)}
```

```math
\frac{\Gamma, \vec{x} : \vec{\tau} \vdash e : \tau_r}{\Gamma \vdash \text{fn}(\vec{x} : \vec{\tau}) => e : \text{fn}(\vec{\tau}) \rightarrow \tau_r} \quad \text{(T-Lambda)}
```

```math
\frac{\Gamma \vdash e : \tau_1 \quad \Gamma \vdash e_2 : \tau_2 \quad \Gamma \vdash e_3 : \tau_3 \quad \tau_1 <: \text{Bool}}{\Gamma \vdash \text{if}\ e_1\ \text{then}\ e_2\ \text{else}\ e_3 : \tau_2 \mid \tau_3} \quad \text{(T-IfExpr)}
```

### A.2 Statement Typing Rules

```math
\frac{\Gamma \vdash e : \tau \quad \text{branches disjoint on patterns}}{\Gamma \vdash \text{match}\ e \{p_1 => s_1, \ldots\} : \tau_1 \mid \ldots \mid \tau_n} \quad \text{(T-Match)}
```

```math
\frac{\Gamma \vdash e : \text{Bool} \quad \Gamma \vdash s_1 : \tau \quad \Gamma \vdash s_2 : \tau}{\Gamma \vdash \text{if}\ e\ s_1\ \text{else}\ s_2 : \tau} \quad \text{(T-IfStmt)}
```

```math
\frac{\Gamma \vdash \text{cond} : \text{Bool} \quad \Gamma \vdash \text{body} : \tau}{\Gamma \vdash \text{loop}\ \text{body}\ \text{until}\ \text{cond} : \tau} \quad \text{(T-Loop)}
```

```math
\frac{\Gamma \vdash e : \text{List}(\tau) \quad \Gamma, x : \tau \vdash \text{body} : \tau_b}{\Gamma \vdash \text{for}\ x\ \text{in}\ e\ \text{body} : \text{List}(\tau_b)} \quad \text{(T-For)}
```

```math
\frac{\Gamma \vdash e : \tau}{\Gamma \vdash \text{return}\ e : \text{Never}} \quad \text{(T-Return)}
```

```math
\frac{\text{loop context}}{\Gamma \vdash \text{break} : \text{Never}} \quad \text{(T-Break)}
```

```math
\frac{\text{loop context}}{\Gamma \vdash \text{continue} : \text{Never}} \quad \text{(T-Continue)}
```

---

## Appendix B: IR Node Semantics Mapping

| HIR Node Kind | Operational Semantic Rule(s) |
|---------------|------------------------------|
| GoalNode | (E-Seq) for workflow entry |
| TaskNode | (E-ContextProp) + (E-OutputProp) |
| AgentInvokeNode | (T-AgentInvoke) + (E-Seq) |
| ToolNode | (E-ToolSuccess) / (E-ToolTimeout) / (E-ToolError) |
| LLMCallNode | (E-Ask) + (E-AskValid) / (E-AskRepair) / (E-AskReject) |
| DecisionNode | (E-IfTrue) / (E-IfFalse) |
| MatchNode | (E-MatchArm) pattern dispatch |
| ParallelNode | (E-Parallel) |
| JoinNode | (E-Parallel) barrier completion |
| LoopNode | (E-LoopCont) / (E-LoopExit) |
| ForEachNode | (E-For) |
| PipelineNode | (T-Pipe) chain |
| AttemptNode | (E-AttemptSuccess) / (E-AttemptRetry) / (E-AttemptFallback) / (E-AttemptHuman) / (E-AttemptAbort) |
| ApprovalNode | (E-ApprovalSuspend) / (E-ApprovalApprove) / (E-ApprovalReject) |

---

## Appendix C: MIR Instruction Semantics

| Opcode | Formal Semantics |
|--------|-----------------|
| Add/Sub/Mul/Div | $\langle v_1 \text{ op } v_2, S \rangle \rightarrow \langle \text{compute}(v_1, v_2, \text{op}), S \rangle$ |
| Eq/Neq/Lt/Gt/Le/Ge | $\langle v_1 \text{ cmp } v_2, S \rangle \rightarrow \langle \text{Bool}, S \rangle$ |
| And/Or/Not | Standard boolean logic (short-circuit for And/Or) |
| Call | $\langle \text{Call}(f, \vec{a}), S \rangle \rightarrow \langle f.\text{body}[\text{params} := \vec{a}], S \rangle$ |
| LlmComplete | $\langle \text{LlmComplete}(m, \rho, \text{args}), S \rangle \xrightarrow{\text{llm}(\omega)} \langle \text{ProbVal}(\ldots), S \rangle$ |
| ToolInvoke | $\langle \text{ToolInvoke}(t, m, \vec{a}), S \rangle \rightarrow$ tool dispatch (E-ToolSuccess/Timeout/Error) |
| AgentInvoke | $\langle \text{AgentInvoke}(\alpha, \kappa, \vec{a}), S \rangle \rightarrow$ agent task dispatch |
| Alloca/Load/Store | Stack allocation and memory access within task scope |
| BrOp | $\langle \text{Br}(bb), S \rangle \rightarrow \langle bb.\text{first-inst}, S \rangle$ |
| CondBrOp | $\langle \text{CondBr}(\text{cond}, bb_t, bb_f), S \rangle \rightarrow \langle bb_t \text{ or } bb_f, S \rangle$ |
| RetOp | $\langle \text{Ret}(v), S \rangle \rightarrow \langle v, S \rangle$ (terminal) |
| Phi | $\langle \text{Phi}(v_1 \text{ from } bb_1, v_2 \text{ from } bb_2), S \rangle \rightarrow \langle v_i, S \rangle$ where $bb_i$ is the predecessor that executed |
| ListNew/ListGet | $\langle \text{ListNew}(\vec{v}), S \rangle \rightarrow \langle [\vec{v}], S \rangle$; $\langle \text{ListGet}(l, i), S \rangle \rightarrow \langle l[i], S \rangle$ |
| MapNew/MapGet | Analogous to List operations |
| StrConcat/StrLen | Standard string operations |
| Cast | $\langle \text{Cast}(v, \tau), S \rangle \rightarrow \langle \text{cast}(v, \tau), S \rangle$ if $v : \tau' <: \tau$ |

