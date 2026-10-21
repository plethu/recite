//! Analysis identity without retaining a project graph for a queued UI action.
use super::{Document, ProjectContext};
use recite_compiler::authoring::{AuthoringKernel, DocumentVersion};
use recite_core::DocumentKey;
use std::sync::{Arc, Weak};

/// A freshness check, not a usable analysis snapshot.
///
/// Weak identities prevent address reuse without keeping analysis payloads alive.
/// The source's inline allocation remains until the stamp is released.
#[derive(Clone)]
pub struct AnalysisStamp {
    key: DocumentKey,
    version: DocumentVersion,
    source: Weak<str>,
    kernel: Weak<AuthoringKernel>,
    context: Weak<ProjectContext>,
}
impl Document {
    pub fn analysis_stamp(&self) -> AnalysisStamp {
        AnalysisStamp {
            key: self.key.clone(),
            version: self.version,
            source: Arc::downgrade(&self.source),
            kernel: Arc::downgrade(&self.kernel),
            context: Arc::downgrade(&self.context),
        }
    }
}
impl AnalysisStamp {
    pub fn is_current(&self, document: &Document) -> bool {
        self.key == document.key
            && self.version == document.version
            && Weak::ptr_eq(&self.source, &Arc::downgrade(&document.source))
            && Weak::ptr_eq(&self.kernel, &Arc::downgrade(&document.kernel))
            && Weak::ptr_eq(&self.context, &Arc::downgrade(&document.context))
    }
}
impl PartialEq for AnalysisStamp {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.version == other.version
            && Weak::ptr_eq(&self.source, &other.source)
            && Weak::ptr_eq(&self.kernel, &other.kernel)
            && Weak::ptr_eq(&self.context, &other.context)
    }
}
impl Eq for AnalysisStamp {}

#[cfg(test)]
mod tests;
