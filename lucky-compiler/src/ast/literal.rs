
/// Literal values in Lucky source.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Null,
    Unknown,
}

impl Literal {
    pub fn type_name(&self) -> &str {
        match self {
            Literal::Bool(_) => "Bool",
            Literal::Int(_) => "Int",
            Literal::Float(_) => "Float",
            Literal::String(_) => "String",
            Literal::Null => "null",
            Literal::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Bool(b) => write!(f, "{}", b),
            Literal::Int(i) => write!(f, "{}", i),
            Literal::Float(n) => write!(f, "{}", n),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::Null => write!(f, "null"),
            Literal::Unknown => write!(f, "unknown"),
        }
    }
}
