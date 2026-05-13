use recite_core::{
    Block, BlockId, BlockReference, Choice, ChoiceEcho, ChoiceId, Comment, ConditionExpression,
    Diagnostic, Divert, DivertTarget, Effect, EffectMode, IfBranch, Line, LineId, MatchArm,
    MatchBranch, MatchPattern, Metadata, MetadataEntry, SourceFile, SourceSpan, SourceText,
    SpeakerId, Statement,
};

use crate::body::{BodyBoundary, BodyCursor, BodyStep};
use crate::condition::{parse_condition_call, parse_condition_expression};
use crate::diagnostics::{
    empty_block_id, expected_statement_or_prose, malformed_case, malformed_condition,
    malformed_divert_target, malformed_effect, malformed_header, misplaced_case, misplaced_else,
    missing_block_id, missing_choice_id, missing_divert_target, missing_line_id,
    prose_after_nested_statement, statement_before_block,
};
use crate::header::{HeaderField, HeaderKeyValue, fields_after_prefix, rest_after_prefix};
use crate::layout::{ClassifiedLine, classify_line};
use crate::markers::StatementMarker;
use crate::source::{LogicalLine, LogicalLines, span_for_line, span_for_text};

#[derive(Clone, Debug, PartialEq)]
pub struct LoweredSourceFile {
    pub source_file: SourceFile,
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) fn lower_source_file(
    path: &str,
    source: &str,
    parse_diagnostics: &[Diagnostic],
) -> LoweredSourceFile {
    let mut diagnostics = parse_diagnostics.to_vec();
    let blocks = Lowerer::new(path, source, &mut diagnostics).lower_blocks();

    LoweredSourceFile {
        source_file: SourceFile::new(path, blocks),
        diagnostics,
    }
}

struct Lowerer<'source, 'diagnostics> {
    path: &'source str,
    lines: Vec<LogicalLine<'source>>,
    diagnostics: &'diagnostics mut Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
struct LoweredProseBody {
    text: String,
    text_span: SourceSpan,
    statements: Vec<Statement>,
    next_index: usize,
}

#[derive(Clone, Debug)]
struct BlockHeader {
    id: BlockId,
    is_default: bool,
    default_speaker: Option<SpeakerId>,
    metadata: Metadata,
    span: SourceSpan,
}

impl<'source, 'diagnostics> Lowerer<'source, 'diagnostics> {
    fn new(
        path: &'source str,
        source: &'source str,
        diagnostics: &'diagnostics mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            path,
            lines: LogicalLines::new(source).collect(),
            diagnostics,
        }
    }

    fn lower_blocks(&mut self) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut index = 0;

        while index < self.lines.len() {
            let line = self.lines[index];
            let trimmed = line.trimmed_content();

            if classify_line(line) != ClassifiedLine::Statement(StatementMarker::Block) {
                self.report_statement_before_block(line, trimmed);
                index += 1;
                continue;
            }

            let (block, next_index) = self.lower_block(index);
            if let Some(block) = block {
                blocks.push(block);
            }
            index = next_index;
        }

