//! Frozen identity shape is local; collisions belong to project validation.
use super::{incremental::ValidationPhase, state::Validator};
use crate::diagnostics;
use recite_core::{Choice, Line, SourceId};

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

        if let Some(first_span) = self.localisable_ids.get(id.as_str())
            && self.duplicate_id_is_in_scope(first_span, &line.span)
        {
            self.diagnostics
                .push(diagnostics::duplicate_line_id(line, id, first_span.clone()));
        } else {
            self.localisable_ids.insert(id.as_str(), line.span.clone());
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
            if let Some(first_span) = self.localisable_ids.get(id.as_str())
                && self.duplicate_id_is_in_scope(first_span, &choice.span)
            {
                self.diagnostics.push(diagnostics::duplicate_choice_id(
                    choice,
                    id,
                    first_span.clone(),
                ));
            } else {
                self.localisable_ids
                    .insert(id.as_str(), choice.span.clone());
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

    fn duplicate_id_is_in_scope(
        &self,
        first_span: &recite_core::SourceSpan,
        current_span: &recite_core::SourceSpan,
    ) -> bool {
        self.project_complete || first_span.file == current_span.file
    }
}
