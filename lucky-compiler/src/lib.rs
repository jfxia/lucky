pub mod ast;
pub mod lexer;
pub mod parser;
pub mod diagnostics;
pub mod lsp;
pub mod semantic;
pub mod hir;
pub mod mir;
pub mod ir_serialize;
pub mod runtime;
pub mod pkg;
pub mod test_runner;
pub mod format;
pub mod debug;

use ast::span::FileId;

/// Compile a Lucky source file and return the parsed AST module.
pub fn compile(source: &str, file_id: FileId) -> (ast::Module, diagnostics::DiagnosticBag) {
    let mut lexer = lexer::Lexer::new(source, file_id);
    let tokens = lexer.tokenize();

    let mut parser = parser::Parser::new(tokens, file_id);
    let module = parser.parse_module();

    let mut diagnostics = diagnostics::DiagnosticBag::new();
    diagnostics.extend(parser.diagnostics);

    (module, diagnostics)
}

/// Full compilation pipeline: source -> AST -> semantic analysis -> HIR -> MIR -> optimized MIR.
pub fn compile_to_ir(
    source: &str,
    file_id: FileId,
    opt_level: mir::optimize::OptimizationLevel,
) -> CompileResult {
    let (module, diagnostics) = compile(source, file_id);

    // Semantic analysis
    let mut resolver = semantic::resolver::NameResolver::new();
    let resolved = resolver.resolve_module(module);

    // HIR construction
    let hir_graph = hir::builder::HirBuilder::new().build_module(&resolved.module);

    // MIR lowering
    let mut mir_functions = mir::lower::MirLowering::new().lower_graph(&hir_graph);

    // Optimize
    mir::optimize::optimize(&mut mir_functions, opt_level);

    CompileResult {
        diagnostics,
        hir_json: Some(ir_serialize::serialize_hir(&hir_graph)),
        mir_json: Some(ir_serialize::serialize_mir(&mir_functions)),
        node_count: hir_graph.nodes.len(),
        function_count: mir_functions.len(),
    }
}

pub struct CompileResult {
    pub diagnostics: diagnostics::DiagnosticBag,
    pub hir_json: Option<String>,
    pub mir_json: Option<String>,
    pub node_count: usize,
    pub function_count: usize,
}

/// Lex a Lucky source file and return tokens (useful for debugging).
pub fn tokenize(source: &str, file_id: FileId) -> (Vec<lexer::Token>, Vec<String>) {
    let mut lexer = lexer::Lexer::new(source, file_id);
    let tokens = lexer.tokenize();
    let errors = lexer.errors().to_vec();
    (tokens, errors)
}