        blocks
    }

    fn lower_block(&mut self, header_index: usize) -> (Option<Block>, usize) {
        let Some(header) = self.lower_block_header(header_index) else {
            return (None, header_index + 1);
        };

        let (statements, next_index) = self.lower_block_statements(header_index);
        let mut block = Block::new(header.id, statements, header.span)
            .with_default(header.is_default)
            .with_metadata(header.metadata);
        if let Some(default_speaker) = header.default_speaker {
            block = block.with_default_speaker(default_speaker);
        }

        (Some(block), next_index)
    }

    fn lower_block_header(&mut self, header_index: usize) -> Option<BlockHeader> {
        let line = self.lines[header_index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let span = span_for_line(self.path, line.number, base_column);
        let fields = header_fields(trimmed, StatementMarker::Block, line, base_column);
        let Some(id_field) = fields.first().copied() else {
            self.diagnostics.push(missing_block_id(span));
            return None;
        };

        if id_field.key_value(self.path).is_some() {
            self.diagnostics
                .push(missing_block_id(id_field.span(self.path)));
            return None;
        }

        let Ok(id) = BlockId::new(id_field.text) else {
            self.diagnostics
                .push(empty_block_id(id_field.span(self.path)));
            return None;
        };

        let mut is_default = false;
        let mut default_speaker = None;
        let mut metadata = Metadata::new();

        for field in fields.iter().skip(1).copied() {
            if field.text == "default" {
                is_default = true;
                continue;
            }

            let Some(kv) = self.valid_key_value(field) else {
                continue;
            };

            if kv.key == "speaker" {
                default_speaker = self.speaker_from_value(&kv);
                continue;
            }

            if let Some(entry) = self.metadata_entry(kv) {
                metadata.push(entry);
            }
        }

        Some(BlockHeader {
            id,
            is_default,
            default_speaker,
            metadata,
            span,
        })
    }

    fn lower_block_statements(&mut self, header_index: usize) -> (Vec<Statement>, usize) {
        let mut statements = Vec::new();
        let mut cursor = BodyCursor::new(&self.lines, header_index, BodyBoundary::NextBlock);

        while cursor.index() < self.lines.len() {
            let line = self.lines[cursor.index()];
            let index = match cursor.step(self.path, line, true, self.diagnostics) {
                BodyStep::Content { index } => index,
                BodyStep::Boundary => break,
                BodyStep::Blank | BodyStep::MixedIndent => continue,
            };

            let (statement, next_index) = self.lower_statement(index);
            if let Some(statement) = statement {
                statements.push(statement);
            }
            cursor.set_index(next_index);
        }

        (statements, cursor.index())
    }

    fn lower_statement(&mut self, index: usize) -> (Option<Statement>, usize) {
        match classify_line(self.lines[index]) {
            ClassifiedLine::Statement(StatementMarker::Line) => {
                let (line, next_index) = self.lower_line(index);
                (Some(Statement::Line(line)), next_index)
            }
            ClassifiedLine::Statement(StatementMarker::Choice) => {
                let (choice, next_index) = self.lower_choice(index);
                (Some(Statement::Choice(choice)), next_index)
            }
            ClassifiedLine::Statement(StatementMarker::Divert) => {
                (self.lower_divert(index).map(Statement::Divert), index + 1)
            }
            ClassifiedLine::Statement(StatementMarker::Effect) => {
                (self.lower_effect(index).map(Statement::Effect), index + 1)
            }
            ClassifiedLine::Statement(StatementMarker::If) => {
                let (branch, next_index) = self.lower_if(index);
                (branch.map(Statement::If), next_index)
            }
            ClassifiedLine::Statement(StatementMarker::Match) => {
                let (branch, next_index) = self.lower_match(index);
                (branch.map(Statement::Match), next_index)
            }
            ClassifiedLine::Statement(StatementMarker::Comment) => (
                Some(Statement::Comment(self.lower_comment(self.lines[index]))),
                index + 1,
            ),
            ClassifiedLine::Statement(StatementMarker::Else) => {
                let line = self.lines[index];
                self.diagnostics.push(misplaced_else(span_for_line(
                    self.path,
                    line.number,
                    line.indent_len() + 1,
                )));
                (None, self.skip_statement_body(index))
            }
            ClassifiedLine::Statement(StatementMarker::Case) => {
                let line = self.lines[index];
                self.diagnostics.push(misplaced_case(span_for_line(
                    self.path,
                    line.number,
                    line.indent_len() + 1,
                )));
                (None, self.skip_statement_body(index))
            }
            ClassifiedLine::Statement(StatementMarker::Block)
            | ClassifiedLine::Blank
            | ClassifiedLine::Prose
            | ClassifiedLine::Error => (None, index + 1),
        }
    }

    fn lower_line(&mut self, line_index: usize) -> (Line, usize) {
        let header = self.lines[line_index];
        let indent = header.indent_len();
        let trimmed = header.trimmed_content();
        let base_column = indent + 1;
        let line_span = span_for_line(self.path, header.number, base_column);
        let fields = header_fields(trimmed, StatementMarker::Line, header, base_column);
        let mut field_start = 0;
        let line_id = if let Some(first) = fields.first().copied() {
            if first.key_value(self.path).is_none() {
                field_start = 1;
                LineId::new(first.text).ok()
            } else {
                None
            }
        } else {
            None
        };

        if line_id.is_none() {
            self.diagnostics.push(missing_line_id(line_span.clone()));
        }

        let (speaker, metadata) = self.lower_speaker_metadata(&fields[field_start..]);
        let body = self.lower_prose_body(line_index, false);
        let mut line = Line::new(
            line_id,
            SourceText::new(body.text, body.text_span),
            line_span,
        )
        .with_metadata(metadata)
        .with_statements(body.statements);
        if let Some(speaker) = speaker {
            line = line.with_speaker(speaker);
        }

        (line, body.next_index)
    }

    fn lower_choice(&mut self, choice_index: usize) -> (Choice, usize) {
        let header = self.lines[choice_index];
        let indent = header.indent_len();
        let trimmed = header.trimmed_content();
        let base_column = indent + 1;
        let choice_span = span_for_line(self.path, header.number, base_column);
        let fields = header_fields(trimmed, StatementMarker::Choice, header, base_column);
        let if_index = fields.iter().position(|field| field.text == "if");
        let header_fields = if let Some(if_index) = if_index {
            &fields[..if_index]
        } else {
            fields.as_slice()
        };

        let mut field_start = 0;
        let choice_id = if let Some(first) = header_fields.first().copied() {
            if first.key_value(self.path).is_none() {
                field_start = 1;
                ChoiceId::new(first.text).ok()
            } else {
                None
            }
        } else {
            None
        };

        if choice_id.is_none() {
            self.diagnostics
                .push(missing_choice_id(choice_span.clone()));
        }

        let (metadata, echo) = self.lower_choice_metadata(&header_fields[field_start..]);
        let condition = if let Some(if_index) = if_index {
            self.lower_choice_condition(trimmed, base_column, fields[if_index])
        } else {
            None
        };
        let body = self.lower_prose_body(choice_index, true);
        let mut target = None;
        let mut statements = Vec::new();
        for statement in body.statements {
            if target.is_none() {
                if let Statement::Divert(divert) = statement {
                    target = Some(divert.target);
                    continue;
                }
            }
            statements.push(statement);
        }

        let mut choice = Choice::new(
            choice_id,
            SourceText::new(body.text, body.text_span),
            choice_span,
        )
        .with_metadata(metadata)
        .with_echo(echo)
        .with_statements(statements);
        if let Some(condition) = condition {
            choice = choice.with_condition(condition);
        }
        if let Some(target) = target {
            choice = choice.with_target(target);
        }

        (choice, body.next_index)
    }

    fn lower_choice_condition(
        &mut self,
        trimmed: &str,
        base_column: usize,
        field: HeaderField<'_>,
    ) -> Option<ConditionExpression> {
        let rest_start = field.offset + field.text.len();
        let rest = &trimmed[rest_start..];
        let whitespace_len = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let condition = &rest[whitespace_len..];
        let column = base_column + trimmed[..rest_start + whitespace_len].chars().count();

        match parse_condition_expression(self.path, field.line, column, condition) {
            Ok(condition) => Some(condition),
            Err(error) => {
                self.diagnostics
                    .push(malformed_condition(error.span, error.message));
                None
            }
        }
    }

    fn lower_divert(&mut self, index: usize) -> Option<Divert> {
        let line = self.lines[index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let fields = header_fields(trimmed, StatementMarker::Divert, line, base_column);
        let span = span_for_line(self.path, line.number, base_column);
        let Some(target) = fields.first().copied() else {
            self.diagnostics.push(missing_divert_target(span));
            return None;
        };

        if fields.len() > 1 {
            self.diagnostics
                .push(malformed_divert_target(fields[1].span(self.path)));
            return None;
        }

        let Some(target) = self.divert_target(target) else {
            return None;
        };

        Some(Divert::new(target, span))
    }

    fn lower_effect(&mut self, index: usize) -> Option<Effect> {
        let line = self.lines[index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let span = span_for_line(self.path, line.number, base_column);
        let fields = header_fields(trimmed, StatementMarker::Effect, line, base_column);
        let Some(mode_field) = fields.first().copied() else {
            self.diagnostics
                .push(malformed_effect(span, "missing effect mode"));
            return None;
        };
        let Some(mode) = effect_mode(mode_field.text) else {
            self.diagnostics.push(malformed_effect(
                mode_field.span(self.path),
                "expected deferred, immediate, or blocking",
            ));
            return None;
        };

        let call_start = mode_field.offset + mode_field.text.len();
        let rest = &trimmed[call_start..];
        let whitespace_len = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let call_text = &rest[whitespace_len..];
        let call_column = base_column + trimmed[..call_start + whitespace_len].chars().count();

        match parse_condition_call(self.path, line.number, call_column, call_text) {
            Ok(call) => Some(Effect::new(mode, call.function, call.args, span)),
            Err(error) => {
                self.diagnostics
                    .push(malformed_effect(error.span, error.message));
                None
            }
        }
    }

    fn lower_if(&mut self, index: usize) -> (Option<IfBranch>, usize) {
        let line = self.lines[index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let rest = rest_after_prefix(trimmed, StatementMarker::If.text(), base_column);
        let condition =
            match parse_condition_expression(self.path, line.number, rest.column, rest.text) {
                Ok(condition) => Some(condition),
                Err(error) => {
                    self.diagnostics
                        .push(malformed_condition(error.span, error.message));
                    None
                }
            };
        let (then_statements, mut next_index) = self.lower_statement_body(index);
        let else_statements = if self.is_else_at(next_index, indent) {
            let (else_body, after_else) = self.lower_else(next_index);
            next_index = after_else;
            else_body
        } else {
            Vec::new()
        };

        let Some(condition) = condition else {
            return (None, next_index);
        };

        let branch = IfBranch::new(
            condition,
            then_statements,
            span_for_line(self.path, line.number, base_column),
        )
        .with_else_statements(else_statements);

        (Some(branch), next_index)
    }

    fn lower_else(&mut self, index: usize) -> (Vec<Statement>, usize) {
        let line = self.lines[index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let rest = rest_after_prefix(trimmed, StatementMarker::Else.text(), base_column);
        if !rest.text.is_empty() {
            self.diagnostics.push(malformed_header(span_for_text(
                self.path,
                line.number,
                rest.column,
                rest.text,
            )));
        }

        self.lower_statement_body(index)
    }

    fn lower_match(&mut self, index: usize) -> (Option<MatchBranch>, usize) {
        let line = self.lines[index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let rest = rest_after_prefix(trimmed, StatementMarker::Match.text(), base_column);
        let scrutinee = match parse_condition_call(self.path, line.number, rest.column, rest.text) {
            Ok(call) => Some(call),
            Err(error) => {
                self.diagnostics
                    .push(malformed_condition(error.span, error.message));
                None
            }
        };
        let (arms, next_index) = self.lower_match_arms(index);

        let Some(scrutinee) = scrutinee else {
            return (None, next_index);
        };

        (
            Some(MatchBranch::new(
                scrutinee,
                arms,
                span_for_line(self.path, line.number, base_column),
            )),
            next_index,
        )
    }

    fn lower_match_arms(&mut self, header_index: usize) -> (Vec<MatchArm>, usize) {
        let mut arms = Vec::new();
        let mut cursor = BodyCursor::new(&self.lines, header_index, BodyBoundary::HeaderIndent);

        while cursor.index() < self.lines.len() {
            let line = self.lines[cursor.index()];
            let index = match cursor.step(self.path, line, true, self.diagnostics) {
                BodyStep::Content { index } => index,
                BodyStep::Boundary => break,
                BodyStep::Blank | BodyStep::MixedIndent => continue,
            };

            if classify_line(line) != ClassifiedLine::Statement(StatementMarker::Case) {
                self.diagnostics.push(malformed_case(span_for_line(
                    self.path,
                    line.number,
                    line.indent_len() + 1,
                )));
                let next_index = if matches!(classify_line(line), ClassifiedLine::Statement(_)) {
                    self.skip_statement_body(index)
                } else {
                    index + 1
                };
                cursor.set_index(next_index);
                continue;
            }

            let (arm, next_index) = self.lower_case(index);
            if let Some(arm) = arm {
                arms.push(arm);
            }
            cursor.set_index(next_index);
        }

        (arms, cursor.index())
    }

    fn lower_case(&mut self, index: usize) -> (Option<MatchArm>, usize) {
        let line = self.lines[index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let fields = header_fields(trimmed, StatementMarker::Case, line, base_column);
        let (statements, next_index) = self.lower_statement_body(index);

        let Some(pattern_field) = fields.first().copied() else {
            self.diagnostics.push(malformed_case(span_for_line(
                self.path,
                line.number,
                base_column,
            )));
            return (None, next_index);
        };

        if fields.len() > 1 {
            self.diagnostics
                .push(malformed_case(fields[1].span(self.path)));
            return (None, next_index);
        }

        let pattern = if pattern_field.text == "_" {
            MatchPattern::Wildcard
        } else {
            MatchPattern::Variant(pattern_field.text.to_owned())
        };

        (
            Some(MatchArm::new(
                pattern,
                statements,
                span_for_line(self.path, line.number, base_column),
            )),
            next_index,
        )
    }

    fn lower_prose_body(
        &mut self,
        header_index: usize,
        emit_mixed_indent: bool,
    ) -> LoweredProseBody {
        let header = self.lines[header_index];
        let header_indent = header.indent_len();
        let mut text_start_line = header.number;
        let mut text_start_column = header_indent + 3;
        let mut text_lines = Vec::new();
        let mut statements = Vec::new();
        let mut saw_statement = false;
        let mut cursor = BodyCursor::new(&self.lines, header_index, BodyBoundary::HeaderIndent);

        while cursor.index() < self.lines.len() {
            let line = self.lines[cursor.index()];
            let index = match cursor.step(self.path, line, emit_mixed_indent, self.diagnostics) {
                BodyStep::Content { index } => index,
                BodyStep::Boundary => break,
                BodyStep::Blank => {
                    if !text_lines.is_empty() && !saw_statement {
                        text_lines.push(String::new());
                    }
                    continue;
                }
                BodyStep::MixedIndent => continue,
            };

            let trimmed = line.trimmed_content();
            let indent = line.indent_len();

            if matches!(classify_line(line), ClassifiedLine::Statement(_)) {
                trim_trailing_blank_lines(&mut text_lines);
                saw_statement = true;
                let (statement, next_index) = self.lower_statement(index);
                if let Some(statement) = statement {
                    statements.push(statement);
                }
                cursor.set_index(next_index);
                continue;
            }

            if saw_statement {
                self.diagnostics
                    .push(prose_after_nested_statement(span_for_line(
                        self.path,
                        line.number,
                        indent + 1,
                    )));
                cursor.advance();
                continue;
            }

            if text_lines.is_empty() {
                text_start_line = line.number;
                text_start_column = indent + 1;
            }
            text_lines.push(trimmed.to_owned());
            cursor.advance();
        }

        trim_trailing_blank_lines(&mut text_lines);

        LoweredProseBody {
            text: text_lines.join("\n"),
            text_span: span_for_line(self.path, text_start_line, text_start_column),
            statements,
            next_index: cursor.index(),
        }
    }

    fn lower_statement_body(&mut self, header_index: usize) -> (Vec<Statement>, usize) {
        let mut statements = Vec::new();
        let mut cursor = BodyCursor::new(&self.lines, header_index, BodyBoundary::HeaderIndent);

        while cursor.index() < self.lines.len() {
            let line = self.lines[cursor.index()];
            let index = match cursor.step(self.path, line, true, self.diagnostics) {
                BodyStep::Content { index } => index,
                BodyStep::Boundary => break,
                BodyStep::Blank | BodyStep::MixedIndent => continue,
            };

            if !matches!(classify_line(line), ClassifiedLine::Statement(_)) {
                self.diagnostics
                    .push(expected_statement_or_prose(span_for_line(
                        self.path,
                        line.number,
                        line.indent_len() + 1,
                    )));
                cursor.advance();
                continue;
            }

            let (statement, next_index) = self.lower_statement(index);
            if let Some(statement) = statement {
                statements.push(statement);
            }
            cursor.set_index(next_index);
        }

        (statements, cursor.index())
    }

    fn lower_comment(&self, line: LogicalLine<'_>) -> Comment {
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let text = trimmed
            .strip_prefix(StatementMarker::Comment.text())
            .expect("comment lowering only receives comment lines")
            .trim_start_matches([' ', '\t']);

        Comment::new(text, span_for_line(self.path, line.number, indent + 1))
    }

    fn lower_speaker_metadata(
        &mut self,
        fields: &[HeaderField<'_>],
    ) -> (Option<SpeakerId>, Metadata) {
        let mut speaker = None;
        let mut metadata = Metadata::new();

        for field in fields.iter().copied() {
            let Some(kv) = self.valid_key_value(field) else {
                continue;
            };

            if kv.key == "speaker" {
                speaker = self.speaker_from_value(&kv);
                continue;
            }

            if let Some(entry) = self.metadata_entry(kv) {
                metadata.push(entry);
            }
        }

        (speaker, metadata)
    }

    fn lower_choice_metadata(&mut self, fields: &[HeaderField<'_>]) -> (Metadata, ChoiceEcho) {
        let mut metadata = Metadata::new();
        let mut echo = ChoiceEcho::None;

        for field in fields.iter().copied() {
            let Some(kv) = self.valid_key_value(field) else {
                continue;
            };

            if kv.key == "echo" {
                if let Some(parsed) = choice_echo(kv.value) {
                    echo = parsed;
                } else {
                    self.diagnostics.push(malformed_header(kv.value_span));
                }
                continue;
            }

            if let Some(entry) = self.metadata_entry(kv) {
                metadata.push(entry);
            }
        }

        (metadata, echo)
    }

    fn valid_key_value<'a>(&mut self, field: HeaderField<'a>) -> Option<HeaderKeyValue<'a>> {
        let Some(kv) = field.key_value(self.path) else {
            self.diagnostics
                .push(malformed_header(field.span(self.path)));
            return None;
        };

        if kv.key.is_empty() || kv.value.is_empty() {
            self.diagnostics
                .push(malformed_header(kv.field_span.clone()));
            return None;
        }

        Some(kv)
    }

    fn speaker_from_value(&mut self, kv: &HeaderKeyValue<'_>) -> Option<SpeakerId> {
        match SpeakerId::new(kv.value) {
            Ok(speaker) => Some(speaker),
            Err(_) => {
                self.diagnostics
                    .push(malformed_header(kv.value_span.clone()));
                None
            }
        }
    }

    fn metadata_entry(&mut self, kv: HeaderKeyValue<'_>) -> Option<MetadataEntry> {
        match metadata_entry(kv) {
            Ok(entry) => Some(entry),
            Err(span) => {
                self.diagnostics.push(malformed_header(span));
                None
            }
        }
    }

    fn divert_target(&mut self, field: HeaderField<'_>) -> Option<DivertTarget> {
        if field.text == "END" {
            return Some(DivertTarget::End);
        }

        let reference = if let Some((file, block_id)) = field.text.split_once("::") {
            if file.is_empty() || block_id.is_empty() || block_id.contains("::") {
                self.diagnostics
                    .push(malformed_divert_target(field.span(self.path)));
                return None;
            }

            let Ok(block_id) = BlockId::new(block_id) else {
                self.diagnostics
                    .push(malformed_divert_target(field.span(self.path)));
                return None;
            };

            BlockReference::external(file, block_id)
        } else {
            let Ok(block_id) = BlockId::new(field.text) else {
                self.diagnostics
                    .push(malformed_divert_target(field.span(self.path)));
                return None;
            };

            BlockReference::local(block_id)
        };

        Some(DivertTarget::Block(reference))
    }

    fn is_else_at(&self, index: usize, indent: usize) -> bool {
        self.lines.get(index).is_some_and(|line| {
            line.indent_len() == indent
                && classify_line(*line) == ClassifiedLine::Statement(StatementMarker::Else)
        })
    }

    fn skip_statement_body(&self, header_index: usize) -> usize {
        let header_indent = self.lines[header_index].indent_len();
        let mut index = header_index + 1;

        while index < self.lines.len() {
            let line = self.lines[index];

            if classify_line(line) == ClassifiedLine::Statement(StatementMarker::Block) {
                break;
            }

            if !line.trimmed_content().is_empty() && line.indent_len() <= header_indent {
                break;
            }

            index += 1;
        }

        index
    }

    fn report_statement_before_block(&mut self, line: LogicalLine<'_>, trimmed: &str) {
        if trimmed.is_empty()
            || classify_line(line) == ClassifiedLine::Statement(StatementMarker::Comment)
        {
            return;
        }

        self.diagnostics.push(statement_before_block(span_for_line(
            self.path,
            line.number,
            1,
        )));
    }
}

fn header_fields<'a>(
    trimmed: &'a str,
    marker: StatementMarker,
    line: LogicalLine<'_>,
    base_column: usize,
) -> Vec<HeaderField<'a>> {
    fields_after_prefix(trimmed, marker.text(), line.number, base_column).collect()
}

fn metadata_entry(kv: HeaderKeyValue<'_>) -> Result<MetadataEntry, SourceSpan> {
    let value = kv.parse_value()?;

    Ok(MetadataEntry::new(kv.key, value)
        .with_source_span(kv.field_span)
        .with_key_value_spans(kv.key_span, Some(kv.value_span)))
}

fn effect_mode(value: &str) -> Option<EffectMode> {
    match value {
        "deferred" => Some(EffectMode::Deferred),
        "immediate" => Some(EffectMode::Immediate),
        "blocking" => Some(EffectMode::Blocking),
        _ => None,
    }
}

fn choice_echo(value: &str) -> Option<ChoiceEcho> {
    match value {
        "none" => Some(ChoiceEcho::None),
        "selected_text" => Some(ChoiceEcho::SelectedText),
        _ => {
            let line_id = value.strip_prefix("line(")?.strip_suffix(')')?;
            Some(ChoiceEcho::Line(LineId::new(line_id).ok()?))
        }
    }
}

fn trim_trailing_blank_lines(lines: &mut Vec<String>) {
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
}
