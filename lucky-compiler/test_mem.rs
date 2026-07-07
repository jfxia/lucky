use std::fs;
use lucky_compiler::ast::span::FileId;
use lucky_compiler::lexer::Lexer;

fn main() {
    let src = fs::read_to_string("examples/data_pipeline/etl.lk").unwrap();
    let file_id = FileId::new(0);
    let mut lexer = Lexer::new(&src, file_id);
    let tokens = lexer.tokenize();
    eprintln!("Token count: {}", tokens.len());
    eprintln!("Error count: {}", lexer.errors.len());
    // Print first 20 tokens
    for t in tokens.iter().take(20) {
        eprintln!("  {:?} {:?}", t.kind, t.text);
    }
    eprintln!("...");
    for t in tokens.iter().rev().take(10).rev() {
        eprintln!("  {:?} {:?}", t.kind, t.text);
    }
}
