//! An indexed borrowed view over immutable accepted diagnostic reports.
use super::Document;
use recite_compiler::authoring::AuthoringKernel;
use recite_core::Diagnostic;
use std::sync::Arc;

#[derive(Clone)]
pub struct DiagnosticSnapshot {
    kernel: Arc<AuthoringKernel>,
}
impl PartialEq for DiagnosticSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.kernel, &other.kernel)
    }
}
impl Document {
    pub fn diagnostic_snapshot(&self) -> DiagnosticSnapshot {
        DiagnosticSnapshot {
            kernel: self.kernel.clone(),
        }
    }
}
impl DiagnosticSnapshot {
    pub fn len(&self) -> usize {
        self.kernel.snapshot().diagnostics().len()
    }
    pub fn is_empty(&self) -> bool {
        self.kernel.snapshot().diagnostics().is_empty()
    }
    pub fn get(&self, mut index: usize) -> Option<&Diagnostic> {
        for document in self.kernel.snapshot().documents() {
            let diagnostics = document.diagnostics();
            if index < diagnostics.len() {
                return diagnostics.get(index);
            }
            index -= diagnostics.len();
        }
        None
    }
}
