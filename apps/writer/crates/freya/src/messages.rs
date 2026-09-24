//! UI wording uses the canonical Fluent catalogue, separate from dialogue PO.
pub(crate) use recite_ui::MsgId;
use recite_ui::{UiCatalog, UiLocale};
thread_local! {
    static MESSAGES: Result<UiCatalog, recite_ui::CatalogError> = UiCatalog::load(&UiLocale::default());
}
pub(crate) fn text(id: MsgId) -> String {
    MESSAGES.with(|catalogue| match catalogue {
        Ok(catalogue) => catalogue.text(id),
        Err(error) => format!("[UI text unavailable: {error}]"),
    })
}
