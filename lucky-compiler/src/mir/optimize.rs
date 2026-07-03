//! Optimization passes for the MIR intermediate representation.
//!
//! Passes operate on a `Vec<Function>` and return a modified `Vec<Function>`.

use crate::mir::*;

/// Run a pipeline of optimization passes on the MIR functions.
pub fn optimize(functions: &mut Vec<Function>, level: OptimizationLevel) {
    match level {
        OptimizationLevel::O0 => {} // no optimizations
        OptimizationLevel::O1 => {
            dead_code_elimination(functions);
            constant_folding(functions);
        }
        OptimizationLevel::O2 => {
            dead_code_elimination(functions);
            constant_folding(functions);
            local_cse(functions);
            copy_propagation(functions);
        }
        OptimizationLevel::O3 => {
            dead_code_elimination(functions);
            constant_folding(functions);
            local_cse(functions);
            copy_propagation(functions);
            loop_invariant_code_motion(functions);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    O0,
    O1,
    O2,
    O3,
}

impl OptimizationLevel {
    pub fn from_str(s: &str) -> Self {
        match s {
            "0" | "O0" => OptimizationLevel::O0,
            "1" | "O1" => OptimizationLevel::O1,
            "2" | "O2" => OptimizationLevel::O2,
            "3" | "O3" => OptimizationLevel::O3,
            _ => OptimizationLevel::O2,
        }
    }
}

/// Dead code elimination: remove instructions whose results are never used.
pub fn dead_code_elimination(functions: &mut Vec<Function>) {
    for func in functions.iter_mut() {
        for block in func.blocks.iter_mut() {
            let mut used = vec![false; block.instructions.len() + 1]; // +1 for block args

            // Mark all uses
            for inst in &block.instructions {
                for op in &inst.operands {
                    if let Operand::Reg(reg_id) = op {
                        if *reg_id < used.len() {
                            used[*reg_id] = true;
                        }
                    }
                }
            }
            // Mark terminators as used
            used[block.instructions.len()] = true; // terminator result if any

            // Remove unused instructions (skip terminators and side-effect instructions)
            let mut new_insts = Vec::new();
            for (i, inst) in block.instructions.iter().enumerate() {
                if inst.flags.is_terminator
                    || inst.flags.has_side_effect
                    || (i < used.len() && used[i])
                {
                    new_insts.push(inst.clone());
                }
            }
            block.instructions = new_insts;
        }
    }
}

/// Constant folding: evaluate instructions with all-constant operands at compile time.
pub fn constant_folding(functions: &mut Vec<Function>) {
    for func in functions.iter_mut() {
        for block in func.blocks.iter_mut() {
            for inst in block.instructions.iter_mut() {
                if inst.flags.has_side_effect || inst.flags.is_terminator {
                    continue;
                }

                let folded = match inst.opcode {
                    Opcode::Add => fold_binary_i64(inst, |a, b| Some(a.wrapping_add(b))),
                    Opcode::Sub => fold_binary_i64(inst, |a, b| Some(a.wrapping_sub(b))),
                    Opcode::Mul => fold_binary_i64(inst, |a, b| Some(a.wrapping_mul(b))),
                    Opcode::Div => fold_binary_i64(inst, |a, b| {
                        if b == 0 { None } else { Some(a / b) }
                    }),
                    Opcode::Eq => fold_binary_i64(inst, |a, b| Some(if a == b { 1 } else { 0 })),
                    Opcode::Neq => fold_binary_i64(inst, |a, b| Some(if a != b { 1 } else { 0 })),
                    Opcode::Lt => fold_binary_i64(inst, |a, b| Some(if a < b { 1 } else { 0 })),
                    Opcode::Gt => fold_binary_i64(inst, |a, b| Some(if a > b { 1 } else { 0 })),
                    Opcode::Not => fold_unary_i64(inst, |a| Some(if a == 0 { 1 } else { 0 })),
                    Opcode::Neg => fold_unary_i64(inst, |a| Some(-a)),
                    _ => None,
                };

                if let Some(val) = folded {
                    *inst = Instruction {
                        opcode: Opcode::Add, // placeholder
                        operands: vec![Operand::Immediate(val)],
                        result_type: IrType::I64,
                        flags: InstructionFlags::new(),
                        result_id: inst.result_id,
                    };
                    // Actually, we should use a proper 'const' opcode. For now,
                    // replace with a mov-like pattern: use Add with 0 + val
                    inst.opcode = Opcode::Add;
                    inst.operands = vec![Operand::Immediate(0), Operand::Immediate(val)];
                }
            }
        }
    }
}

fn fold_binary_i64(inst: &Instruction, f: fn(i64, i64) -> Option<i64>) -> Option<i64> {
    if inst.operands.len() == 2 {
        if let (Operand::Immediate(a), Operand::Immediate(b)) = (&inst.operands[0], &inst.operands[1]) {
            return f(*a, *b);
        }
    }
    None
}

fn fold_unary_i64(inst: &Instruction, f: fn(i64) -> Option<i64>) -> Option<i64> {
    if inst.operands.len() == 1 {
        if let Operand::Immediate(a) = &inst.operands[0] {
            return f(*a);
        }
    }
    None
}

/// Local common subexpression elimination (within a basic block).
pub fn local_cse(functions: &mut Vec<Function>) {
    for func in functions.iter_mut() {
        for block in func.blocks.iter_mut() {
            let mut seen: std::collections::HashMap<(Opcode, Vec<Operand>), RegId> =
                std::collections::HashMap::new();
            let mut replacements: std::collections::HashMap<RegId, RegId> =
                std::collections::HashMap::new();

            for inst in block.instructions.iter_mut() {
                // Don't CSE side-effecting or terminator instructions
                if inst.flags.has_side_effect || inst.flags.is_terminator {
                    continue;
                }

                let key = (inst.opcode.clone(), inst.operands.clone());
                if let Some(&existing_reg) = seen.get(&key) {
                    replacements.insert(inst.result_id, existing_reg);
                    // Mark this instruction for removal (set opcode to Phi as sentinel)
                    inst.flags.has_side_effect = true; // will be DCE'd
                } else {
                    seen.insert(key, inst.result_id);
                }
            }

            // Apply replacements to remaining instructions
            for inst in block.instructions.iter_mut() {
                for op in inst.operands.iter_mut() {
                    if let Operand::Reg(ref mut reg) = op {
                        if let Some(&replacement) = replacements.get(reg) {
                            *reg = replacement;
                        }
                    }
                }
            }
        }
    }
}

/// Copy propagation: replace register uses with the source register when a
/// register is only defined as a copy of another register.
pub fn copy_propagation(functions: &mut Vec<Function>) {
    for func in functions.iter_mut() {
        for block in func.blocks.iter_mut() {
            let mut copies: std::collections::HashMap<RegId, RegId> =
                std::collections::HashMap::new();

            // Find copy instructions (add with 0, etc.)
            for inst in &block.instructions {
                if inst.opcode == Opcode::Add
                    && inst.operands.len() == 2
                    && inst.operands[0] == Operand::Immediate(0)
                {
                    if let Operand::Reg(src) = inst.operands[1] {
                        copies.insert(inst.result_id, src);
                    }
                }
            }

            // Propagate copies
            for inst in block.instructions.iter_mut() {
                for op in inst.operands.iter_mut() {
                    if let Operand::Reg(ref mut reg) = op {
                        while let Some(&target) = copies.get(reg) {
                            *reg = target;
                        }
                    }
                }
            }
        }
    }
}

/// Loop invariant code motion: move loop-invariant instructions to the pre-header.
pub fn loop_invariant_code_motion(_functions: &mut Vec<Function>) {
    // Stub: requires loop analysis (dominator tree, loop detection).
    // Full implementation deferred to Phase 2.
}
