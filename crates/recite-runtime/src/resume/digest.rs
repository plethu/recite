//! Revision-independent digests of compiled dialogue structure.
//!
//! Table indices, source maps and positional effect IDs change whenever any
//! file in the project is edited, so they never enter these digests. Blocks,
//! lines and choices are referenced by their authored IDs instead.

use recite_core::{
    Value,
    compiled::{
        CompiledBlock, CompiledChoice, CompiledChoiceEcho, CompiledDivertTarget,
        CompiledEffectMode, CompiledLine, CompiledMatchPattern, CompiledStatementKind,
        MetadataRange, SpeakerIndex, StatementRange,
    },
};

use crate::DialogueError;
use crate::traversal::AssetView;

mod encoding;

pub(crate) use encoding::{arguments_digest, call_digest, condition_digest};
use encoding::{
    write_bindings, write_interpolation_mode, write_len, write_optional_str, write_scalar,
    write_str, write_tag, write_u32,
};

pub(crate) const DIGEST_LEN: usize = blake3::OUT_LEN;
pub(crate) type Digest = [u8; DIGEST_LEN];

/// Content and structure digests of one block.
///
/// `structure` covers control flow, conditions, effects, divert targets and
/// the stable IDs of lines and choices. `content` additionally covers text,
/// interpolation bindings, speakers and metadata, so equal structure with a
/// different content digest means only presentation changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BlockDigests {
    pub(crate) content: Digest,
    pub(crate) structure: Digest,
}

pub(crate) fn block_digests(
    asset: AssetView<'_>,
    block: &CompiledBlock,
) -> Result<BlockDigests, DialogueError> {
    let mut writer = BlockWriter {
        asset,
        content: blake3::Hasher::new(),
        structure: blake3::Hasher::new(),
    };
    writer.structural_tag(b"block");
    writer.structural_str(block.id.as_str());
    writer.speaker(block.default_speaker)?;
    writer.metadata(block.metadata)?;
    writer.statements(block.statements)?;
    Ok(BlockDigests {
        content: *writer.content.finalize().as_bytes(),
        structure: *writer.structure.finalize().as_bytes(),
    })
}

struct BlockWriter<'a> {
    asset: AssetView<'a>,
    content: blake3::Hasher,
    structure: blake3::Hasher,
}

