//! Insertion beside an existing passage (a line after a line, a reply after a reply, each with a
//! frozen stable ID and one undo step), and the speakers a line may take.
use crate::{Document, DocumentVersion, EditError, PassageKind};
use recite_core::StatementHeader;

impl Document {
    /// The speakers a line may take: the project's declared speakers, then any this scene uses
    /// that are not declared, each once in order.
    pub fn speakers(&self) -> Result<Vec<String>, EditError> {
        let mut speakers: Vec<String> = self
            .schema()
            .map(|schema| schema.speakers.keys().cloned().collect())
            .unwrap_or_default();
        for passage in self.passage_snapshot()?.iter() {
            if let PassageKind::Dialogue {
                speaker: Some(speaker),
            } = &passage.kind
                && !speakers.contains(speaker)
            {
                speakers.push(speaker.clone());
            }
        }
        Ok(speakers)
    }
    /// Inserts a passage of the same kind directly after `passage` and returns its new ID: a
    /// line after a line, or a reply after a reply that continues to END until rewired.
    pub fn insert_after(
        &mut self,
        expected: DocumentVersion,
        passage: &str,
    ) -> Result<String, EditError> {
        self.check_revision(expected)?;
        let target = self
            .find_passage(passage)?
            .ok_or(EditError::MissingPassage)?;
        let source = self.source();
        // The passage's last source line: its text, or a reply's own destination below it.
        let last_line = target.target_line.unwrap_or(0);
        let mut insertion = source
            .get(target.text_range.end..)
            .and_then(|rest| rest.find('\n'))
            .map_or(source.len(), |newline| target.text_range.end + newline + 1);
        if let Some(line) = usize::try_from(last_line).ok().filter(|line| *line > 0) {
            insertion = insertion.max(line_end(source, line));
        }
        let header_start = line_start(source, usize::try_from(target.header_line).unwrap_or(1));
        let indent: String = source[header_start..]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let newline = if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let prefix = if source[..insertion].ends_with('\n') {
            ""
        } else {
            newline
        };
        let (gap, addition) = match target.kind {
            PassageKind::Dialogue { .. } => (
                newline,
                format!(
                    "{indent}{}{newline}{indent}  New dialogue.{newline}",
                    StatementHeader::line()
                ),
            ),
            PassageKind::Choice { .. } => (
                "",
                format!(
                    "{indent}{}{newline}{indent}  New choice.{newline}{indent}  -> END{newline}",
                    StatementHeader::choice()
                ),
            ),
        };
        let mut next = source.to_owned();
        next.insert_str(insertion, &format!("{prefix}{gap}{addition}"));
        let (next, added) =
            self.freeze_inserted_passage(next, insertion + prefix.len() + gap.len())?;
        self.replace_source(expected, next)?;
        Ok(added)
    }
}

/// The byte where 1-based `line` starts, or the end of the source.
fn line_start(source: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    source
        .match_indices('\n')
        .nth(line - 2)
        .map_or(source.len(), |(at, _)| at + 1)
}
/// The byte just past 1-based `line`'s newline, or the end of the source.
fn line_end(source: &str, line: usize) -> usize {
    line_start(source, line + 1)
}
