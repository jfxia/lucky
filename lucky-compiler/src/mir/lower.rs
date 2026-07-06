use super::*;
use crate::ast::{self, BinOp, Expr, Stmt, UnaryOp};
use crate::ast::stmt::Block;
use crate::hir;

pub struct MirLowering {
    reg_counter: RegId,
    block_counter: BlockId,
    const_counter: ConstId,
    functions: Vec<Function>,
    constants: Vec<Operand>,
    current_block: Option<BlockId>,
    current_func_id: usize,
}

impl MirLowering {
    pub fn new() -> Self {
        Self {
            reg_counter: 0,
            block_counter: 0,
            const_counter: 0,
            functions: Vec::new(),
            constants: Vec::new(),
            current_block: None,
            current_func_id: 0,
        }
    }

    pub fn lower_graph(mut self, hir_graph: &hir::HirGraph) -> Vec<Function> {
        // Walk through HIR nodes and lower them to MIR functions
        for node in &hir_graph.nodes {
            match node {
                hir::HirNode::Task { task_ref, inputs, outputs, steps, .. } => {
                    self.lower_task_from_hir(task_ref, inputs, outputs, steps, hir_graph);
                }
                hir::HirNode::Workflow { workflow_ref, context, body, .. } => {
                    self.lower_workflow_from_hir(workflow_ref, context, body, hir_graph);
                }
                hir::HirNode::AgentInvoke { agent_ref, task_ref, .. } => {
                    let name = format!("{}.{}", agent_ref, task_ref);
                    self.lower_invoke_stub(&name);
                }
                _ => {}
            }
        }
        self.functions
    }

    pub fn lower_task_from_hir(
        &mut self,
        name: &str,
        inputs: &[(String, String)],
        outputs: &[(String, String)],
        steps: &[hir::NodeId],
        hir_graph: &hir::HirGraph,
    ) {
        let func_id = self.current_func_id;
        self.current_func_id += 1;

        let entry_id = self.alloc_block_id();
        let params: Vec<(String, IrType)> = inputs.iter()
            .map(|(n, t)| (n.clone(), IrType::StringType))
            .collect();
        let return_type = if outputs.is_empty() { IrType::Void } else { IrType::StringType };
        let mut function = Function::new(
            func_id, name.to_string(), params.clone(), return_type.clone(), entry_id,
        );

        let entry_block_id = entry_id;
        let mut entry_block = BasicBlock::new(entry_block_id);
        self.current_block = Some(entry_block_id);

        // Allocate stack space for inputs
        for (inp_name, _) in inputs {
            let alloca_reg = self.alloc_reg_id();
            entry_block.push_instruction(Instruction::new(
                Opcode::Alloca, vec![Operand::Symbol(inp_name.clone())],
                IrType::StringType, alloca_reg,
            ));
        }

        // Walk step nodes from HIR and lower them
        let mut prev_block_id = entry_block_id;
        let mut current_block = entry_block;
        for &step_id in steps {
            if let Some(hir_node) = hir_graph.nodes.get(step_id) {
                self.lower_single_hir_node(hir_node, &mut current_block, hir_graph);
            }
        }

        // Add return if not already terminated
        if !current_block.is_terminated() {
            let ret_reg = self.alloc_reg_id();
            current_block.push_instruction(Instruction::new(
                Opcode::RetOp, vec![], IrType::Void, ret_reg,
            ));
            current_block.set_terminator(TerminatorInst::Ret { value: None });
        }

        function.blocks.push(current_block);
        self.functions.push(function);
    }

