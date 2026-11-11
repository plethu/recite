//! Text and JSON renderings of one catalogue's review status.

use std::io::Write;

use recite_core::po::PoReviewState;
use recite_ui::UiArg;
use serde_json::json;

use super::{Attention, Catalog, Status};
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

pub(super) fn write_text(
    stdout: &mut dyn Write,
    messages: &Messages,
    catalog: &Catalog,
    status: &Status,
) -> Result<(), CliError> {
    let locale = status
        .locale
        .clone()
        .unwrap_or_else(|| messages.text(MsgId::CliCatalogNoLocale));
    writeln!(
        stdout,
        "{}",
        messages.format(
            MsgId::CliCatalogSummary,
            [
                ("catalog", UiArg::from(catalog.display.clone())),
                ("locale", UiArg::from(locale)),
                ("reviewed", UiArg::from(status.reviewed)),
                ("attention", UiArg::from(status.items.len())),
            ],
        )
    )?;
    if status.refresh_needed {
        writeln!(stdout, "{}", messages.text(MsgId::CliCatalogRefreshNeeded))?;
    }
    for item in &status.items {
        writeln!(
            stdout,
            "{}",
            messages.format(
                MsgId::CliCatalogEntry,
                [
                    ("state", item.attention.label(messages)),
                    ("context", item.context.clone()),
                    ("reference", item.reference.clone().unwrap_or_default()),
                ],
            )
        )?;
    }
    Ok(())
}

pub(super) fn json_status(catalog: &Catalog, status: &Status) -> serde_json::Value {
    let count = |attention: Attention| {
        status
            .items
            .iter()
            .filter(|item| item.attention.key() == attention.key())
            .count()
    };
    json!({
        "path": catalog.display,
        "locale": status.locale,
        "refresh_needed": status.refresh_needed,
        "counts": {
            "reviewed": status.reviewed,
            "missing": count(Attention::Missing),
            "untranslated": count(Attention::Review(PoReviewState::Untranslated)),
            "needs_review": count(Attention::Review(PoReviewState::NeedsReview)),
            "source_changed": count(Attention::Review(PoReviewState::SourceChanged)),
            "removed_from_source": status.removed,
        },
        "attention": status.items.iter().map(|item| json!({
            "context": item.context,
            "state": item.attention.key(),
            "reference": item.reference,
        })).collect::<Vec<_>>(),
    })
}
