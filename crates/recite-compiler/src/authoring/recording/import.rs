//! Validate returned handoff rows against current items, then fold them into the sidecar.

use std::collections::{BTreeMap, BTreeSet};

use recite_core::recording::{HandoffRow, RecordingSidecar, SidecarItem, StoredStatus, Take};

use super::RecordingItem;

/// Why one returned row cannot be applied. Nothing is applied while any row is refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportRefusal {
    UnknownId,
    WrongLocale { found: String },
    DuplicateTake,
    NotReady,
    StaleRevision { current: String },
    MissingMedia,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowRefusal {
    /// The row's source line in CSV, or its 1-based position in JSON.
    pub row: usize,
    pub id: String,
    pub take: String,
    pub reason: ImportRefusal,
}

/// The sidecar after an import, and which takes it adds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportPlan {
    pub sidecar: RecordingSidecar,
    /// `(id, take reference)` pairs new to the sidecar, in row order.
    pub added: Vec<(String, String)>,
    /// Rows whose take the sidecar already holds.
    pub already_present: usize,
}

/// Rows without a `take` are skipped: they are lines the collaborator did not record.
/// `media_exists` answers for project-relative references; references with a URI scheme are
/// accepted as external.
pub fn plan_import(
    items: &[RecordingItem],
    sidecar: &RecordingSidecar,
    locale: Option<&str>,
    rows: &[(usize, HandoffRow)],
    media_exists: &dyn Fn(&str) -> bool,
) -> Result<ImportPlan, Vec<RowRefusal>> {
    let items = items
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let expected_locale = locale.unwrap_or_default();
    let mut refusals = Vec::new();
    let mut seen = BTreeSet::new();
    let returned = rows.iter().filter(|(_, row)| !row.take.trim().is_empty());
    for (line, row) in returned.clone() {
        let take = row.take.trim();
        let refuse = |reason| RowRefusal {
            row: *line,
            id: row.id.clone(),
            take: take.to_owned(),
            reason,
        };
        if row.locale != expected_locale {
            refusals.push(refuse(ImportRefusal::WrongLocale {
                found: row.locale.clone(),
            }));
        } else if !seen.insert((row.id.as_str(), take)) {
            refusals.push(refuse(ImportRefusal::DuplicateTake));
        } else if let Some(item) = items.get(row.id.as_str()) {
            match &item.revision {
                None => refusals.push(refuse(ImportRefusal::NotReady)),
                Some(current) if current.as_str() != row.revision.trim() => {
                    refusals.push(refuse(ImportRefusal::StaleRevision {
                        current: current.to_string(),
                    }));
                }
                Some(_) if !take.contains("://") && !media_exists(take) => {
                    refusals.push(refuse(ImportRefusal::MissingMedia));
                }
                Some(_) => {}
            }
        } else {
            refusals.push(refuse(ImportRefusal::UnknownId));
        }
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }

    let mut updated = sidecar.clone();
    let mut added = Vec::new();
    let mut already_present = 0;
    for (_, row) in returned {
        let take = row.take.trim();
        let revision = row.revision.trim().to_owned();
        let entry = updated
            .items
            .entry(row.id.clone())
            .or_insert_with(|| SidecarItem {
                status: StoredStatus::Recorded,
                revision: Some(revision.clone()),
                note: None,
                takes: Vec::new(),
            });
        if entry
            .takes
            .iter()
            .any(|existing| existing.reference == take)
        {
            already_present += 1;
            continue;
        }
        let optional =
            |value: &str| Some(value.trim().to_owned()).filter(|value| !value.is_empty());
        entry.takes.push(Take {
            reference: take.to_owned(),
            revision: revision.clone(),
            handoff: optional(&row.handoff),
            by: optional(&row.by),
            date: optional(&row.date),
        });
        // A new take needs fresh approval at its revision.
        entry.status = StoredStatus::Recorded;
        entry.revision = Some(revision);
        if let Some(note) = optional(&row.note) {
            entry.note = Some(note);
        }
        added.push((row.id.clone(), take.to_owned()));
    }
    Ok(ImportPlan {
        sidecar: updated,
        added,
        already_present,
    })
}
