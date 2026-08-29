use crate::passes::gc_sections::RemovedSection;
use cuddld_diagnostics::widgets::Table;
use cuddld_diagnostics::{Diagnostic, DiagnosticKind};

pub(super) fn render_gc(removed: &[RemovedSection]) -> Diagnostic {
    let mut removed_table = Table::new();
    removed_table.set_title("Removed sections:");
    removed_table.add_head(["Section name", "Source"]);
    for section in removed {
        removed_table.add_body([section.name.resolve().to_string(), section.source.to_string()]);
    }

    Diagnostic::new(DiagnosticKind::DebugPrint, "garbage collector outcome").add(removed_table)
}
