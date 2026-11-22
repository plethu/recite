//! Applying a prose finding's replacement or adding its word to the project dictionary.
//!
//! A replacement splices the checked source at the finding's byte range, which `recite-prose`
//! keeps inside one verbatim prose piece, and is accepted through full-source replacement so it
//! is one undoable step that never changes Recite syntax.
use super::CheckGeneration;
use crate::{Phase, Writer, feedback};
use recite_compiler::authoring::CancellationToken;
use recite_writer_model::SourceValidation;
use recite_writer_session::messages::{MsgId, format, text};

impl Writer {
    /// Replace finding `finding` of check `generation` with its fix `fix`.
    pub fn apply_prose_fix(&mut self, generation: CheckGeneration, finding: usize, fix: usize) {
        let result = self
            .prose_fix_source(generation, finding, fix)
            .and_then(|source| self.replace_prose(source));
        self.status = match result {
            Ok(()) => text(MsgId::WriterProseApplied),
            Err(message) => message,
        };
    }

    /// Append the word of finding `finding` of check `generation` to the project dictionary.
    pub fn add_prose_word(&mut self, generation: CheckGeneration, finding: usize) {
        let Some((configuration, word)) = self
            .current_prose()
            .filter(|checked| checked.generation == generation)
            .and_then(|checked| {
                let finding = checked.outcome.finding(finding)?;
                Some((checked.configuration.clone(), finding.text.clone()))
            })
        else {
            self.status = text(MsgId::WriterProseStale);
            return;
        };
        self.status = match configuration.add_word(&word) {
            Ok(true) => {
                self.recheck_prose();
                format(MsgId::WriterProseWordAdded, [("word", word.as_str())])
            }
            Ok(false) => format(MsgId::WriterProseWordListed, [("word", word.as_str())]),
            Err(error) => format(
                MsgId::WriterProseDictionaryFailed,
                [("detail", error.to_string().as_str())],
            ),
        };
    }

    /// The current source with the fix applied, refused once the source has changed.
    fn prose_fix_source(
        &self,
        generation: CheckGeneration,
        finding: usize,
        fix: usize,
    ) -> Result<String, String> {
        if self.phase != Phase::Writing
            || self.navigation.busy()
            || self.work.busy()
            || self.session.read_only()
        {
            return Err(text(MsgId::WriterProseBusy));
        }
        if self.session.model().has_draft() {
            return Err(text(MsgId::WriterProseDraftPending));
        }
        let stale = || text(MsgId::WriterProseStale);
        let checked = self
            .current_prose()
            .filter(|checked| checked.generation == generation)
            .ok_or_else(stale)?;
        let edit = checked
            .outcome
            .finding(finding)
            .and_then(|finding| finding.fixes.get(fix))
            .ok_or_else(stale)?;
        let source = checked.source.as_ref();
        let (Some(before), Some(after)) =
            (source.get(..edit.range.start), source.get(edit.range.end..))
        else {
            return Err(stale());
        };
        let mut replaced =
            String::with_capacity(before.len() + edit.replacement.len() + after.len());
        replaced.push_str(before);
        replaced.push_str(&edit.replacement);
        replaced.push_str(after);
        Ok(replaced)
    }

    fn replace_prose(&mut self, source: String) -> Result<(), String> {
        self.session
            .edit(|model| {
                let mut prepared = model
                    .source_replacement_request(SourceValidation::NoNewErrors)
                    .prepare(source.into(), &CancellationToken::new())?;
                model.accept_source_replacement(&mut prepared)
            })
            .map_err(|error| feedback::file(&error))?;
        self.retire_rules();
        self.reading.prepare(self.session.model());
        self.invalidate_map();
        Ok(())
    }
}
