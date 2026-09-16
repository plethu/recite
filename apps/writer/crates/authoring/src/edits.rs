use recite_core::SpeakerId;
use recite_parser::{ReciteSyntaxKind, ReciteSyntaxNode, metadata_assignments, parse};

use crate::{DOCUMENT_NAME, Document, EditError, Passage, PassageKind, projection::offset};

impl Document {
    pub fn replace_text(&mut self, expected: i64, id: &str, text: &str) -> Result<(), EditError> {
        self.check_revision(expected)?;
        let passage = self.passage(id)?;
        let newline = if self.source().contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let replacement = text
            .split('\n')
            .collect::<Vec<_>>()
            .join(&format!("{newline}{}", passage.indentation));
        let mut next = self.source().to_owned();
        next.replace_range(passage.text_range, &replacement);
        // The parser, not a GUI-side grammar, decides whether the field is still prose.
        let projected = crate::projection::passages(&next).map_err(|_| EditError::NotProse)?;
        if !projected
            .iter()
            .any(|item| item.id == id && item.text == text)
        {
            return Err(EditError::NotProse);
        }
        if projected.len() != self.passages()?.len() {
            return Err(EditError::NotProse);
        }
        self.replace_source(expected, next)
    }

    pub fn set_speaker(&mut self, expected: i64, id: &str, speaker: &str) -> Result<(), EditError> {
        self.check_revision(expected)?;
        let passage = self.passage(id)?;
        if !matches!(passage.kind, PassageKind::Dialogue { .. }) {
            return Err(EditError::MissingPassage);
        }
        SpeakerId::new(speaker)?;
        if !recite_parser::is_metadata_symbol(speaker) {
            return Err(EditError::SourceRequired("speaker must be a schema symbol"));
        }
        let row = row(self.source(), passage.header_line)?;
        let raw = row.text().to_string();
        let start = usize::from(row.text_range().start());
        let mut next = self.source().to_owned();
        if let Some(assignment) = metadata_assignments(raw.trim_end_matches(['\r', '\n']))
            .into_iter()
            .rfind(|a| a.key == "speaker")
        {
            next.replace_range(
                start + assignment.value_start..start + assignment.end,
                speaker,
            );
        } else {
            let end = start + raw.trim_end_matches(['\r', '\n']).len();
            next.insert_str(end, &format!(" speaker={speaker}"));
        }
        self.replace_source(expected, next)
    }

    pub fn set_destination(
        &mut self,
        expected: i64,
        id: &str,
        destination: &str,
    ) -> Result<(), EditError> {
        self.check_revision(expected)?;
        if destination != "END" && !self.sections().iter().any(|section| section == destination) {
            return Err(EditError::Destination);
        }
        let passage = self.passage(id)?;
        if !matches!(passage.kind, PassageKind::Choice { .. }) {
            return Err(EditError::MissingPassage);
        }
        let line = passage
            .target_line
            .ok_or(EditError::SourceRequired("a choice without a target"))?;
        self.replace_target(expected, line, destination)
    }

    pub(crate) fn replace_target(
        &mut self,
        expected: i64,
        line: u32,
        destination: &str,
    ) -> Result<(), EditError> {
        if destination != "END" && !self.sections().iter().any(|section| section == destination) {
            return Err(EditError::Destination);
        }
        let target_row = row(self.source(), line)?;
        let token = target_row
            .children_with_tokens()
            .filter_map(|item| item.into_token())
            .find(|token| token.kind() == ReciteSyntaxKind::Ident)
            .ok_or(EditError::Position)?;
        let mut next = self.source().to_owned();
        next.replace_range(
            usize::from(token.text_range().start())..usize::from(token.text_range().end()),
            destination,
        );
        self.replace_source(expected, next)
    }

    /// Adds one choice to a section; the existing compiler planner freezes its ID.
    pub fn add_choice(&mut self, expected: i64, section: &str) -> Result<String, EditError> {
        self.check_revision(expected)?;
        self.passages()?;
        let blocks = self
            .kernel()
            .snapshot()
            .document(self.key())
            .ok_or(EditError::Position)?
            .summary()
            .blocks();
        let index = blocks
            .iter()
            .position(|block| block.id().as_str() == section)
            .ok_or(EditError::Destination)?;
        let mut insertion = if let Some(next) = blocks.get(index + 1) {
            offset(self.source(), next.span().start)?
        } else {
            self.source().len()
        };
        // A section ending in an unconditional divert cannot reach an appended choice.
        let start = offset(
            self.source(),
            blocks.get(index).ok_or(EditError::Position)?.span().start,
        )?;
        let section_source = self
            .source()
            .get(start..insertion)
            .ok_or(EditError::Position)?;
        let divert = parse(DOCUMENT_NAME, section_source)
            .syntax()
            .children()
            .find(|node| {
                node.kind() == ReciteSyntaxKind::Divert && node.text().to_string().starts_with("->")
            });
        if let Some(divert) = &divert {
            insertion = start + usize::from(divert.text_range().start());
        }
        let newline = if self.source().contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let prefix = if insertion > 0
            && self
                .source()
                .get(..insertion)
                .is_some_and(|s| !s.ends_with('\n'))
        {
            newline
        } else {
            ""
        };
        // An existing continuation becomes the new reply's target, preserving its
        // destination and comment rather than leaving an unreachable appended choice.
        let addition = if divert.is_some() {
            format!("{prefix}?{newline}  New choice.{newline}  ")
        } else {
            format!("{prefix}?{newline}  New choice.{newline}  -> END{newline}{newline}")
        };
        let mut next = self.source().to_owned();
        next.insert_str(insertion, &addition);
        let (next, added) = self.freeze_inserted_passage(next, insertion + prefix.len())?;
        self.replace_source(expected, next)?;
        Ok(added)
    }

    fn passage(&self, id: &str) -> Result<Passage, EditError> {
        self.passages()?
            .into_iter()
            .find(|passage| passage.id == id)
            .ok_or(EditError::MissingPassage)
    }
}

fn row(source: &str, line: u32) -> Result<ReciteSyntaxNode, EditError> {
    let index = usize::try_from(line - 1).map_err(|_| EditError::Position)?;
    parse(DOCUMENT_NAME, source)
        .syntax()
        .children()
        .nth(index)
        .ok_or(EditError::Position)
}
