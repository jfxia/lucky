# Lucky v0.4 — Production Scale: Design Plan

<img src="../../logo/logo128.png" alt="Lucky logo" width="64" align="right" />

**Version:** 0.4 Draft  
**Status:** Design Plan  
**Based on:** v0.3 Embeddable Runtime + Dynamic Sub-Agent System  

---

## Table of Contents

1. [Strategic Overview](#1-strategic-overview)
2. [Architecture Vision](#2-architecture-vision)
3. [Work Packages](#3-work-packages)
   - [A: Advanced Orchestration Patterns](#a-advanced-orchestration-patterns-30)
   - [B: Advanced Optimizer & IR](#b-advanced-optimizer--ir-25)
   - [C: Ecosystem & Platform](#c-ecosystem--platform-25)
   - [D: Advanced Security](#d-advanced-security-20)
4. [Timeline & Milestones](#4-timeline--milestones)
5. [Design Decisions](#5-design-decisions)
6. [Risk Assessment](#6-risk-assessment)
7. [Success Criteria](#7-success-criteria)

---

## 1. Strategic Overview

### 1.1 Where v0.3 Leaves Us

v0.3 transforms Lucky from a single-node orchestration engine into an **embeddable platform**:

| Capability | v0.3 Target |
|---|---|
| LTP Embedding C SDK — any platform can link Lucky in 5 minutes | ✅ |
| MCP bridge — Lucky speaks the universal agent-tool protocol | ✅ |
| Docker sandbox — tool isolation for production trust | ✅ |
| Standard library runtime — String, List, Map, ai, http methods | ✅ |
| Dynamic sub-agent system — `register agent`, `isolate`, `mount` | ✅ |
| Platform adapters with CI — Claude Code, WorkBuddy, Windsurf | ✅ |
| 3+ new platform integrations | ✅ |
| Backward compatible with v0.2 | ✅ |

### 1.2 v0.4 Goal

> **Scale Lucky from a platform-friendly runtime to a production-grade system with advanced orchestration, industrial-strength optimization, and a full cloud ecosystem.**

v0.4 is about **depth and scale**:
- **Orchestration depth**: multi-level delegation, contract enforcement, adaptive workflows
- **Compiler maturity**: GVN, LICM, inlining, LIR, binary IR
- **Cloud-native deployment**: K8s operator, Lucky Cloud, package registry
- **Security hardening**: Firecracker, full mTLS, SIEM integration

### 1.3 Guiding Principles

1. **Backward compatibility first.** v0.3 workflows must run on v0.4 without changes.
2. **Proven patterns, not speculation.** Every new feature justifies itself against real usage data from v0.3.
3. **Dogfood Lucky.** The Lucky team's own CI/CD, documentation generation, and release pipeline run on Lucky workflows.
4. **Enterprise-ready.** Security, audit, operator — these unlock enterprise adoption.

---

## 2. Architecture Vision

### 2.1 v0.4 Component Map

```
┌──────────────────────────────────────────────────────────────────────┐
│                       Lucky v0.4 System                              │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              Advanced Orchestration Layer                     │   │
│  │                                                               │   │
│  │  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────┐  │   │
│  │  │ Multi-level │ │ Contract   │ │ Auto-      │ │ Parallel │  │   │
│  │  │ Delegation │ │ Enforcement│ │ Rethink    │ │ Patterns │  │   │
│  │  └────────────┘ └────────────┘ └────────────┘ └──────────┘  │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              Compiler Pipeline (Enhanced)                     │   │
│  │                                                               │   │
│  │  Lexer → AST → Semantic → HIR → MIR → Opt → LIR → Exec     │   │
│  │                                   │                           │   │
│  │  New: GVN, LICM, Inlining       │ LIR: linear IR            │   │
│  │       AI opt, Critical Path      │ Binary: FlatBuffers .lkr  │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              Cloud Ecosystem                                  │   │
│  │                                                               │   │
│  │  ┌─────────────────┐ ┌──────────────────┐ ┌──────────────┐  │   │
│  │  │  K8s Operator   │ │  Package Registry│ │ Lucky Cloud  │  │   │
│  │  │  (CRD + Jobs)   │ │  (OCI + Ed25519) │ │ (REST API)   │  │   │
│  │  └─────────────────┘ └──────────────────┘ └──────────────┘  │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              Security (Hardened)                              │   │
│  │                                                               │   │
│  │  ┌─────────────────┐ ┌──────────────────┐ ┌──────────────┐  │   │
│  │  │  Firecracker    │ │  Full mTLS       │ │  SIEM Export │  │   │
│  │  │  (VM isolation) │ │  (all channels)  │ │  (OTel/Splunk)│  │   │
│  │  └─────────────────┘ └──────────────────┘ └──────────────┘  │   │
│  └──────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
```

### 2.2 Key New Interfaces

```rust
/// Multi-level delegation context — tracks the delegation tree
pub struct DelegationContext {
    pub run_id: Uuid,
    pub parent_span: Option<SpanContext>,
    pub delegation_depth: u32,
    pub max_depth: u32,           // default: 5
    pub inherited_context: HashMap<String, RuntimeValue>,
}

/// Contract — the I/O shape an agent promises
pub struct AgentContract {
    pub name: String,
    pub inputs: Vec<ContractField>,    // name + type + optional
    pub outputs: Vec<ContractField>,
    pub min_confidence: Option<f64>,  // reject below this
}

pub struct ContractField {
    pub name: String,
    pub type_expr: TypeExpr,
    pub description: String,
}

/// Adaptive policy — auto-rethink on failure
pub struct AdaptivePolicy {
    pub rethink_on: Vec<FailureKind>,   // partial_failure, timeout, low_confidence
    pub max_rethink: u32,               // max re-plan attempts
    pub escalate_on_stuck: bool,
    pub rethink_timeout_secs: u64,
}
```

---

## 3. Work Packages

### A) Advanced Orchestration Patterns (30% of effort)

Build on the v0.3 dynamic sub-agent system with deeper orchestration capabilities.

| # | Feature | Effort | Description |
|---|---|---|---|
| **A1** | Multi-level delegation | L | Sub-agents can also use the `task` tool to delegate to other sub-agents. Runtime tracks a delegation tree (not just a flat DAG). Maximum depth enforced by config. |
| **A2** | Contract enforcement | M | Runtime validates agent I/O contracts. If an agent declares `output ResearchBrief`, the runtime checks that the actual output matches the declared shape. Low confidence → re-delegate. |
| **A3** | Auto-rethink / adaptive workflows | M | `policy AdaptivePolicy { rethink on partial_failure; max_rethink 3; escalate_on_stuck }`. When a node fails, the orchestrator can re-plan — choose a different sub-agent or try a different approach — instead of just retrying. |
| **A4** | Parallel sub-agent patterns | M | Built-in composite patterns: `split` (divide input across N agents), `aggregate` (merge outputs), `vote` (majority consensus), `refine` (iterative improvement). |

#### A1 — Multi-Level Delegation

```lucky
workflow ArticleProduction
  // Level 1: Editor-in-Chief delegates to team leads
  register agent EditorInChief
    model DeepSeek
    tools task              // has the power to delegate
    prompt "You coordinate the writing pipeline..."

  // Level 2: Researcher delegates to sub-specialists
  register agent Researcher
    model DeepSeek
    tools task, Browser     // can also delegate
    prompt "..."

  // Level 3: Fact-checkers (leaf agents — no task tool)
  register agent FactChecker
    model DeepSeek
    tools Browser           // no task tool — cannot delegate further

  EditorInChief.run("Write a technical article about Lucky")
  // Editor delegates: researcher -> planner -> writer -> reviewer
  // Researcher internally delegates: deep_search -> code_verify -> fact_check
```

**Runtime model:**
```
Run
└── EditorInChief (depth=1)
    ├── Researcher (depth=2)
    │   ├── DeepSearch (depth=3)
    │   ├── CodeVerify (depth=3)
    │   └── FactCheck (depth=3)
    ├── Planner (depth=2)
    ├── Writer (depth=2)
    └── Reviewer (depth=2)
```

**Safety:** Maximum delegation depth is configurable (default: 5). Circular delegation detection at compile time. The `task` tool is a capability — only agents with it can delegate.

#### A2 — Contract Enforcement

```lucky
type ResearchBrief = {
  findings: List<String>,
  sources: List<Source>,
  confidence: Float,
}

agent Researcher
  model DeepSeek
  output ResearchBrief       // declared contract
  tools Browser

workflow ResearchTask
  let brief = Researcher.search("async Rust patterns")
  // Runtime validates: brief.shape == ResearchBrief
  // If brief.confidence < 0.6 → re-delegate with "needs more sources"
  // If brief is missing fields → error with clear message
```

**Runtime validation:**
```rust
fn validate_contract(output: &RuntimeValue, contract: &AgentContract) -> Result<(), ContractError> {
    let shape = output.type_shape();
    for field in &contract.outputs {
        if !shape.has_field(&field.name) {
            return Err(ContractError::MissingField {
                field: field.name.clone(),
                expected_type: field.type_expr.clone(),
            });
        }
    }
    if let Some(min_conf) = contract.min_confidence {
        if output.confidence() < min_conf {
            return Err(ContractError::LowConfidence {
                actual: output.confidence(),
                required: min_conf,
            });
        }
    }
    Ok(())
}
```

#### A3 — Auto-Rethink / Adaptive Workflows

```lucky
policy AdaptivePolicy
  rethink on
    partial_failure        // agent hit a tool error or got stuck
    timeout                // exceeded time limit
    low_confidence         // output confidence below threshold
  max_rethink 3
  escalate_on_stuck true   // if 3 rethinks fail, escalate to human
  rethink_timeout 2m       // each rethink gets 2 minutes
```

**How it works:**
1. A node fails (timeout, error, or low confidence)
2. Instead of just retrying the same task, the runtime calls the orchestrator agent's `rethink` entry
3. The orchestrator can: choose a different sub-agent, reformulate the prompt, split the task, or escalate
4. If `max_rethink` is exceeded, execution escalates to a human approval gate

#### A4 — Parallel Sub-Agent Patterns

```lucky
// Split: divide workload across N identical agents
let results = split(100 files) across 5 Reviewer.check(files)
// Each Reviewer gets 20 files. Results are collected.

// Aggregate: merge outputs from multiple agents
let final_report = aggregate(results) using Synthesizer.merge
// Synthesizer receives all outputs and produces a unified result

// Vote: run N agents and take majority consensus
let decision = vote(3) on CodeReviewer.assess(pr)
// 3 reviewers independently review. If 2/3 approve → approved.

// Refine: iterative improvement loop
let polished = refine(article) with Editor.review
// Loops: draft → review → apply_feedback → re-review until quality threshold
// max_iterations defaults to 5
```

---

### B) Advanced Optimizer & IR (25% of effort)

This section was deferred from v0.3 and v0.2. Now that Lucky has users, optimization matters.

| # | Feature | Effort | Description |
|---|---|---|---|
| **B1** | GVN pass | M | Global Value Numbering. Detects redundant computations across basic blocks. Eliminates identical expressions with different SSA registers. |
| **B2** | LICM pass | M | Loop Invariant Code Motion. Hoists loop-invariant computations out of loops. Effective for workflows with repeated LLM calls. |
| **B3** | Inlining pass | M | Inline small task/function calls into their callers. Heuristic: inline if the callee has < 10 nodes or is a simple pass-through. |
| **B4** | Low-level IR (LIR) | L | Third IR level below MIR. Linear instruction sequence, explicit virtual registers, basic block layout. Bridges the gap between MIR and execution. Enables future native codegen. |
| **B5** | Binary IR serialization | M | `.lkr` binary format via FlatBuffers. Zero-copy deserialization, 60-70% smaller than JSON. `lucky compile --format binary`. |
| **B6** | AI-specific optimization | L | LLM call fusion (merge adjacent calls to same model), prompt caching hints, speculative execution (cheap checks before expensive LLM calls). |
| **B7** | Critical path analysis | S | Compute the critical path through the execution DAG. Report bottleneck nodes at compile time. `lucky compile --critical-path`. |

#### B1 — GVN (Global Value Numbering)

```
Before GVN:
  %1 = add %a, %b
  ...
  %2 = add %a, %b    // same computation, different register

After GVN:
  %1 = add %a, %b
  ...
  %2 = %1            // replaced with %1

Implementation: hash-based value numbering across all blocks.
Value table maps (opcode, operands) -> value number.
```

#### B6 — AI-Specific Optimization

```
// Before fusion:
let summary = ask DeepSeek: summarize {{document}}
let keywords = ask DeepSeek: extract keywords from {{document}}

// After fusion (single LLM call with multi-part prompt):
let result = ask DeepSeek: 
  Part 1: summarize {{document}}
  Part 2: extract keywords from the same document
  Output format: { summary: string, keywords: string[] }

let summary = result.summary
let keywords = result.keywords

// Speculative execution:
// "Run a cheap grep before an expensive LLM audit"
check test_output.contains("panic")  // cheap
if true
    run LLMAudit                      // only if checked
```

---

### C) Ecosystem & Platform (25% of effort)

Move Lucky from a developer tool to a platform with cloud services and enterprise deployment options.

| # | Feature | Effort | Description |
|---|---|---|---|
| **C1** | Kubernetes operator | L | Custom Kubernetes controller. `lucky` CRD (Custom Resource Definition). Manages workflow runs as K8s Jobs. Native scaling, secrets, networking. |
| **C2** | Package registry server | L | Central OCI-compatible registry. `lucky pkg publish`, `lucky pkg search`, `lucky pkg install` with dependency resolution. Ed25519 signing. |
| **C3** | Lucky Cloud service | L | Managed Lucky runtime as a service. REST API: `POST /run (ir.json)`, `GET /events/{run_id}`, `POST /approve/{gate_id}`. Pay-per-run pricing. |
| **C4** | Confidence expressions | M | `expr confidence > threshold` lowering to HIR/MIR → runtime Probabilistic value branching. |
| **C5** | Stream types | L | `Stream<T>` type. `Stream::from_iter`, `Stream::from_channel`. `map`, `filter`, `take`, `batch`, `merge` operations. Channel-based runtime. |
| **C6** | Knowledge declarations | S | `knowledge` declaration for RAG. `knowledge ProjectDocs from "./docs"`. Vector store integration at runtime. |
| **C7** | More platform adapters | M | Cline, Continue.dev, JetBrains AI, GitHub Copilot Extensions. Each with CI pipeline. |

#### C1 — Kubernetes Operator Architecture

```yaml
# lucky.yaml — CRD instance
apiVersion: lucky-lang.org/v1
kind: WorkflowRun
metadata:
  name: pr-review-123
spec:
  ir: "s3://lucky-runs/pr-123.lir"
  context:
    pr_number: 123
    repo: "jfxia/lucky"
  resources:
    limits:
      memory: "1Gi"
      cpu: "2"
  policies:
    timeout: "30m"
    max_retries: 3
  approvals:
    - gate: "deploy"
      slack_channel: "#deploy-approvals"
status:
  state: "Running"
  events_url: "/api/v1/workflowruns/pr-review-123/events"
```

**Components:**
1. **CRD controller** — watches `WorkflowRun` resources, creates K8s Jobs
2. **Job template** — each workflow run = one pod running `lucky run --in-cluster`
3. **Lucky runtime sidecar** — handles approvals via K8s events
4. **ConfigMap / Secret injection** — lucky.toml + API keys from K8s secrets

#### C2 — Package Registry

```bash
# Publish a package
lucky pkg publish ./my-workflow \
  --name "ci-review" \
  --version "1.2.0" \
  --sign

# Search
lucky pkg search "code review"

# Install
lucky pkg install ci-review@1.2.0
```

**Architecture:**
- Storage: OCI-compatible (GHCR, Docker Hub, or self-hosted)
- Signing: Ed25519 keys, public keys published to Keybase or well-known URL
- Resolution: Semver with lockfile (`lucky.lock`)
- The registry server is a standalone binary (`lucky-registry`) deployable via Docker

---

### D) Advanced Security (20% of effort)

Enterprise security — for organizations that deploy Lucky in production at scale.

| # | Feature | Effort | Description |
|---|---|---|---|
| **D1** | Firecracker sandbox | L | VM-level isolation for tool execution. Stronger than Docker — each tool runs in its own micro-VM. Linux-only. Uses Firecracker VMM. |
| **D2** | Full mTLS everywhere | M | Mutual TLS for all communication channels: LTP client↔server, SDK↔runtime, coordinator↔worker. Certificate management CLI (`lucky cert create/renew/revoke`). |
| **D3** | Audit SIEM integration | M | Structured audit events shipped to SIEM platforms: Splunk HEC, Elasticsearch, Datadog, OTLP-compatible receivers. Structured JSONL with CEF mapping. |

#### D1 — Firecracker Sandbox

```rust
pub struct FirecrackerSandbox {
    vm_id: String,
    jailer_root: PathBuf,
    kernel_path: PathBuf,
    rootfs_path: PathBuf,
    vsock_path: PathBuf,
}
```

**Compared to Docker sandbox (v0.3):**

| Aspect | Docker (v0.3) | Firecracker (v0.4) |
|---|---|---|
| Isolation level | Container | Micro-VM |
| Kernel | Shared with host | Isolated (guest kernel) |
| Boot time | Instant | ~125ms |
| Resource overhead | Low | Very low |
| Linux only | No | Yes |
| Use case | Dev / CI | Production / multi-tenant |

---

## 4. Timeline & Milestones

Total estimated effort: **24 weeks** (6 months) for 1-2 engineers.

| Milestone | Weeks | Content | Dependencies |
|---|---|---|---|
| **M1 — Advanced Orchestration** | 1-6 | A1 (multi-level delegation), A2 (contract enforcement), A3 (auto-rethink), A4 (parallel patterns) | v0.3 sub-agent system |
| **M2 — Optimizer & IR** | 7-12 | B1 (GVN), B2 (LICM), B3 (inlining), B4 (LIR), B5 (binary IR), B6 (AI opt), B7 (critical path) | v0.2 MIR |
| **M3 — Cloud Ecosystem** | 13-16 | C1 (K8s operator), C2 (package registry), C3 (Lucky Cloud) | v0.3 runtime |
| **M4 — Language + Security** | 17-20 | C4 (confidence), C5 (streams), C6 (knowledge), D1 (Firecracker), D2 (mTLS), D3 (SIEM) | M1 |
| **M5 — Polish & Release** | 21-24 | C7 (new adapters), integration testing, docs, changelog, beta program | M2, M3, M4 |

### Dependency Graph

```
M1 ──────────────────────┐
                          │
M2 ───────────────────┐   │
                      │   │
M3 ─────────────────┐ │   │
                    │ │   │
M4 ───────────────┐ │ │   │
                  ▼ ▼ ▼   ▼
                  M5 ────► Release
```

M2 (optimizer) and M3 (cloud) are independent and can run in parallel. M4 requires M1 (advanced orchestration concepts build on sub-agents).

---

## 5. Design Decisions

### D1: FlatBuffers over Protobuf for Binary IR

**Decision:** Use FlatBuffers (zero-copy deserialization) for `.lkr` binary IR.

**Rationale:**
- Zero-copy deserialization is critical for IR graphs with 1000+ nodes (multi-level delegation workflows)
- Schema evolution via explicit ID fields
- Typically 60-70% smaller than JSON IR
- Rust codegen is mature and well-tested
- Protobuf would require a deserialization step that adds latency

### D2: Ed25519 over ECDSA for Package Signing

**Decision:** Ed25519 for package signing.

**Rationale:**
- Smaller signatures (64 bytes vs ~70 bytes for ECDSA)
- Faster verification (~3x faster than ECDSA P-256)
- Constant-time implementation prevents timing attacks
- `ed25519-dalek` Rust crate is well-audited
- Wider ecosystem support (Keybase, Sigstore, etc.)

### D3: K8s CRD over Helm Chart Only

**Decision:** Build a K8s operator with CRD, not just a Helm chart.

**Rationale:**
- A Helm chart can install Lucky, but it can't manage individual workflow runs
- CRD enables: `kubectl get workflowruns`, declarative workflow specs, GitOps integration (ArgoCD)
- The operator pattern is the standard for production-grade K8s applications
- Helm chart is still provided for installation; the CRD+operator is the advanced option

### D4: Native Rust Firecracker SDK over Wrapper Scripts

**Decision:** Use the `rust-vmm` Firecracker SDK directly, not shell wrappers or Python `firecracker-py`.

**Rationale:**
- Firecracker exposes a REST API over a Unix socket — `rust-vmm` provides native Rust bindings
- Shell wrappers are fragile and slow
- Python dependency would add 50+ MB to the runtime binary
- `rust-vmm` is maintained by AWS (the creators of Firecracker)

### D5: Lucky Cloud REST over gRPC

**Decision:** REST/JSON for Lucky Cloud API, not gRPC.

**Rationale:**
- REST is universally accessible — any platform can call `POST /run` with curl
- gRPC adds ceremony (protobuf compilation, HTTP/2 requirement)
- The API surface is small (~10 endpoints) — REST is simpler
- Streaming events use Server-Sent Events (SSE), which is REST-compatible
- If gRPC is needed for performance, add it as a secondary transport

---

## 6. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| **Multi-level delegation creates runaway costs** | Medium | High | Enforce `max_depth` (default: 5) and `max_total_delegations` (default: 50). Budget enforcement at each level. Real-time cost display. |
| **Contract enforcement is too strict** | Medium | Medium | Contracts are warnings by default, errors with `--strict-contracts` flag. Users can opt in to enforcement. |
| **Firecracker requires complex setup** | High | Medium | Firecracker needs kernel image + rootfs + jailer setup. Provide pre-built images and a `lucky sandbox setup` command. Linux-only — clearly documented. |
| **K8s operator scope creep** | Medium | Medium | Ship CRD + core controller in v0.4. Defer Webhook validation, custom scheduler, and horizontal auto-scaling to v0.5. |
| **Package registry adoption** | Medium | Low | Registry is optional. Users can still use filesystem packages. Registry just adds convenience. |
| **FlatBuffers schema evolution breaks old .lir files** | Low | High | Schema versioning via explicit `version` field. Old-format files transparently upgraded on load. |
| **AI-specific optimizer is hard to validate** | High | Low | Ship in v0.4.1 as opt-in (`--opt experimental`). Let real users validate before default-enabling. |

---

## 7. Success Criteria

### Must-Have (v0.4.0 Release)

- [ ] Multi-level delegation works: depth-3 delegation tree executes correctly. `max_depth` enforced.
- [ ] Contract enforcement: mismatched I/O shapes produce clear errors. Low-confidence outputs trigger re-delegation.
- [ ] Auto-rethink: orchestrator re-plans on configurable failure types. `max_rethink` enforced.
- [ ] Built-in patterns: `split`, `aggregate`, `vote`, `refine` all produce correct results.
- [ ] GVN, LICM, and inlining passes pass correctness test suites. Compile-time improvements measurable.
- [ ] LIR correctly lowers from MIR. Linear instruction sequence executes identically to MIR interpretation.
- [ ] Binary IR: `.lkr` files are 60% smaller than JSON and load 2x faster.
- [ ] Critical path analysis: `lucky compile --critical-path` reports bottleneck nodes.
- [ ] AI-specific optimizer: LLM call fusion works for adjacent calls to the same model.
- [ ] K8s operator: `kubectl apply -f lucky-workflow.yaml` creates a running workflow. Status updates correctly.
- [ ] Package registry: `publish`, `search`, `install` with dependency resolution work end-to-end.
- [ ] Lucky Cloud: `POST /run`, `GET /events`, `POST /approve` work. SSE streaming for live events.
- [ ] Confidence expressions: `result.confidence > 0.9` branches correctly at runtime.
- [ ] Stream types: `Stream` with `map`/`filter`/`take` works. Channel-based runtime.
- [ ] Knowledge declarations: `knowledge X from "./docs"` loads and indexes documents for RAG.
- [ ] Firecracker sandbox: tool execution in micro-VM. Boot time < 200ms. Isolation verified.
- [ ] Full mTLS: LTP, SDK, and inter-worker communication all support mTLS. Certificate CLI works.
- [ ] SIEM integration: audit events exportable to Splunk HEC and Elasticsearch.
- [ ] New platform adapters: at least 2 of Cline, Continue, JetBrains AI, GitHub Copilot.
- [ ] v0.3 programs compile without changes (backward compatibility).
- [ ] All spec documents updated to v0.4.

### Nice-to-Have (v0.4.1+)

- [ ] AI optimizer speculative execution: cheap precondition checks before expensive LLM calls.
- [ ] K8s Webhook validation for WorkflowRun CRD.
- [ ] Package registry: automated CI/CD publishing from GitHub Actions.
- [ ] Lucky Cloud: per-run billing dashboard.
- [ ] mTLS auto-renewal via cert-manager integration.
- [ ] Firecracker Windows support (requires WSL2 integration).

### Metrics to Track

| Metric | Target |
|---|---|
| Multi-level delegation overhead | < 10% vs. flat delegation for depth <= 3 |
| GVN + LICM + inlining combined speedup | ≥ 15% reduction in IR node count |
| Binary IR load time vs. JSON | ≤ 50% of JSON load time |
| Package registry publish time | < 5 seconds (including signing) |
| K8s operator: time from `kubectl apply` to running pod | < 10 seconds |
| Lucky Cloud API p99 latency | < 200ms |
| Firecracker sandbox boot time | < 200ms (p99) |
| Firecracker vs. Docker overhead per call | < 15% latency increase |
| mTLS handshake overhead | < 50ms added per connection |
| v0.3 workflow compatibility | 100% pass on v0.4 |

---

*Last updated: July 2026 — v0.4 Design Plan*
