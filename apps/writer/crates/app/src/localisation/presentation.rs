//! Prepare bounded catalogue presentation on the worker.
pub use super::slot::Owner;
use super::*;
use recite_writer_model::Passage;
use recite_writer_session::localisation::{
    context_data, navigation::Destination as SourceDestination,
};

pub struct Presentation {
    pub(crate) queue: Queue,
    pub(crate) entry: Option<Entry>,
    pub(crate) message: &'static str,
}
pub(super) fn describe(
    owner: &Owner,
    query: &Query,
    selected: Option<PoEntryId>,
    passages: Option<Arc<[Passage]>>,
) -> Result<Presentation, String> {
    let catalogue = &owner.catalogue;
    let index = catalogue.queue_index(passages.clone());
    let mut matches = index.matching_entries(
        catalogue,
        &query.text.trim().to_lowercase(),
        query.filter == Filter::Attention,
    )?;
    if query.filter == Filter::Unsaved {
        matches.retain(|id| catalogue.changed(*id));
    }
    let page = query.page.min(matches.len().saturating_sub(1) / 8);
    let rows = matches
        .iter()
        .skip(page * 8)
        .take(8)
        .filter_map(|id| {
            let entry = catalogue.document.entry(*id)?;
            Some(Row {
                id: *id,
                source: entry.source_text().chars().take(160).collect(),
                caption: SourceDestination::resolve(entry, passages.as_deref().unwrap_or_default())
                    .map_or_else(
                        || entry.context().unwrap_or("Catalogue entry").to_owned(),
                        |destination| destination.caption(),
                    )
                    .chars()
                    .take(160)
                    .collect(),
                status: TranslationStatus::entry_label(catalogue, *id),
                translation: catalogue
                    .forms(*id)
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .take(160)
                    .collect(),
            })
        })
        .collect();
    let entry =
        selected.and_then(|id| describe_entry(owner, id, passages.as_deref().unwrap_or_default()));
    Ok(Presentation {
        queue: Queue {
            rows,
            total: matches.len(),
            page,
        },
        entry,
        message: "Catalogue ready. Drafts stay in Writer until you Save.",
    })
}
fn describe_entry(owner: &Owner, id: PoEntryId, passages: &[Passage]) -> Option<Entry> {
    let catalogue = &owner.catalogue;
    let entry = catalogue.document.entry(id)?;
    let details = catalogue.entry_details(id)?;
    let rule = details.plural.rule().map(text::pages).unwrap_or_default();
    Some(Entry {
        key: EntryKey {
            catalogue: owner.identity.clone(),
            id,
        },
        source: text::pages(&context_data::source(entry)),
        context: context_data::Nearby::new(&catalogue.document)
            .for_entry(entry)
            .iter()
            .flat_map(|source| text::pages(source))
            .collect(),
        source_page: 0,
        context_page: 0,
        form_page: 0,
        details,
        wording_page: 0,
        source_location: SourceDestination::resolve(entry, passages)
            .map(|destination| destination.caption().chars().take(160).collect()),
        rule: PluralRuleView {
            pages: rule,
            page: 0,
            expanded: false,
        },
    })
}
