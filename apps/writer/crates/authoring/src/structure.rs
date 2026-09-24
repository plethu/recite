//! Structured insertions share the compiler's stable-ID planner and one undo boundary.
use crate::{Document, EditError, projection::offset};
use recite_core::SourcePosition;

impl Document {
    pub fn set_continuation(
        &mut self,
        expected: i64,
        block: &str,
        destination: &str,
    ) -> Result<(), EditError> {
        self.check_revision(expected)?;
        self.script()?;
        let snapshot = self.kernel().snapshot();
        let blocks = snapshot
            .document(self.key())
            .ok_or(EditError::Position)?
            .summary()
            .blocks();
        let index = blocks
            .iter()
            .position(|b| b.id().as_str() == block)
            .ok_or(EditError::Destination)?;
        let start = blocks[index].span().start.line();
        let end = blocks
            .get(index + 1)
            .map(|b| b.span().start.line())
            .unwrap_or(u32::MAX);
        let parsed = recite_parser::parse(self.key().as_str(), self.source());
        let line = parsed
            .syntax()
            .children()
            .enumerate()
            .find_map(|(index, node)| {
                let line = u32::try_from(index + 1).ok()?;
                (line > start
                    && line < end
                    && node.kind() == recite_parser::ReciteSyntaxKind::Divert
                    && node.text().to_string().starts_with("->"))
                .then_some(line)
            })
            .ok_or(EditError::SourceRequired(
                "no unconditional continuation in this beat",
            ))?;
        self.replace_target(expected, line, destination)
    }

    pub fn rename_beat(&mut self, expected: i64, block: &str, name: &str) -> Result<(), EditError> {
        self.check_revision(expected)?;
        let snapshot = self.kernel().snapshot();
        let declaration = snapshot
            .document(self.key())
            .ok_or(EditError::Position)?
            .summary()
            .blocks()
            .iter()
            .find(|b| b.id().as_str() == block)
            .ok_or(EditError::Destination)?;
        let position = SourcePosition::new(declaration.span().start.line(), 4)?;
        let plan = snapshot.plan_rename_block(self.key(), position, name)?;
        plan.validate(snapshot)?;
        if plan
            .edits()
            .iter()
            .any(|edit| edit.document() != self.key())
        {
            return Err(EditError::SourceRequired(
                "this rename also changes another scene",
            ));
        }
        let mut next = self.source().to_owned();
        for edit in plan.edits().iter().rev() {
            let start = offset(&next, edit.range().start())?;
            let end = offset(&next, edit.range().end())?;
            next.replace_range(start..end, edit.replacement());
        }
        self.replace_source(expected, next)
    }

    pub fn add_beat(&mut self, expected: i64) -> Result<String, EditError> {
        self.check_revision(expected)?;
        self.script()?;
        let snapshot = self.kernel().snapshot();
        let mut number = 1;
        let id = loop {
            let candidate = format!("new_beat_{number}");
            if !snapshot.documents().iter().any(|document| {
                document
                    .summary()
                    .blocks()
                    .iter()
                    .any(|block| block.id().as_str() == candidate)
            }) {
                break candidate;
            }
            number += 1;
        };
        let newline = if self.source().contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let mut next = format!("{}{newline}{newline}:: {id}{newline}", self.source());
        let insertion = next.len();
        next.push_str(&format!(
            ">{newline}  New dialogue.{newline}-> END{newline}"
        ));
        let (next, _) = self.freeze_inserted_passage(next, insertion)?;
        self.replace_source(expected, next)?;
        Ok(id)
    }

    /// Inserts before the beat's first top-level choice or divert, or at its end.
    pub fn add_line(&mut self, expected: i64, section: &str) -> Result<String, EditError> {
        self.check_revision(expected)?;
        self.passages()?;
        let snapshot = self.kernel().snapshot();
        let blocks = snapshot
            .document(self.key())
            .ok_or(EditError::Position)?
            .summary()
            .blocks();
        let index = blocks
            .iter()
            .position(|b| b.id().as_str() == section)
            .ok_or(EditError::Destination)?;
        let start = offset(self.source(), blocks[index].span().start)?;
        let end = blocks
            .get(index + 1)
            .map(|b| offset(self.source(), b.span().start))
            .transpose()?
            .unwrap_or(self.source().len());
        let syntax = recite_parser::parse(crate::DOCUMENT_NAME, self.source());
        let insertion = syntax
            .syntax()
            .children()
            .find_map(|node| {
                let at = usize::from(node.text_range().start());
                let raw = node.text().to_string();
                (at > start
                    && at < end
                    && matches!(
                        node.kind(),
                        recite_parser::ReciteSyntaxKind::Choice
                            | recite_parser::ReciteSyntaxKind::Divert
                            | recite_parser::ReciteSyntaxKind::If
                            | recite_parser::ReciteSyntaxKind::Match
                    )
                    && !raw.starts_with(char::is_whitespace))
                .then_some(at)
            })
            .unwrap_or(end);
        let newline = if self.source().contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let prefix = if self.source()[..insertion].ends_with('\n') {
            ""
        } else {
            newline
        };
        let mut next = self.source().to_owned();
        next.insert_str(
            insertion,
            &format!("{prefix}>{newline}  New dialogue.{newline}{newline}"),
        );
        let (next, id) = self.freeze_inserted_passage(next, insertion + prefix.len())?;
        self.replace_source(expected, next)?;
        Ok(id)
    }

    pub(crate) fn freeze_inserted_passage(
        &self,
        mut next: String,
        insertion: usize,
    ) -> Result<(String, String), EditError> {
        let line = u32::try_from(
            next.get(..insertion)
                .ok_or(EditError::Position)?
                .bytes()
                .filter(|b| *b == b'\n')
                .count()
                + 1,
        )
        .map_err(|_| EditError::Position)?;
        let candidate = Self::in_project(self.key().clone(), next.clone(), self.project_context())?;
        let plan = candidate
            .kernel()
            .snapshot()
            .plan_insert_missing_id(candidate.key(), SourcePosition::new(line, 1)?)?;
        plan.validate(candidate.kernel().snapshot())?;
        for edit in plan.edits().iter().rev() {
            let start = offset(&next, edit.range().start())?;
            let end = offset(&next, edit.range().end())?;
            next.replace_range(start..end, edit.replacement());
        }
        let added = crate::projection::passages(&next)?
            .into_iter()
            .find(|passage| passage.header_line == line)
            .ok_or(EditError::MissingPassage)?;
        Ok((next, added.id))
    }
}
