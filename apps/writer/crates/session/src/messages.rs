//! UI wording uses the canonical Fluent catalogue, separate from dialogue PO.
pub use recite_ui::{MsgId, UiArg};
use recite_ui::{UiCatalog, UiLocale};
thread_local! {
    static MESSAGES: Result<UiCatalog, recite_ui::CatalogError> = UiCatalog::load(&UiLocale::default());
}
pub fn text(id: MsgId) -> String {
    MESSAGES.with(|catalogue| match catalogue {
        Ok(catalogue) => catalogue.text(id),
        Err(error) => format!("[UI text unavailable: {error}]"),
    })
}
/// Wording with named arguments, such as a suggested replacement or a line number.
pub fn format<const N: usize, V: Into<recite_ui::UiArg>>(
    id: MsgId,
    arguments: [(&str, V); N],
) -> String {
    MESSAGES.with(|catalogue| match catalogue {
        Ok(catalogue) => catalogue.format_pairs(id, arguments),
        Err(error) => format!("[UI text unavailable: {error}]"),
    })
}
