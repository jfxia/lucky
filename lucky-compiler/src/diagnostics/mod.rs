pub mod diagnostic;
pub mod reporter;

pub use diagnostic::{Diagnostic, DiagnosticBag, Severity};
pub use reporter::print_diagnostics;
