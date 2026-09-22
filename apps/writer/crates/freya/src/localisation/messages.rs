pub(crate) use crate::messages::{MsgId, text};
use recite_ui::UiLocale;

/// Regional spelling is UI locale policy, independent of the dialogue target.
pub(crate) fn mode_label(locale: &UiLocale) -> String {
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
