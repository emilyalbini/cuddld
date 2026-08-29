mod builder;
mod diagnostic;
mod span;
pub mod widgets;
mod writer;

pub use crate::builder::{DiagnosticBuilder, DiagnosticContext, GatheredContext};
pub use crate::diagnostic::{Diagnostic, DiagnosticKind};
pub use crate::span::ObjectSpan;
pub use crate::writer::WidgetWriter;