    fn lower_single_hir_node(
        &mut self,
        node: &hir::HirNode,
        block: &mut BasicBlock,
        hir_graph: &hir::HirGraph,
    ) {
        match node {
            hir::HirNode::Let { name, value, .. } => {
                let reg = self.alloc_reg_id();
                block.push_instruction(Instruction::new(
                    Opcode::Alloca, vec![Operand::Symbol(name.clone())],
                    IrType::StringType, reg,
                ));
                let store_reg = self.alloc_reg_id();
                block.push_instruction(Instruction::new(
                    Opcode::Store, vec![Operand::Reg(reg), Operand::Symbol(value.clone())],
                    IrType::Void, store_reg,
                ));
            }
            hir::HirNode::Return { value, .. } => {
                let val = value.as_ref().map(|v| Operand::Symbol(v.clone()));
                let ret_reg = self.alloc_reg_id();
                block.push_instruction(Instruction::new(
                    Opcode::RetOp, vec![], IrType::Void, ret_reg,
                ));
                block.set_terminator(TerminatorInst::Ret { value: val });
            }
            hir::HirNode::LlmCall { model_ref, prompt_ref, .. } => {
                let reg = self.alloc_reg_id();
                let prompt = prompt_ref.as_deref().unwrap_or("?");
                block.push_instruction(Instruction::new(
                    Opcode::LlmComplete,
                    vec![Operand::Symbol(model_ref.clone()), Operand::Symbol(prompt.to_string())],
                    IrType::StringType, reg,
                ));
            }
            hir::HirNode::ToolCall { tool_ref, method, .. } => {
                let reg = self.alloc_reg_id();
                let m = method.as_deref().unwrap_or("invoke");
                block.push_instruction(Instruction::new(
                    Opcode::ToolInvoke,
                    vec![Operand::Symbol(tool_ref.clone()), Operand::Symbol(m.to_string())],
                    IrType::Void, reg,
                ));
            }
            hir::HirNode::AgentInvoke { agent_ref, task_ref, .. } => {
                let reg = self.alloc_reg_id();
                block.push_instruction(Instruction::new(
                    Opcode::AgentInvoke,
                    vec![Operand::Symbol(agent_ref.clone()), Operand::Symbol(task_ref.clone())],
                    IrType::Void, reg,
                ));
            }
            _ => {
                let reg = self.alloc_reg_id();
                block.push_instruction(Instruction::new(
                    Opcode::Call,
                    vec![Operand::Symbol("noop".into())],
                    IrType::Void, reg,
                ));
            }
        }
    }

    fn lower_workflow_from_hir(
        &mut self,
        name: &str,
        _context: &[(String, String)],
        body: &[hir::NodeId],
        hir_graph: &hir::HirGraph,
    ) {
        let func_id = self.current_func_id;
        self.current_func_id += 1;

        let entry_id = self.alloc_block_id();
        let mut function = Function::new(
            func_id, name.to_string(), Vec::new(), IrType::Void, entry_id,
        );

        let mut block = BasicBlock::new(entry_id);
        let mut prev_id = entry_id;

        for &step_id in body {
            if let Some(hir_node) = hir_graph.nodes.get(step_id) {
                match hir_node {
                    hir::HirNode::AgentInvoke { agent_ref, task_ref, .. } => {
                        let call_reg = self.alloc_reg_id();
                        block.push_instruction(Instruction::new(
                            Opcode::AgentInvoke,
                            vec![Operand::Symbol(agent_ref.clone()), Operand::Symbol(task_ref.clone())],
                            IrType::Void, call_reg,
                        ));
                    }
                    hir::HirNode::ToolCall { tool_ref, method, .. } => {
                        let m = method.as_deref().unwrap_or("invoke");
                        let call_reg = self.alloc_reg_id();
                        block.push_instruction(Instruction::new(
                            Opcode::ToolInvoke,
                            vec![Operand::Symbol(tool_ref.clone()), Operand::Symbol(m.to_string())],
                            IrType::Void, call_reg,
                        ));
                    }
                    hir::HirNode::LlmCall { model_ref, prompt_ref, .. } => {
                        let prompt = prompt_ref.as_deref().unwrap_or("?");
                        let call_reg = self.alloc_reg_id();
                        block.push_instruction(Instruction::new(
                            Opcode::LlmComplete,
                            vec![Operand::Symbol(model_ref.clone()), Operand::Symbol(prompt.to_string())],
                            IrType::StringType, call_reg,
                        ));
                    }
                    _ => {}
                }
            }
        }

        let ret_reg = self.alloc_reg_id();
        block.push_instruction(Instruction::new(
            Opcode::RetOp, vec![], IrType::Void, ret_reg,
        ));
        block.set_terminator(TerminatorInst::Ret { value: None });

        function.blocks.push(block);
        self.functions.push(function);
    }

