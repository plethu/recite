//! UI wording uses the canonical Fluent catalogue, separate from dialogue PO.
pub use recite_ui::MsgId;
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
/// Wording with named string arguments, such as a suggested replacement.
pub fn format<const N: usize>(id: MsgId, arguments: [(&str, &str); N]) -> String {
    MESSAGES.with(|catalogue| match catalogue {
        Ok(catalogue) => catalogue.format_pairs(id, arguments),
        Err(error) => format!("[UI text unavailable: {error}]"),
    })
}
