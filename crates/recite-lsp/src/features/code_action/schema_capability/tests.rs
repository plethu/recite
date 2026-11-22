use lsp_types::CodeActionOrCommand;
use recite_compiler::authoring::SchemaSummary;
use recite_ui::UiCatalog;

/// Every declaration's actions, for tests of the projection itself.
pub(crate) fn actions(
    summary: &SchemaSummary,
    editable_source_open: bool,
    catalog: &UiCatalog,
) -> Vec<CodeActionOrCommand> {
    super::project(
        &super::collect::collect(summary),
        editable_source_open,
        catalog,
    )
}