    fn lower_invoke_stub(&mut self, name: &str) {
        let func_id = self.current_func_id;
        self.current_func_id += 1;
        let entry_id = self.alloc_block_id();
        let mut function = Function::new(
            func_id, name.to_string(), Vec::new(), IrType::Void, entry_id,
        );
        let mut block = BasicBlock::new(entry_id);
        let reg = self.alloc_reg_id();
        block.push_instruction(Instruction::new(
            Opcode::AgentInvoke,
            vec![Operand::Symbol(name.to_string())],
            IrType::Void, reg,
        ));
        let ret_reg = self.alloc_reg_id();
        block.push_instruction(Instruction::new(
            Opcode::RetOp, vec![], IrType::Void, ret_reg,
        ));
        block.set_terminator(TerminatorInst::Ret { value: None });
        function.blocks.push(block);
        self.functions.push(function);
    }

    fn lower_statement(&mut self, stmt: &Stmt, block: &mut BasicBlock) {
        match stmt {
            Stmt::Let { name, typ, value, .. } => {
                let value_op = self.lower_expression(value, block);
                let ty = typ
                    .as_ref()
                    .map(|t| IrType::from_ast_type(t))
                    .unwrap_or(IrType::Void);

                let alloca_reg = self.alloc_reg_id();
                let alloca = Instruction::new(
                    Opcode::Alloca,
                    vec![Operand::Symbol(name.clone())],
                    ty.clone(),
                    alloca_reg,
                );
                block.push_instruction(alloca);

                let store_reg = self.alloc_reg_id();
                let store = Instruction::new(
                    Opcode::Store,
                    vec![Operand::Reg(alloca_reg), value_op],
                    IrType::Void,
                    store_reg,
                );
                block.push_instruction(store);
            }

            Stmt::Const { name, typ, value, .. } => {
                let value_op = self.lower_expression(value, block);
                let ty = typ
                    .as_ref()
                    .map(|t| IrType::from_ast_type(t))
                    .unwrap_or(IrType::Void);

                let alloca_reg = self.alloc_reg_id();
                let alloca = Instruction::new(
                    Opcode::Alloca,
                    vec![Operand::Symbol(name.clone())],
                    ty,
                    alloca_reg,
                );
                block.push_instruction(alloca);

                let store_reg = self.alloc_reg_id();
                let store = Instruction::new(
                    Opcode::Store,
                    vec![Operand::Reg(alloca_reg), value_op],
                    IrType::Void,
                    store_reg,
                );
                block.push_instruction(store);
            }

            Stmt::Assign { target, value, .. } => {
                let value_op = self.lower_expression(value, block);
                let target_op = self.lower_expression(target, block);

                let store_reg = self.alloc_reg_id();
                let store = Instruction::new(
                    Opcode::Store,
                    vec![target_op, value_op],
                    IrType::Void,
                    store_reg,
                );
                block.push_instruction(store);
            }

            Stmt::ExprStmt { expr, .. } => {
                self.lower_expression(expr, block);
            }

            Stmt::If { branches, else_body, .. } => {
                self.lower_if_stmt(branches, else_body.as_ref(), block);
            }

            Stmt::Return { value, .. } => {
                let val = value
                    .as_ref()
                    .map(|v| self.lower_expression(v, block));
                let ret_reg = self.alloc_reg_id();
                let ret = Instruction::new(
                    Opcode::RetOp,
                    val.into_iter().collect(),
                    IrType::Void,
                    ret_reg,
                );
                block.push_instruction(ret);
            }

            Stmt::Loop { body, .. } => {
                self.lower_loop_stmt(body, block);
            }

            Stmt::For { pattern, iterable, body, .. } => {
                self.lower_for_stmt(pattern, iterable, body, block);
            }

            _ => {
                let reg = self.alloc_reg_id();
                let stub = Instruction::new(
                    Opcode::Call,
                    vec![Operand::Symbol("stub_stmt".into())],
                    IrType::Void,
                    reg,
                );
                block.push_instruction(stub);
            }
        }
    }

