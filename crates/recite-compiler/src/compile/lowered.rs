use recite_core::SourceFile;

#[derive(Clone, Debug)]
pub(super) struct LoweredInput {
    pub(super) input_index: usize,
    pub(super) source: String,
    pub(super) source_file: SourceFile,
}
