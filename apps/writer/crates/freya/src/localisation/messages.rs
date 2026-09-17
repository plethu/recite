//! UI wording uses the canonical Fluent catalogue, separate from dialogue PO.
pub(super) use recite_ui::MsgId;
use recite_ui::{UiCatalog, UiLocale};
thread_local! {
    static MESSAGES: Result<UiCatalog, recite_ui::CatalogError> = UiCatalog::load(&UiLocale::default());
}
pub(super) fn text(id: MsgId) -> String {
    MESSAGES.with(|catalogue| match catalogue {
        Ok(catalogue) => catalogue.text(id),
        Err(error) => format!("[UI text unavailable: {error}]"),
    })
}

/// Regional spelling is UI locale policy, independent of the dialogue target.
pub(super) fn mode_label(locale: &UiLocale) -> String {
    let locale = locale.resolve();
    text(
        if locale.language.as_str() == "en" && locale.region.is_none_or(|r| r.as_str() != "US") {
            MsgId::WriterLocalise
        } else {
            MsgId::WriterLocalisation
        },
    )
}

#[cfg(test)]
mod tests;