    pub fn lower_expression(&mut self, expr: &Expr, block: &mut BasicBlock) -> Operand {
        match expr {
            Expr::Lit { value, .. } => self.lower_literal(value, block),

            Expr::Var { name, .. } => {
                let reg = self.alloc_reg_id();
                let load = Instruction::new(
                    Opcode::Load,
                    vec![Operand::Symbol(name.to_string())],
                    IrType::StringType,
                    reg,
                );
                block.push_instruction(load);
                Operand::Reg(reg)
            }

            Expr::BinaryOp { op, lhs, rhs, .. } => {
                let lhs_op = self.lower_expression(lhs, block);
                let rhs_op = self.lower_expression(rhs, block);
                let opcode = Opcode::from_bin_op(op);
                let result_type = self.infer_binary_result_type(op);

                let reg = self.alloc_reg_id();
                let inst = Instruction::new(
                    opcode,
                    vec![lhs_op, rhs_op],
                    result_type,
                    reg,
                );
                block.push_instruction(inst);
                Operand::Reg(reg)
            }

            Expr::UnaryOp { op, expr, .. } => {
                let operand = self.lower_expression(expr, block);
                let opcode = Opcode::from_unary_op(op);
                let result_type = self.infer_unary_result_type(op);

                let reg = self.alloc_reg_id();
                let inst = Instruction::new(
                    opcode,
                    vec![operand],
                    result_type,
                    reg,
                );
                block.push_instruction(inst);
                Operand::Reg(reg)
            }

            Expr::Call { callee, args, .. } => {
                let callee_op = self.lower_expression(callee, block);
                let mut ops = vec![callee_op];
                for arg in args {
                    let arg_op = self.lower_expression(&arg.value, block);
                    ops.push(arg_op);
                }

                let reg = self.alloc_reg_id();
                let inst = Instruction::new(
                    Opcode::Call,
                    ops,
                    IrType::Void,
                    reg,
                );
                block.push_instruction(inst);
                Operand::Reg(reg)
            }

            Expr::List { elements, .. } => {
                let mut ops = Vec::new();
                for elem in elements {
                    let elem_op = self.lower_expression(elem, block);
                    ops.push(elem_op);
                }

                let reg = self.alloc_reg_id();
                let inst = Instruction::new(
                    Opcode::ListNew,
                    ops,
                    IrType::List(Box::new(IrType::StringType)),
                    reg,
                );
                block.push_instruction(inst);
                Operand::Reg(reg)
            }

            Expr::Map { entries, .. } => {
                let mut ops = Vec::new();
                for (key, value) in entries {
                    let key_op = self.lower_expression(key, block);
                    let value_op = self.lower_expression(value, block);
                    ops.push(key_op);
                    ops.push(value_op);
                }

                let reg = self.alloc_reg_id();
                let inst = Instruction::new(
                    Opcode::MapNew,
                    ops,
                    IrType::Map(Box::new(IrType::StringType), Box::new(IrType::StringType)),
                    reg,
                );
                block.push_instruction(inst);
                Operand::Reg(reg)
            }

            Expr::Index { base, index, .. } => {
                let base_op = self.lower_expression(base, block);
                let index_op = self.lower_expression(index, block);

                let reg = self.alloc_reg_id();
                let inst = Instruction::new(
                    Opcode::ListGet,
                    vec![base_op, index_op],
                    IrType::StringType,
                    reg,
                );
                block.push_instruction(inst);
                Operand::Reg(reg)
            }

            Expr::IfExpr { cond, then, else_, .. } => {
                self.lower_if_expr(cond, then, else_, block)
            }

            Expr::InterpolatedString { parts, .. } => {
                let reg = self.alloc_reg_id();
                let mut ops = Vec::new();
                for part in parts {
                    match part {
                        ast::InterpolatedPart::Text(s) => {
                            ops.push(Operand::Symbol(s.clone()));
                        }
                        ast::InterpolatedPart::Expr(e) => {
                            let e_op = self.lower_expression(e, block);
                            ops.push(e_op);
                        }
                    }
                }
                let inst = Instruction::new(
                    Opcode::StrConcat,
                    ops,
                    IrType::StringType,
                    reg,
                );
                block.push_instruction(inst);
                Operand::Reg(reg)
            }

            _ => {
                let reg = self.alloc_reg_id();
                let stub = Instruction::new(
                    Opcode::Call,
                    vec![Operand::Symbol("stub_expr".into())],
                    IrType::Void,
                    reg,
                );
                block.push_instruction(stub);
                Operand::Reg(reg)
            }
        }
    }

