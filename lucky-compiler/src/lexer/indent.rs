use crate::ast::span::{FileId, Span};
use super::token::{Token, TokenKind};

/// Processes raw tokens to insert INDENT/DEDENT tokens based on leading whitespace.
pub struct IndentProcessor {
    indent_stack: Vec<usize>,
    file_id: FileId,
    pending_dedents: usize,
    seen_token_on_line: bool,
    current_line: usize,
    at_line_start: bool,
    /// The byte offset of the start of the current line (for computing indentation).
    line_start_offset: usize,
}

impl IndentProcessor {
    pub fn new(file_id: FileId) -> Self {
        Self {
            indent_stack: vec![0],
            file_id,
            pending_dedents: 0,
            seen_token_on_line: false,
            current_line: 0,
            at_line_start: true,
            line_start_offset: 0,
        }
    }

    /// Process a stream of raw tokens, inserting INDENT/DEDENT tokens.
    /// Returns a Vec of tokens including the synthetic INDENT/DEDENT tokens.
    /// The input tokens must NOT already contain INDENT/DEDENT; they should
    /// include NEWLINE tokens at the end of each logical line.
    pub fn process(&mut self, raw_tokens: Vec<Token>) -> Vec<Token> {
        let mut result = Vec::with_capacity(raw_tokens.len() + 16);

        for token in raw_tokens {
            match token.kind {
                TokenKind::Newline => {
                    self.seen_token_on_line = false;
                    self.at_line_start = true;
                    // Track the offset where the next line starts
                    self.line_start_offset = token.span.end;
                    // Emit NEWLINE tokens so the parser can use them
                    result.push(Token::new(TokenKind::Newline, "\\n", token.span));
                }
                TokenKind::Comment | TokenKind::DocComment => {
                    // Comments are preserved but don't affect indentation tracking
                    result.push(token);
                }
                TokenKind::Eof => {
                    // Emit DEDENTs back to level 0 before EOF
                    while self.indent_stack.len() > 1 {
                        self.indent_stack.pop();
                        let span = Span::new(token.span.start, token.span.start, self.file_id);
                        result.push(Token::new(TokenKind::Dedent, "", span));
                    }
                    result.push(token);
                }
                _ => {
                    if self.at_line_start {
                        self.at_line_start = false;
                        // Compute indentation as byte offset from start of line
                        let indent = token.span.start - self.line_start_offset;
                        self.handle_indent(indent, token.span, &mut result);
                    }
                    self.seen_token_on_line = true;
                    result.push(token);
                }
            }
        }

        result
    }

    fn handle_indent(&mut self, column: usize, span: Span, result: &mut Vec<Token>) {
        let current_indent = *self.indent_stack.last().unwrap_or(&0);

        if column > current_indent {
            self.indent_stack.push(column);
            let span = Span::new(span.start, span.start, self.file_id);
            result.push(Token::new(TokenKind::Indent, "", span));
        } else if column < current_indent {
            // Pop back to the matching indentation level
            while self.indent_stack.len() > 1 && column < *self.indent_stack.last().unwrap() {
                self.indent_stack.pop();
                let sp = Span::new(span.start, span.start, self.file_id);
                result.push(Token::new(TokenKind::Dedent, "", sp));
            }
            // Verify we landed on a valid indent level
            if self.indent_stack.len() > 1 && column != *self.indent_stack.last().unwrap() {
                // Force recovery: push the new level
                self.indent_stack.push(column);
            }
        }
    }
}
