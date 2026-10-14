use std::sync::Arc;

use recite_core::ast::SourceFile;

#[derive(Clone, Debug)]
pub(super) struct LoweredInput {
    pub(super) input_index: usize,
    pub(super) source: Arc<str>,
    pub(super) fingerprint: Option<recite_core::compiled::ContentFingerprint>,
    pub(super) source_file: SourceFile,
}
