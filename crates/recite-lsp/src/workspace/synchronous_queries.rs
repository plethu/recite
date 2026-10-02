use super::LspWorkspace;
#[cfg(feature = "bench-support")]
use lsp_types::{CompletionResponse, WorkspaceEdit};
use lsp_types::{GotoDefinitionResponse, Position, Uri};
use recite_compiler::authoring::CancellationToken;
impl LspWorkspace {
    #[cfg(feature = "bench-support")]
    pub(crate) fn completion(&self, uri: &Uri, position: Position) -> Option<CompletionResponse> {
        self.completion_with_control(uri, position, &CancellationToken::new())
    }
    pub(crate) fn definition(
        &self,
        uri: &Uri,
        position: Position,
    ) -> Option<GotoDefinitionResponse> {
        self.definition_with_control(uri, position, &CancellationToken::new())
    }
    #[cfg(feature = "bench-support")]
    pub(crate) fn rename(
        &self,
        uri: &Uri,
        position: Position,
        new_name: &str,
    ) -> Option<WorkspaceEdit> {
        self.rename_with_control(uri, position, new_name, &CancellationToken::new())
    }
}
