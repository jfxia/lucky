pub mod lower;
pub mod optimize;


pub type BlockId = usize;
pub type RegId = usize;
pub type ConstId = usize;

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub arguments: Vec<(String, IrType)>,
    pub instructions: Vec<Instruction>,
    pub terminator: TerminatorInst,
    pub predecessors: Vec<BlockId>,
}

impl BasicBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            arguments: Vec::new(),
            instructions: Vec::new(),
            terminator: TerminatorInst::Unreachable,
            predecessors: Vec::new(),
        }
    }

    pub fn push_instruction(&mut self, inst: Instruction) -> RegId {
        let result = inst.result_id;
        self.instructions.push(inst);
        result
    }

    pub fn set_terminator(&mut self, terminator: TerminatorInst) {
        self.terminator = terminator;
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub id: usize,
    pub name: String,
    pub params: Vec<(String, IrType)>,
    pub return_type: IrType,
    pub entry_block: BlockId,
    pub blocks: Vec<BasicBlock>,
}

impl Function {
    pub fn new(id: usize, name: String, params: Vec<(String, IrType)>, return_type: IrType, entry_block: BlockId) -> Self {
        Self {
            id,
            name,
            params,
            return_type,
            entry_block,
            blocks: Vec::new(),
        }
    }

    pub fn add_block(&mut self, block: BasicBlock) -> BlockId {
        let id = block.id;
        self.blocks.push(block);
        id
    }