    fn lower_literal(&mut self, value: &ast::Literal, block: &mut BasicBlock) -> Operand {
        let _const_id = self.alloc_const_id();
        let operand = match value {
            ast::Literal::Int(i) => Operand::Immediate(*i),
            ast::Literal::Float(f) => {
                let reg = self.alloc_reg_id();
                let inst = Instruction::new(
                    Opcode::Cast,
                    vec![Operand::Immediate(*f as i64)],
                    IrType::F64,
                    reg,
                );
                block.push_instruction(inst);
                return Operand::Reg(reg);
            }
            ast::Literal::String(s) => Operand::Symbol(s.clone()),
            ast::Literal::Bool(b) => Operand::Immediate(if *b { 1 } else { 0 }),
            _ => Operand::Immediate(0),
        };
        self.constants.push(operand.clone());
        operand
    }

    fn lower_if_stmt(
        &mut self,
        branches: &[ast::IfBranch],
        else_body: Option<&Block>,
        current_block: &mut BasicBlock,
    ) {
        let after_block_id = self.alloc_block_id();
        let mut branch_blocks: Vec<(BlockId, BlockId)> = Vec::new();

        for branch in branches {
            let then_block_id = self.alloc_block_id();
            let next_check_block_id = self.alloc_block_id();

            let cond_op = self.lower_expression(&branch.condition, current_block);

            current_block.set_terminator(TerminatorInst::CondBr {
                condition: cond_op,
                true_block: then_block_id,
                true_args: Vec::new(),
                false_block: next_check_block_id,
                false_args: Vec::new(),
            });

            let mut then_block = BasicBlock::new(then_block_id);
            then_block.predecessors.push(current_block.id);

            for stmt in &branch.body.stmts {
                self.lower_statement(stmt, &mut then_block);
            }

            if then_block.instructions.iter().all(|i| i.opcode != Opcode::RetOp) {
                then_block.set_terminator(TerminatorInst::Br {
                    successor: after_block_id,
                    args: Vec::new(),
                });
            }

            let func = self.functions.last_mut().unwrap();
            let cur_id = current_block.id;
            let then_id = then_block_id;
            func.add_block(then_block);

            branch_blocks.push((then_id, next_check_block_id));

            *current_block = BasicBlock::new(next_check_block_id);
            current_block.predecessors.push(cur_id);
        }

        let mut after_block;

        if let Some(else_body) = else_body {
            let else_block_id = self.alloc_block_id();
            let mut else_block = BasicBlock::new(else_block_id);
            else_block.predecessors.push(current_block.id);

            for stmt in &else_body.stmts {
                self.lower_statement(stmt, &mut else_block);
            }

            if else_block.instructions.iter().all(|i| i.opcode != Opcode::RetOp) {
                else_block.set_terminator(TerminatorInst::Br {
                    successor: after_block_id,
                    args: Vec::new(),
                });
            }

            let func = self.functions.last_mut().unwrap();
            func.add_block(else_block);

            current_block.set_terminator(TerminatorInst::Br {
                successor: else_block_id,
                args: Vec::new(),
            });

            after_block = BasicBlock::new(after_block_id);
            after_block.predecessors.push(else_block_id);
        } else {
            current_block.set_terminator(TerminatorInst::Br {
                successor: after_block_id,
                args: Vec::new(),
            });

            after_block = BasicBlock::new(after_block_id);
            after_block.predecessors.push(current_block.id);
        }

        for (then_id, _) in &branch_blocks {
            after_block.predecessors.push(*then_id);
        }

        let func = self.functions.last_mut().unwrap();
        func.add_block(std::mem::replace(current_block, BasicBlock::new(current_block.id)));
        func.add_block(after_block);
    }

