//! IR serialization: convert HIR graph and MIR functions to JSON format.

use crate::hir::{HirGraph, HirNode, HirEdgeKind};
use crate::mir::{Function, BasicBlock, TerminatorInst, Operand, IrType};

/// Serialize a HIR graph to a JSON string.
pub fn serialize_hir(graph: &HirGraph) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"version\": \"0.1\",\n");
    s.push_str("  \"meta\": {\n");
    s.push_str("    \"ir_level\": \"high\"\n");
    s.push_str("  },\n");

    // Nodes
    s.push_str("  \"graph\": {\n");
    s.push_str("    \"nodes\": [\n");
    for (i, node) in graph.nodes.iter().enumerate() {
        if i > 0 { s.push_str(",\n"); }
        serialize_hir_node(&mut s, node, i);
    }
    s.push_str("\n    ],\n");

    // Edges
    s.push_str("    \"edges\": [\n");
    for (i, edge) in graph.edges.iter().enumerate() {
        if i > 0 { s.push_str(",\n"); }
        s.push_str(&format!(
            "      {{ \"from\": {}, \"to\": {}, \"kind\": \"{:?}\" }}",
            edge.from, edge.to, edge.kind
        ));
    }
    s.push_str("\n    ]\n");
    s.push_str("  }\n");
    s.push_str("}\n");
    s
}

fn serialize_hir_node(s: &mut String, node: &HirNode, id: usize) {
    s.push_str("      {\n");
    s.push_str(&format!("        \"id\": {},\n", id));
    s.push_str(&format!("        \"kind\": \"{}\",\n", node_kind_str(node)));
    s.push_str(&format!("        \"label\": \"{}\"\n", node_label(node)));
    s.push_str("      }");
}

fn node_kind_str(node: &HirNode) -> &str {
    match node {
        HirNode::Goal { .. } => "goal",
        HirNode::Workflow { .. } => "workflow",
        HirNode::Task { .. } => "task",
        HirNode::AgentInvoke { .. } => "agent_invoke",
        HirNode::ToolCall { .. } => "tool",
        HirNode::LlmCall { .. } => "llm_call",
        HirNode::Decision { .. } => "decision",
        HirNode::Match { .. } => "match",
        HirNode::Parallel { .. } => "parallel",
        HirNode::Join { .. } => "join",
        HirNode::Loop { .. } => "loop",
        HirNode::ForEach { .. } => "for_each",
        HirNode::Pipeline { .. } => "pipeline",
        HirNode::Attempt { .. } => "attempt",
        HirNode::Approval { .. } => "approval",
        HirNode::Let { .. } => "let",
        HirNode::Return { .. } => "return",
        HirNode::Noop { .. } => "noop",
    }
}

fn node_label_str(node: &HirNode) -> String {
    match node {
        HirNode::Goal { goal_ref, .. } => format!("goal:{}", goal_ref),
        HirNode::Workflow { workflow_ref, .. } => format!("workflow:{}", workflow_ref),
        HirNode::Task { task_ref, .. } => format!("task:{}", task_ref),
        HirNode::AgentInvoke { agent_ref, task_ref, .. } => format!("{}.{}", agent_ref, task_ref),
        HirNode::ToolCall { tool_ref, method, .. } => format!("{}.{}", tool_ref, method.as_deref().unwrap_or("?")),
        HirNode::LlmCall { model_ref, .. } => format!("llm:{}", model_ref),
        HirNode::Decision { condition, .. } => condition.clone(),
        HirNode::Match { .. } => "match".into(),
        HirNode::Parallel { has_wait, .. } => format!("parallel(wait={})", has_wait),
        HirNode::Join { .. } => "join".into(),
        HirNode::Loop { .. } => "loop".into(),
        HirNode::ForEach { binding, .. } => format!("for {}", binding),
        HirNode::Pipeline { .. } => "pipeline".into(),
        HirNode::Attempt { .. } => "attempt".into(),
        HirNode::Approval { operation, .. } => format!("approval:{}", operation),
        HirNode::Let { name, .. } => format!("let {}", name),
        HirNode::Return { .. } => "return".into(),
        HirNode::Noop { .. } => "noop".into(),
    }
}

fn node_label(node: &HirNode) -> &str {
    // For simplicity in the JSON output, return a static string.
    // The detailed label is embedded in the kind.
    match node {
        HirNode::Goal { .. } => "Goal",
        HirNode::Workflow { .. } => "Workflow",
        HirNode::Task { .. } => "Task",
        HirNode::AgentInvoke { .. } => "AgentInvoke",
        HirNode::ToolCall { .. } => "ToolCall",
        HirNode::LlmCall { .. } => "LlmCall",
        HirNode::Decision { .. } => "Decision",
        HirNode::Match { .. } => "Match",
        HirNode::Parallel { .. } => "Parallel",
        HirNode::Join { .. } => "Join",
        HirNode::Loop { .. } => "Loop",
        HirNode::ForEach { .. } => "ForEach",
        HirNode::Pipeline { .. } => "Pipeline",
        HirNode::Attempt { .. } => "Attempt",
        HirNode::Approval { .. } => "Approval",
        HirNode::Let { .. } => "Let",
        HirNode::Return { .. } => "Return",
        HirNode::Noop { .. } => "Noop",
    }
}