impl BlockWriter<'_> {
    fn structural_tag(&mut self, tag: &[u8]) {
        write_tag(&mut self.structure, tag);
        write_tag(&mut self.content, tag);
    }

    fn structural_str(&mut self, value: &str) {
        write_str(&mut self.structure, value);
        write_str(&mut self.content, value);
    }

    fn structural_u32(&mut self, value: u32) {
        write_u32(&mut self.structure, value);
        write_u32(&mut self.content, value);
    }

    fn structural_digest(&mut self, digest: &Digest) {
        self.structure.update(digest);
        self.content.update(digest);
    }

    fn statements(&mut self, range: StatementRange) -> Result<(), DialogueError> {
        let statements = self.asset.statement_range(range)?;
        self.structural_tag(b"range");
        self.structural_u32(range.len);
        for index in statements {
            let statement = self.asset.statement_at(statement_index(index))?;
            self.statement(&statement.kind)?;
        }
        Ok(())
    }

    fn statement(&mut self, kind: &CompiledStatementKind) -> Result<(), DialogueError> {
        match kind {
            CompiledStatementKind::Line(line) => {
                self.structural_tag(b"line");
                self.line(self.asset.line_at(*line)?)?;
            }
            CompiledStatementKind::Prompt { line, choices } => {
                self.structural_tag(b"prompt");
                match line {
                    Some(line) => {
                        self.structural_tag(b"some");
                        self.line(self.asset.line_at(*line)?)?;
                    }
                    None => self.structural_tag(b"none"),
                }
                let choices = self.asset.choices(*choices)?;
                self.structural_u32(u32::try_from(choices.len()).unwrap_or(u32::MAX));
                for choice in choices {
                    self.choice(choice)?;
                }
            }
            CompiledStatementKind::Divert(target) => {
                self.structural_tag(b"divert");
                self.target(target)?;
            }
            CompiledStatementKind::If {
                condition,
                then_statements,
                else_statements,
            } => {
                self.structural_tag(b"if");
                self.structural_digest(&condition_digest(condition));
                self.statements(*then_statements)?;
                self.statements(*else_statements)?;
            }
            CompiledStatementKind::Match { scrutinee, arms } => {
                self.structural_tag(b"match");
                self.structural_digest(&call_digest(&scrutinee.function, &scrutinee.args));
                let arms = self.asset.match_arms(*arms)?;
                self.structural_u32(u32::try_from(arms.len()).unwrap_or(u32::MAX));
                for arm in arms {
                    match &arm.pattern {
                        CompiledMatchPattern::Variant(variant) => {
                            self.structural_tag(b"variant");
                            self.structural_str(variant);
                        }
                        CompiledMatchPattern::Wildcard => self.structural_tag(b"wildcard"),
                    }
                    self.statements(arm.statements)?;
                }
            }
            CompiledStatementKind::Effect(effect) => {
                let effect = self.asset.effect_at(*effect)?;
                self.structural_tag(b"effect");
                self.structural_tag(effect_mode_tag(effect.mode));
                self.structural_str(&effect.function);
                self.structural_digest(&arguments_digest(&effect.args));
            }
            CompiledStatementKind::End => self.structural_tag(b"end"),
        }
        Ok(())
    }

    fn line(&mut self, line: &CompiledLine) -> Result<(), DialogueError> {
        self.structural_str(line.id.as_str());
        write_str(&mut self.content, &line.source_text);
        write_optional_str(&mut self.content, line.plural_source_text.as_deref());
        write_str(&mut self.content, &line.authored_source_text);
        write_optional_str(
            &mut self.content,
            line.authored_plural_source_text.as_deref(),
        );
        write_bindings(&mut self.content, &line.interpolation_bindings);
        write_interpolation_mode(&mut self.content, line.interpolation_mode);
        self.speaker(line.speaker)?;
        self.metadata(line.metadata)
    }

    fn choice(&mut self, choice: &CompiledChoice) -> Result<(), DialogueError> {
        self.structural_tag(b"choice");
        self.structural_str(choice.id.as_str());
        match &choice.availability_requirement {
            Some(requirement) => {
                self.structural_tag(b"requires");
                self.structural_digest(&condition_digest(requirement));
            }
            None => self.structural_tag(b"always"),
        }
        match &choice.availability_reason_override {
            Some(reason) => {
                self.structural_tag(b"reason");
                self.structural_str(reason.as_str());
            }
            None => self.structural_tag(b"no-reason"),
        }
        self.target(&choice.target)?;
        match &choice.echo {
            CompiledChoiceEcho::None => self.structural_tag(b"echo-none"),
            CompiledChoiceEcho::SelectedText => self.structural_tag(b"echo-selected"),
            CompiledChoiceEcho::ExplicitLine(line) => {
                self.structural_tag(b"echo-line");
                self.structural_str(line.as_str());
            }
        }
        write_str(&mut self.content, &choice.source_text);
        write_str(&mut self.content, &choice.authored_source_text);
        write_optional_str(
            &mut self.content,
            choice.availability_requirement_source_text.as_deref(),
        );
        write_bindings(&mut self.content, &choice.interpolation_bindings);
        write_interpolation_mode(&mut self.content, choice.interpolation_mode);
        self.metadata(choice.metadata)
    }

    fn target(&mut self, target: &CompiledDivertTarget) -> Result<(), DialogueError> {
        match target {
            CompiledDivertTarget::Block(block) => {
                let block = self.asset.block_at(*block)?;
                self.structural_tag(b"block");
                self.structural_str(block.id.as_str());
            }
            CompiledDivertTarget::End => self.structural_tag(b"end"),
        }
        Ok(())
    }

    fn speaker(&mut self, speaker: Option<SpeakerIndex>) -> Result<(), DialogueError> {
        match speaker {
            Some(speaker) => {
                let speaker = self.asset.speaker_at(speaker)?;
                write_tag(&mut self.content, b"speaker");
                write_str(&mut self.content, speaker.id.as_str());
            }
            None => write_tag(&mut self.content, b"no-speaker"),
        }
        Ok(())
    }

    fn metadata(&mut self, range: MetadataRange) -> Result<(), DialogueError> {
        let entries = self.asset.metadata_entries(range)?;
        write_len(&mut self.content, entries.len());
        for entry in entries {
            write_str(&mut self.content, &entry.key);
            match &entry.value {
                Value::Scalar(value) => {
                    write_tag(&mut self.content, b"scalar");
                    write_scalar(&mut self.content, value);
                }
                Value::Array(values) => {
                    write_tag(&mut self.content, b"array");
                    write_len(&mut self.content, values.len());
                    for value in values {
                        write_scalar(&mut self.content, value);
                    }
                }
            }
        }
        Ok(())
    }
}

fn statement_index(index: usize) -> recite_core::compiled::StatementIndex {
    // Ranges were validated against the table, whose length fits in u32.
    recite_core::compiled::StatementIndex::new(u32::try_from(index).unwrap_or(u32::MAX))
}

pub(crate) fn effect_mode_tag(mode: CompiledEffectMode) -> &'static [u8] {
    match mode {
        CompiledEffectMode::Deferred => b"deferred",
        CompiledEffectMode::Immediate => b"immediate",
        CompiledEffectMode::Blocking => b"blocking",
    }
}

pub(crate) const DIGEST_ALGORITHM: &str = "blake3";

impl BlockDigests {
    pub(crate) fn to_snapshot(self) -> crate::session_snapshot::DialogueBlockDigestsSnapshot {
        crate::session_snapshot::DialogueBlockDigestsSnapshot {
            content: digest_snapshot(&self.content),
            structure: digest_snapshot(&self.structure),
        }
    }
}

fn digest_snapshot(digest: &Digest) -> crate::session_snapshot::DialogueContentFingerprintSnapshot {
    crate::session_snapshot::DialogueContentFingerprintSnapshot {
        algorithm: DIGEST_ALGORITHM.to_owned(),
        digest: digest.to_vec(),
    }
}