    fn lower_if_expr(
        &mut self,
        cond: &Expr,
        then: &Expr,
        else_: &Expr,
        current_block: &mut BasicBlock,
    ) -> Operand {
        let then_block_id = self.alloc_block_id();
        let else_block_id = self.alloc_block_id();
        let merge_block_id = self.alloc_block_id();

        let cond_op = self.lower_expression(cond, current_block);

        current_block.set_terminator(TerminatorInst::CondBr {
            condition: cond_op,
            true_block: then_block_id,
            true_args: Vec::new(),
            false_block: else_block_id,
            false_args: Vec::new(),
        });

        let mut then_block = BasicBlock::new(then_block_id);
        then_block.predecessors.push(current_block.id);
        let then_result = self.lower_expression(then, &mut then_block);
        then_block.set_terminator(TerminatorInst::Br {
            successor: merge_block_id,
            args: vec![then_result.clone()],
        });

        let mut else_block = BasicBlock::new(else_block_id);
        else_block.predecessors.push(current_block.id);
        let else_result = self.lower_expression(else_, &mut else_block);
        else_block.set_terminator(TerminatorInst::Br {
            successor: merge_block_id,
            args: vec![else_result.clone()],
        });

        let merge_reg = self.alloc_reg_id();
        let mut merge_block = BasicBlock::new(merge_block_id);
        merge_block.predecessors.push(then_block_id);
        merge_block.predecessors.push(else_block_id);

        let phi = Instruction::new(
            Opcode::Phi,
            vec![
                Operand::Reg(merge_reg),
                Operand::Block(then_block_id),
                Operand::Block(else_block_id),
            ],
            IrType::StringType,
            merge_reg,
        );
        merge_block.push_instruction(phi);

        let func = self.functions.last_mut().unwrap();
        let old_block = std::mem::replace(current_block, BasicBlock::new(current_block.id));
        func.add_block(old_block);
        func.add_block(then_block);
        func.add_block(else_block);
        func.add_block(merge_block);

        Operand::Reg(merge_reg)
    }

    fn lower_loop_stmt(&mut self, body: &Block, current_block: &mut BasicBlock) {
        let header_block_id = self.alloc_block_id();
        let body_block_id = self.alloc_block_id();
        let exit_block_id = self.alloc_block_id();

        current_block.set_terminator(TerminatorInst::Br {
            successor: header_block_id,
            args: Vec::new(),
        });

        let mut header_block = BasicBlock::new(header_block_id);
        header_block.predecessors.push(current_block.id);
        header_block.predecessors.push(body_block_id);

        let header_reg = self.alloc_reg_id();
        let phi = Instruction::new(
            Opcode::Call,
            vec![Operand::Symbol("loop_continue".into())],
            IrType::I1,
            header_reg,
        );
        header_block.push_instruction(phi);

        header_block.set_terminator(TerminatorInst::CondBr {
            condition: Operand::Reg(header_reg),
            true_block: body_block_id,
            true_args: Vec::new(),
            false_block: exit_block_id,
            false_args: Vec::new(),
        });

        let mut body_block = BasicBlock::new(body_block_id);
        body_block.predecessors.push(header_block_id);
        for stmt in &body.stmts {
            self.lower_statement(stmt, &mut body_block);
        }
        body_block.set_terminator(TerminatorInst::Br {
            successor: header_block_id,
            args: Vec::new(),
        });

        let mut exit_block = BasicBlock::new(exit_block_id);
        exit_block.predecessors.push(header_block_id);

        let func = self.functions.last_mut().unwrap();
        func.add_block(header_block);
        func.add_block(body_block);
        *current_block = exit_block;
    }

