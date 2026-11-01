//! Wording and plural guidance derive from one PO entry and its current form count.
use super::{Catalogue, MsgId, wording};
use recite_core::po::{PoDocument, PoEntryId, evaluate_plural_form, validate_plural_rule};
use std::sync::{Arc, Weak};

pub struct Wording {
    pub entry: PoEntryId,
    pub context: String,
    pub caption: String,
}
pub struct EntryDetails {
    pub wordings: Vec<Wording>,
    pub plural: PluralGuide,
}
pub enum PluralGuide {
    Singular,
    Valid {
        rule: String,
        examples: Vec<Vec<i64>>,
    },
    Invalid {
        rule: Option<String>,
    },
}
impl PluralGuide {
    pub fn valid(&self) -> bool {
        !matches!(self, Self::Invalid { .. })
    }
    pub fn is_plural(&self) -> bool {
        !matches!(self, Self::Singular)
    }
    pub fn rule(&self) -> Option<&str> {
        match self {
            Self::Singular => None,
            Self::Valid { rule, .. } => Some(rule),
            Self::Invalid { rule } => rule.as_deref(),
        }
    }
    pub fn form_caption(&self, index: usize) -> String {
        match self {
            Self::Singular => wording(MsgId::WriterTranslation),
            Self::Valid { examples, .. } => {
                let examples = examples
                    .get(index)
                    .into_iter()
                    .flatten()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                let caption = format!("{} {}", wording(MsgId::WriterPluralForm), index + 1);
                if examples.is_empty() {
                    caption
                } else {
                    format!("{caption} · {examples}")
                }
            }
            Self::Invalid { .. } => format!("{} {}", wording(MsgId::WriterPluralForm), index + 1),
        }
    }
    fn prepare(rule: Option<&str>, forms: usize) -> Self {
        let Some(rule) = rule.filter(|rule| validate_plural_rule(rule).ok() == Some(forms)) else {
            return Self::Invalid {
                rule: rule.map(str::to_owned),
            };
        };
        let mut examples = vec![Vec::new(); forms];
        // One bounded walk supplies every arm; large rules do not multiply evaluation work.
        for count in 0..=200 {
            let Ok(arm) = evaluate_plural_form(rule, count) else {
                return Self::Invalid {
                    rule: Some(rule.into()),
                };
            };
            if let Some(values) = examples.get_mut(arm)
                && values.len() < 5
            {
                values.push(count);
            }
        }
        Self::Valid {
            rule: rule.into(),
            examples,
        }
    }
}

pub(super) struct Cache {
    document: Weak<PoDocument>,
    entry: PoEntryId,
    forms: usize,
    details: Arc<EntryDetails>,
}
impl Catalogue {
    /// Typing and review changes reuse this immutable projection. PO adoption or
    /// a changed arm count replaces it; callers can retain older snapshots safely.
    pub fn entry_details(&self, id: PoEntryId) -> Option<Arc<EntryDetails>> {
        let entry = self.document.entry(id)?;
        if entry.is_header() || entry.is_obsolete() {
            return None;
        }
        let forms = self.drafts.get(&id).map_or_else(
            || {
                if entry.is_plural() {
                    entry.plural_translations().len()
                } else {
                    1
                }
            },
            |draft| draft.forms.len(),
        );
        let mut cache = self.entry_cache.borrow_mut();
        if let Some(cache) = cache.as_ref()
            && cache.entry == id
            && cache.forms == forms
            && Weak::ptr_eq(&cache.document, &Arc::downgrade(&self.document))
        {
            return Some(cache.details.clone());
        }
        let root = entry
            .context()
            .and_then(|context| context.split('&').next());
        let wordings = self
            .document
            .entries()
            .iter()
            .filter(|entry| {
                !entry.is_obsolete()
                    && !entry.is_header()
                    && root.is_some()
                    && entry
                        .context()
                        .and_then(|context| context.split('&').next())
                        == root
            })
            .map(|entry| Wording {
                entry: entry.id(),
                context: entry.context().unwrap_or_default().into(),
                caption: entry
                    .variant()
                    .map_or_else(|| wording(MsgId::WriterDefaultWording), str::to_owned),
            })
            .collect();
        let details = Arc::new(EntryDetails {
            wordings,
            plural: if entry.is_plural() {
                PluralGuide::prepare(self.plural_rule(), forms)
            } else {
                PluralGuide::Singular
            },
        });
        *cache = Some(Cache {
            document: Arc::downgrade(&self.document),
            entry: id,
            forms,
            details: details.clone(),
        });
        Some(details)
    }
}

#[cfg(test)]
mod tests;
