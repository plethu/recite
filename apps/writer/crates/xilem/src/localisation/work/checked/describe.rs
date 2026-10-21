//! Prepare bounded catalogue presentation on the worker.
use super::*;
pub(in crate::localisation::work) fn describe(
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
            })
        })
        .collect();
    let entry = selected.and_then(|id| {
        let entry = catalogue.document.entry(id)?;
        (!entry.is_header() && !entry.is_obsolete()).then(|| Entry {
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
        })
    });
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