    fn lower_for_stmt(
        &mut self,
        _pattern: &ast::Pattern,
        iterable: &Expr,
        body: &Block,
        current_block: &mut BasicBlock,
    ) {
        let header_block_id = self.alloc_block_id();
        let body_block_id = self.alloc_block_id();
        let exit_block_id = self.alloc_block_id();

        let iterable_op = self.lower_expression(iterable, current_block);
        let iter_reg = self.alloc_reg_id();
        let iter_new = Instruction::new(
            Opcode::Call,
            vec![Operand::Symbol("iter_new".into()), iterable_op],
            IrType::Void,
            iter_reg,
        );
        current_block.push_instruction(iter_new);

        current_block.set_terminator(TerminatorInst::Br {
            successor: header_block_id,
            args: Vec::new(),
        });

        let mut header_block = BasicBlock::new(header_block_id);
        header_block.predecessors.push(current_block.id);
        header_block.predecessors.push(body_block_id);

        let next_reg = self.alloc_reg_id();
        let iter_next = Instruction::new(
            Opcode::Call,
            vec![Operand::Symbol("iter_next".into()), Operand::Reg(iter_reg)],
            IrType::StringType,
            next_reg,
        );
        header_block.push_instruction(iter_next);

        let has_next_reg = self.alloc_reg_id();
        let has_next = Instruction::new(
            Opcode::Call,
            vec![Operand::Symbol("iter_has_next".into()), Operand::Reg(iter_reg)],
            IrType::I1,
            has_next_reg,
        );
        header_block.push_instruction(has_next);

        header_block.set_terminator(TerminatorInst::CondBr {
            condition: Operand::Reg(has_next_reg),
            true_block: body_block_id,
            true_args: vec![Operand::Reg(next_reg)],
            false_block: exit_block_id,
            false_args: Vec::new(),
        });

        let mut body_block = BasicBlock::new(body_block_id);
        body_block.predecessors.push(header_block_id);
        body_block.arguments.push(("for_elem".into(), IrType::StringType));

        for stmt in &body.stmts {
            self.lower_statement(stmt, &mut body_block);
        }
        body_block.set_terminator(TerminatorInst::Br {
            successor: header_block_id,
            args: Vec::new(),
        });

        let mut exit_block = BasicBlock::new(exit_block_id);
        exit_block.predecessors.push(header_block_id);

        let func = self.functions.last_mut().unwrap();
        func.add_block(header_block);
        func.add_block(body_block);
        *current_block = exit_block;
    }

    fn alloc_reg_id(&mut self) -> RegId {
        let id = self.reg_counter;
        self.reg_counter += 1;
        id
    }

    fn alloc_block_id(&mut self) -> BlockId {
        let id = self.block_counter;
        self.block_counter += 1;
        id
    }

    fn alloc_const_id(&mut self) -> ConstId {
        let id = self.const_counter;
        self.const_counter += 1;
        id
    }

    fn infer_binary_result_type(&self, op: &BinOp) -> IrType {
        match op {
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge
            | BinOp::And | BinOp::Or => IrType::I1,
            BinOp::Concat | BinOp::Repeat => IrType::StringType,
            _ => IrType::I64,
        }
    }

    fn infer_unary_result_type(&self, op: &UnaryOp) -> IrType {
        match op {
            UnaryOp::Not => IrType::I1,
            UnaryOp::Neg => IrType::I64,
        }
    }
}