/// Serialize MIR functions to a JSON string.
pub fn serialize_mir(functions: &[Function]) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"version\": \"0.1\",\n");
    s.push_str("  \"meta\": {\n");
    s.push_str("    \"ir_level\": \"mid\"\n");
    s.push_str("  },\n");
    s.push_str("  \"functions\": [\n");

    for (i, func) in functions.iter().enumerate() {
        if i > 0 { s.push_str(",\n"); }
        serialize_mir_function(&mut s, func);
    }

    s.push_str("\n  ]\n");
    s.push_str("}\n");
    s
}

fn serialize_mir_function(s: &mut String, func: &Function) {
    s.push_str("    {\n");
    s.push_str(&format!("      \"id\": {},\n", func.id));
    s.push_str(&format!("      \"name\": \"{}\",\n", func.name));
    s.push_str(&format!("      \"entry_block\": {},\n", func.entry_block));
    s.push_str("      \"blocks\": [\n");

    for (i, block) in func.blocks.iter().enumerate() {
        if i > 0 { s.push_str(",\n"); }
        serialize_basic_block(s, block);
    }

    s.push_str("\n      ]\n");
    s.push_str("    }");
}

fn serialize_basic_block(s: &mut String, block: &BasicBlock) {
    s.push_str("        {\n");
    s.push_str(&format!("          \"id\": {},\n", block.id));

    // Arguments
    s.push_str("          \"arguments\": [");
    for (i, (name, typ)) in block.arguments.iter().enumerate() {
        if i > 0 { s.push_str(", "); }
        s.push_str(&format!("{{\"name\": \"{}\", \"type\": \"{}\"}}", name, type_str(typ)));
    }
    s.push_str("],\n");

    // Instructions
    s.push_str("          \"instructions\": [\n");
    for (i, inst) in block.instructions.iter().enumerate() {
        if i > 0 { s.push_str(",\n"); }
        s.push_str("            {\n");
        s.push_str(&format!("              \"result\": \"%{}\",\n", inst.result_id));
        s.push_str(&format!("              \"opcode\": \"{:?}\",\n", inst.opcode));
        s.push_str("              \"operands\": [");
        for (j, op) in inst.operands.iter().enumerate() {
            if j > 0 { s.push_str(", "); }
            match op {
                Operand::Reg(r) => s.push_str(&format!("\"%{}\"", r)),
                Operand::Const(c) => s.push_str(&format!("\"#const{}\"", c)),
                Operand::Symbol(sym) => s.push_str(&format!("\"@{}\"", sym)),
                Operand::Block(b) => s.push_str(&format!("\"bb{}\"", b)),
                Operand::Immediate(v) => s.push_str(&format!("{}", v)),
            }
        }
        s.push_str("],\n");
        s.push_str(&format!("              \"type\": \"{}\"\n", type_str(&inst.result_type)));
        s.push_str("            }");
    }
    s.push_str("\n          ],\n");

    // Terminator
    s.push_str("          \"terminator\": ");
    match &block.terminator {
        TerminatorInst::Br { successor, args } => {
            s.push_str(&format!("{{\"kind\": \"br\", \"succ\": {}", successor));
            if !args.is_empty() {
                s.push_str(", \"args\": [");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 { s.push_str(", "); }
                    serialize_operand(s, a);
                }
                s.push_str("]");
            }
            s.push_str("}");
        }
        TerminatorInst::CondBr { condition, true_block, true_args, false_block, false_args: _ } => {
            s.push_str(&format!(
                "{{\"kind\": \"cond_br\", \"cond\": \"%{:?}\", \"true_succ\": {}, \"false_succ\": {}",
                condition, true_block, true_block
            ));
            s.push_str("}");
        }
        TerminatorInst::Ret { value } => {
            s.push_str("{\"kind\": \"ret\"");
            if let Some(v) = value {
                s.push_str(", \"value\": \"");
                serialize_operand(s, v);
                s.push_str("\"");
            }
            s.push_str("}");
        }
        TerminatorInst::Unreachable => {
            s.push_str("{\"kind\": \"unreachable\"}");
        }
    }

    s.push_str("\n        }");
}

fn serialize_operand(s: &mut String, op: &Operand) {
    match op {
        Operand::Reg(r) => s.push_str(&format!("%{}", r)),
        Operand::Const(c) => s.push_str(&format!("#{}", c)),
        Operand::Symbol(sym) => s.push_str(&format!("@{}", sym)),
        Operand::Block(b) => s.push_str(&format!("bb{}", b)),
        Operand::Immediate(v) => s.push_str(&format!("{}", v)),
    }
}

fn type_str(typ: &IrType) -> String {
    match typ {
        IrType::I1 => "i1".into(),
        IrType::I64 => "i64".into(),
        IrType::F64 => "f64".into(),
        IrType::StringType => "str".into(),
        IrType::BytesType => "bytes".into(),
        IrType::List(inner) => format!("list<{}>", type_str(inner)),
        IrType::Map(k, v) => format!("map<{},{}>", type_str(k), type_str(v)),
        IrType::Void => "void".into(),
        IrType::Agent(name) => format!("agent:{}", name),
        IrType::Task(name) => format!("task:{}", name),
    }
}
