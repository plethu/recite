//! Frozen identity shape is local; collisions belong to project validation.
use super::{incremental::ValidationPhase, state::Validator};
use crate::diagnostics;
use recite_core::{
    SourceId, SourceSpan,
    ast::{Choice, Line},
};
use std::collections::btree_map::Entry;

impl<'a> Validator<'a> {
    pub(crate) fn validate_line_localisable_id(&mut self, line: &'a Line) {
        let SourceId::Frozen { .. } = &line.source_id else {
            if self.phase == ValidationPhase::Project {
                return;
            }
            self.diagnostics.push(match &line.source_id {
                SourceId::Missing => diagnostics::missing_line_id(line),
                SourceId::Draft { .. } => diagnostics::draft_line_id(line),
                SourceId::Malformed { .. } => diagnostics::malformed_line_id(line, &line.source_id),
                SourceId::Frozen { .. } => unreachable!("frozen ID matched earlier"),
            });
            return;
        };
        if self.phase == ValidationPhase::Local {
            return;
        }
        let Some(id) = line.id.as_ref() else {
            return;
        };

        if let Some(first_span) = self.record_localisable_id(id.as_str(), &line.span) {
            self.diagnostics
                .push(diagnostics::duplicate_line_id(line, id, first_span.clone()));
        }
    }
    pub(crate) fn validate_choice_localisable_id(&mut self, choice: &'a Choice) {
        if let SourceId::Frozen { .. } = &choice.source_id {
            if self.phase == ValidationPhase::Local {
                return;
            }
            let Some(id) = choice.id.as_ref() else {
                return;
            };
            if let Some(first_span) = self.record_localisable_id(id.as_str(), &choice.span) {
                self.diagnostics.push(diagnostics::duplicate_choice_id(
                    choice,
                    id,
                    first_span.clone(),
                ));
            }
        } else if self.phase != ValidationPhase::Project {
            self.diagnostics.push(match &choice.source_id {
                SourceId::Missing => diagnostics::missing_choice_id(choice),
                SourceId::Draft { .. } => diagnostics::draft_choice_id(choice),
                SourceId::Malformed { .. } => {
                    diagnostics::malformed_choice_id(choice, &choice.source_id)
                }
                SourceId::Frozen { .. } => unreachable!("frozen ID matched earlier"),
            });
        }
    }

    fn record_localisable_id(
        &mut self,
        id: &'a str,
        span: &'a SourceSpan,
    ) -> Option<&'a SourceSpan> {
        match self.localisable_ids.entry(id) {
            Entry::Occupied(mut entry) => {
                let first = *entry.get();
                if self.project_complete || first.file == span.file {
                    Some(first)
                } else {
                    entry.insert(span);
                    None
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(span);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests;