    pub fn get_block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.iter().find(|b| b.id == id)
    }

    pub fn get_block_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock> {
        self.blocks.iter_mut().find(|b| b.id == id)
    }

    pub fn add_predecessor(&mut self, block_id: BlockId, pred_id: BlockId) {
        if let Some(block) = self.blocks.iter_mut().find(|b| b.id == block_id) {
            if !block.predecessors.contains(&pred_id) {
                block.predecessors.push(pred_id);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operands: Vec<Operand>,
    pub result_type: IrType,
    pub flags: InstructionFlags,
    pub result_id: RegId,
}

impl Instruction {
    pub fn new(opcode: Opcode, operands: Vec<Operand>, result_type: IrType, result_id: RegId) -> Self {
        let flags = InstructionFlags::for_opcode(&opcode);
        Self {
            opcode,
            operands,
            result_type,
            flags,
            result_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerminatorInst {
    Br {
        successor: BlockId,
        args: Vec<Operand>,
    },
    CondBr {
        condition: Operand,
        true_block: BlockId,
        true_args: Vec<Operand>,
        false_block: BlockId,
        false_args: Vec<Operand>,
    },
    Ret {
        value: Option<Operand>,
    },
    Unreachable,
}

impl TerminatorInst {
    pub fn is_terminator(&self) -> bool {
        true
    }

    pub fn successor_blocks(&self) -> Vec<BlockId> {
        match self {
            TerminatorInst::Br { successor, .. } => vec![*successor],
            TerminatorInst::CondBr { true_block, false_block, .. } => vec![*true_block, *false_block],
            TerminatorInst::Ret { .. } => Vec::new(),
            TerminatorInst::Unreachable => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    Reg(RegId),
    Const(ConstId),
    Symbol(String),
    Block(BlockId),
    Immediate(i64),
}

impl Operand {
    pub fn is_reg(&self) -> bool {
        matches!(self, Operand::Reg(_))
    }

    pub fn is_const(&self) -> bool {
        matches!(self, Operand::Const(_))
    }

    pub fn is_immediate(&self) -> bool {
        matches!(self, Operand::Immediate(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opcode {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Not,
    Neg,
    Call,
    LlmComplete,
    ToolInvoke,
    AgentInvoke,
    Alloca,
    Load,
    Store,
    BrOp,
    CondBrOp,
    RetOp,
    ListNew,
    ListGet,
    MapNew,
    MapGet,
    StrConcat,
    StrLen,
    Cast,
    Phi,
}

impl Opcode {
    pub fn from_bin_op(op: &crate::ast::BinOp) -> Self {
        match op {
            crate::ast::BinOp::Add => Opcode::Add,
            crate::ast::BinOp::Sub => Opcode::Sub,
            crate::ast::BinOp::Mul => Opcode::Mul,
            crate::ast::BinOp::Div => Opcode::Div,
            crate::ast::BinOp::Eq => Opcode::Eq,
            crate::ast::BinOp::Neq => Opcode::Neq,
            crate::ast::BinOp::Lt => Opcode::Lt,
            crate::ast::BinOp::Gt => Opcode::Gt,
            crate::ast::BinOp::Le => Opcode::Le,
            crate::ast::BinOp::Ge => Opcode::Ge,
            crate::ast::BinOp::And => Opcode::And,
            crate::ast::BinOp::Or => Opcode::Or,
            crate::ast::BinOp::Concat => Opcode::StrConcat,
            _ => Opcode::Call,
        }
    }

    pub fn from_unary_op(op: &crate::ast::UnaryOp) -> Self {
        match op {
            crate::ast::UnaryOp::Neg => Opcode::Neg,
            crate::ast::UnaryOp::Not => Opcode::Not,
        }
    }

    pub fn is_binary(&self) -> bool {
        matches!(
            self,
            Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div
                | Opcode::Eq | Opcode::Neq | Opcode::Lt | Opcode::Gt
                | Opcode::Le | Opcode::Ge | Opcode::And | Opcode::Or
                | Opcode::StrConcat
        )
    }

    pub fn is_unary(&self) -> bool {
        matches!(self, Opcode::Not | Opcode::Neg | Opcode::Cast)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionFlags {
    pub has_side_effect: bool,
    pub is_terminator: bool,
    pub is_volatile: bool,
}

impl InstructionFlags {
    pub fn new() -> Self {
        Self {
            has_side_effect: false,
            is_terminator: false,
            is_volatile: false,
        }
    }

    pub fn for_opcode(opcode: &Opcode) -> Self {
        let has_side_effect = matches!(
            opcode,
            Opcode::Call | Opcode::LlmComplete | Opcode::ToolInvoke
                | Opcode::AgentInvoke | Opcode::Store
        );
        let is_terminator = matches!(opcode, Opcode::BrOp | Opcode::CondBrOp | Opcode::RetOp);
        let is_volatile = matches!(opcode, Opcode::Load | Opcode::Store);

        Self {
            has_side_effect,
            is_terminator,
            is_volatile,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrType {
    I1,
    I64,
    F64,
    StringType,
    BytesType,
    List(Box<IrType>),
    Map(Box<IrType>, Box<IrType>),
    Void,
    Agent(String),
    Task(String),
}

impl IrType {
    pub fn from_ast_type(typ: &crate::ast::TypeExpr) -> Self {
        match typ {
            crate::ast::TypeExpr::Primitive { name, .. } => match name.as_str() {
                "Bool" => IrType::I1,
                "Int" => IrType::I64,
                "Float" | "Decimal" => IrType::F64,
                "String" => IrType::StringType,
                "Bytes" => IrType::BytesType,
                _ => IrType::StringType,
            },
            crate::ast::TypeExpr::Named { name, .. } => match name.as_str() {
                "Agent" => IrType::Agent(String::new()),
                "Task" => IrType::Task(String::new()),
                _ => IrType::StringType,
            },
            crate::ast::TypeExpr::List { element, .. } => {
                IrType::List(Box::new(IrType::from_ast_type(element)))
            }
            crate::ast::TypeExpr::Map { key, value, .. } => {
                IrType::Map(
                    Box::new(IrType::from_ast_type(key)),
                    Box::new(IrType::from_ast_type(value)),
                )
            }
            _ => IrType::Void,
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, IrType::I64)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, IrType::F64)
    }

    pub fn is_void(&self) -> bool {
        matches!(self, IrType::Void)
    }
}
