use std::collections::{HashSet, VecDeque};
use crate::hir::{HirGraph, HirNode, HirEdgeKind};

pub fn verify_graph(graph: &HirGraph) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if graph.nodes.is_empty() {
        errors.push("Graph has no nodes".to_string());
        return Err(errors);
    }

    verify_acyclicity(graph, &mut errors);
    verify_reachability(graph, &mut errors);
    verify_edge_integrity(graph, &mut errors);
    verify_node_references(graph, &mut errors);

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn verify_acyclicity(graph: &HirGraph, errors: &mut Vec<String>) {
    let n = graph.nodes.len();
    let mut in_degree = vec![0usize; n];
    let mut adj = vec![Vec::new(); n];

    for edge in &graph.edges {
        if edge.from < n && edge.to < n {
            adj[edge.from].push(edge.to);
            in_degree[edge.to] += 1;
        }
    }

    let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut visited = 0usize;

    while let Some(u) = queue.pop_front() {
        visited += 1;
        for &v in &adj[u] {
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    if visited < n {
        let mut cycle_nodes = Vec::new();
        for i in 0..n {
            if in_degree[i] > 0 {
                cycle_nodes.push(node_label(&graph.nodes[i]));
            }
        }
        errors.push(format!(
            "Graph contains a cycle involving {} node(s): {}",
            n - visited,
            cycle_nodes.join(", ")
        ));
    }
}

fn verify_reachability(graph: &HirGraph, errors: &mut Vec<String>) {
    let n = graph.nodes.len();
    if n == 0 { return; }

    let mut visited = vec![false; n];
    let mut queue = VecDeque::new();

    for &entry in &graph.entry_points {
        if entry < n {
            visited[entry] = true;
            queue.push_back(entry);
        }
    }

    if queue.is_empty() {
        errors.push("Graph has no reachable entry points".to_string());
        return;
    }

    let mut adj = vec![Vec::new(); n];
    for edge in &graph.edges {
        if edge.from < n && edge.to < n {
            adj[edge.from].push(edge.to);
        }
    }

    while let Some(u) = queue.pop_front() {
        for &v in &adj[u] {
            if !visited[v] {
                visited[v] = true;
                queue.push_back(v);
            }
        }
    }

    let unreachable: Vec<_> = (0..n)
        .filter(|&i| !visited[i])
        .map(|i| format!("#{}: {}", i, node_label(&graph.nodes[i])))
        .collect();

    if !unreachable.is_empty() {
        errors.push(format!(
            "{} unreachable node(s) (not reachable from any entry point): {}",
            unreachable.len(),
            unreachable.join("; ")
        ));
    }
}

fn verify_edge_integrity(graph: &HirGraph, errors: &mut Vec<String>) {
    let n = graph.nodes.len();

    for edge in &graph.edges {
        if edge.from >= n {
            errors.push(format!(
                "Edge from invalid node {} (max {})", edge.from, n.saturating_sub(1)
            ));
        }
        if edge.to >= n {
            errors.push(format!(
                "Edge to invalid node {} (max {})", edge.to, n.saturating_sub(1)
            ));
        }
    }

    for (i, node) in graph.nodes.iter().enumerate() {
        match node {
            HirNode::Goal { subgoals, .. } => {
                for &sg in subgoals {
                    if sg >= n {
                        errors.push(format!(
                            "Goal node #{} references invalid subgoal {}", i, sg
                        ));
                    }
                }
            }
            HirNode::Decision { then_branch, else_branch, .. } => {
                if *then_branch >= n {
                    errors.push(format!("Decision #{} then_branch {} invalid", i, then_branch));
                }
                if let Some(eb) = else_branch {
                    if *eb >= n {
                        errors.push(format!("Decision #{} else_branch {} invalid", i, eb));
                    }
                }
            }
            HirNode::Workflow { body, .. } | HirNode::Loop { body, .. }
            | HirNode::ForEach { body, .. } | HirNode::Attempt { body, .. }
            | HirNode::Parallel { branches: body, .. } => {
                for &bid in body {
                    if bid >= n {
                        errors.push(format!(
                            "Node #{} references invalid body node {}", i, bid
                        ));
                    }
                }
            }
            HirNode::Task { steps, rollback, .. } => {
                for &sid in steps {
                    if sid >= n {
                        errors.push(format!("Task #{} references invalid step {}", i, sid));
                    }
                }
                for &rid in rollback {
                    if rid >= n {
                        errors.push(format!("Task #{} references invalid rollback {}", i, rid));
                    }
                }
            }
            HirNode::Pipeline { stages, .. } => {
                for &sid in stages {
                    if sid >= n {
                        errors.push(format!("Pipeline #{} invalid stage {}", i, sid));
                    }
                }
            }
            _ => {}
        }
    }
}

fn verify_node_references(graph: &HirGraph, errors: &mut Vec<String>) {
    let mut agent_names = HashSet::new();
    let mut task_names = HashSet::new();
    let mut tool_names = HashSet::new();

    for node in &graph.nodes {
        match node {
            HirNode::AgentInvoke { agent_ref, .. } => { agent_names.insert(agent_ref.clone()); }
            HirNode::Task { task_ref, .. } => { task_names.insert(task_ref.clone()); }
            HirNode::ToolCall { tool_ref, .. } => { tool_names.insert(tool_ref.clone()); }
            _ => {}
        }
    }

    for node in &graph.nodes {
        match node {
            HirNode::LlmCall { model_ref, .. } => {
                if model_ref == "?" || model_ref.is_empty() {
                    errors.push(format!("LlmCall has empty model reference"));
                }
            }
            HirNode::ToolCall { tool_ref, .. } => {
                if tool_ref.is_empty() {
                    errors.push("ToolCall has empty tool reference".to_string());
                }
            }
            HirNode::AgentInvoke { agent_ref, task_ref, .. } => {
                if agent_ref.is_empty() {
                    errors.push("AgentInvoke has empty agent reference".to_string());
                }
                if task_ref.is_empty() {
                    errors.push(format!(
                        "AgentInvoke '{}' has empty task reference", agent_ref
                    ));
                }
            }
            _ => {}
        }
    }
}

fn node_label(node: &HirNode) -> String {
    match node {
        HirNode::Goal { goal_ref, .. } => format!("Goal:{}", goal_ref),
        HirNode::Workflow { workflow_ref, .. } => format!("Workflow:{}", workflow_ref),
        HirNode::Task { task_ref, .. } => format!("Task:{}", task_ref),
        HirNode::AgentInvoke { agent_ref, task_ref, .. } => {
            format!("{}.{}", agent_ref, task_ref)
        }
        HirNode::ToolCall { tool_ref, method, .. } => {
            format!("{}.{}", tool_ref, method.as_deref().unwrap_or("?"))
        }
        HirNode::LlmCall { model_ref, .. } => format!("LLM:{}", model_ref),
        HirNode::Decision { condition, .. } => format!("if {}", condition),
        HirNode::Match { .. } => "match".into(),
        HirNode::Parallel { .. } => "parallel".into(),
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
