//! Save requests capture immutable input; acceptance rebases newer drafts.
use super::*;
use crate::recovery::worker::Durability;
use recite_core::po::{PoEdit, PoUpdate};

pub(crate) type CatalogueDurability = Durability<recovery::SnapshotData>;

pub(crate) struct CatalogueSave {
    path: PathBuf,
    document: Arc<PoDocument>,
    baseline: PoDocumentFingerprint,
    drafts: BTreeMap<PoEntryId, Arc<Draft>>,
    retired_forms: Vec<(PoEntryId, Vec<String>)>,
}
pub(crate) struct CatalogueSaved {
    forms: Vec<(PoEntryId, Vec<String>)>,
    document: Arc<PoDocument>,
    baseline: PoDocumentFingerprint,
}
impl CatalogueSave {
    pub(crate) fn execute(&self) -> Result<CatalogueSaved, String> {
        if self.drafts.is_empty() {
            return Ok(CatalogueSaved {
                forms: Vec::new(),
                document: self.document.clone(),
                baseline: self.baseline.clone(),
            });
        }
        let mut candidate = self.document.as_ref().clone();
        let mut updates = Vec::new();
        for (&id, draft) in &self.drafts {
            draft_updates(&self.document, id, draft, &mut updates)?;
        }
        candidate
            .apply_updates(updates)
            .map_err(|e| e.to_string())?;
        crate::project::read_regular(&self.path).map_err(|e| e.to_string())?;
        let baseline = candidate
            .write_atomically(&self.path, &self.baseline)
            .map_err(|error| match error {
                recite_core::po::PoWriteError::Conflict { .. } =>
                    "The translation file changed on disk. Choose “Compare external changes” in the catalogue menu before saving again".to_owned(),
                other => other.to_string(),
            })?;
        let forms = self
            .drafts
            .keys()
            .filter_map(|&id| {
                candidate.entry(id).map(|entry| {
                    let forms = entry
                        .translation()
                        .into_iter()
                        .chain(entry.plural_translations().iter().map(|arm| arm.text()))
                        .map(str::to_lowercase)
                        .collect();
                    (id, forms)
                })
            })
            .collect();
        Ok(CatalogueSaved {
            forms,
            document: Arc::new(candidate),
            baseline,
        })
    }
}
impl Catalogue {
    pub(crate) fn require_idle_save(&self) -> Result<(), String> {
        if self.saving {
            Err("A catalogue save is still in progress.".into())
        } else {
            Ok(())
        }
    }
    pub(crate) fn begin_save(
        &mut self,
        ids: Option<&[PoEntryId]>,
    ) -> Result<CatalogueSave, String> {
        self.require_idle_save()?;
        let drafts: BTreeMap<_, _> = self
            .drafts
            .iter()
            .filter(|(id, _)| ids.is_none_or(|ids| ids.contains(id)))
            .map(|(&id, draft)| (id, draft.clone()))
            .collect();
        self.saving = true;
        Ok(CatalogueSave {
            path: self.path.clone(),
            document: self.document.clone(),
            baseline: self.baseline.clone(),
            drafts,
            retired_forms: Vec::new(),
        })
    }
    pub(crate) fn accept_save(
        &mut self,
        request: &mut CatalogueSave,
        mut saved: CatalogueSaved,
    ) -> Result<Option<CatalogueDurability>, String> {
        if self.path != request.path
            || !Arc::ptr_eq(&self.document, &request.document)
            || self.baseline != request.baseline
        {
            return Err("The saved catalogue is no longer open.".into());
        }
        // A missing draft may mean the user typed the OLD saved text again.
        // Preserve its effective value before installing the submitted baseline.
        let newer: Vec<_> = request
            .drafts
            .iter()
            .map(|(&id, submitted)| {
                let draft = match self.drafts.get(&id) {
                    Some(current) if Arc::ptr_eq(current, submitted) => None,
                    Some(current) => Some(current.clone()),
                    None => self.saved(id).map(Arc::new),
                };
                (id, draft)
            })
            .collect();
        if let Some(cache) = self.queue_cache.get_mut().as_mut() {
            cache.index.borrow_mut().replace_forms(&mut saved.forms);
        }
        // Queue may never have been opened. Its unused normalized forms still
        // belong to the worker receipt, just like displaced cached forms.
        request.retired_forms = saved.forms;
        self.document = saved.document;
        self.baseline = saved.baseline;
        self.revision = Arc::new(());
        for (id, draft) in newer {
            self.drafts.remove(&id);
            if let Some(draft) = draft
                && self.saved(id).as_ref() != Some(draft.as_ref())
            {
                self.drafts.insert(id, draft);
            }
        }
        let snapshot = self.recovery_snapshot();
        let ticket = self
            .recovery
            .as_mut()
            .map(|store| store.begin_persist(snapshot))
            .transpose()
            .map_err(|e| e.to_string())?;
        self.recovery_failure = None;
        Ok(ticket)
    }
    pub(crate) fn finish_save(&mut self) {
        self.saving = false;
    }
}
fn draft_updates(
    document: &PoDocument,
    id: PoEntryId,
    draft: &Draft,
    updates: &mut Vec<PoUpdate>,
) -> Result<(), String> {
    let entry = document
        .entry(id)
        .ok_or_else(|| wording(MsgId::WriterNoEntry))?;
    if draft.reviewed && draft.forms.iter().any(|text| text.trim().is_empty()) {
        return Err(wording(MsgId::WriterEmptyReview));
    }
    if entry.is_plural() {
        let count = document
            .headers()
            .iter()
            .find(|h| h.key().eq_ignore_ascii_case("Plural-Forms"))
            .map(|h| h.value())
            .and_then(|rule| recite_core::po::validate_plural_rule(rule).ok())
            .ok_or_else(|| wording(MsgId::WriterEntryPluralInvalid))?;
        if draft.forms.len() != count {
            return Err(wording(MsgId::WriterEntryPluralInvalid));
        }
        updates.extend(
            draft.forms.iter().enumerate().map(|(index, value)| {
                PoUpdate::Field(PoEdit::plural_translation(id, index, value))
            }),
        );
    } else {
        let [value] = draft.forms.as_slice() else {
            return Err(wording(MsgId::WriterNoEntry));
        };
        updates.push(PoUpdate::Field(PoEdit::translation(id, value)));
    }
    updates.push(PoUpdate::Fuzzy {
        entry: id,
        fuzzy: !draft.reviewed,
    });
    Ok(())
}

#[cfg(test)]
mod tests;
