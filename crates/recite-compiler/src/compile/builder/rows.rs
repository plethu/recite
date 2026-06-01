use recite_core::{
    BlockIndex, BlockLookupEntry, BlockLookupTable, Choice, ChoiceIndex, ChoiceLookupEntry,
    ChoiceLookupTable, ChoiceRange, CompiledChoice, CompiledDivertTarget, CompiledEffect,
    CompiledLine, CompiledMatchArm, CompiledMetadataEntry, CompiledSourceMapEntry, CompiledSpeaker,
    DivertTarget, Effect, EffectIndex, Line, LineIndex, LineLookupEntry, LineLookupTable, MatchArm,
    MatchArmIndex, MatchArmRange, MetadataIndex, MetadataRange, ScalarValue, SourceFileIndex,
    SourceMapIndex, SourceMetadata, SourceMetadataScalar, SourceMetadataValue, SourceSpan,
    SpeakerId, SpeakerIndex, Value,
};

use super::AssetBuilder;
use crate::compile::CompileError;
use crate::compile::convert::{
    compile_argument, compile_choice_echo, compile_condition_expression, compile_effect_mode,
    compile_match_pattern, effect_id_for, required_choice_id, required_line_id,
};
use crate::compile::table::{increment_u32_len, usize_to_u32};

impl AssetBuilder<'_> {
    pub(super) fn compile_line_row(&mut self, line: &Line) -> Result<LineIndex, CompileError> {
        let index = LineIndex::new(usize_to_u32("lines", self.lines.len())?);
        let speaker = line
            .speaker
            .as_ref()
            .map(|speaker| self.intern_speaker(speaker))
            .transpose()?;
        let metadata = self.compile_metadata(&line.metadata)?;
        let source_map = self.push_source_map(&line.span)?;
        let id = required_line_id(line)?;

        self.lines.push(CompiledLine {
            id,
            source_text: line.source_text.text.clone(),
            speaker,
            metadata,
            source_map,
        });

        Ok(index)
    }

    pub(super) fn compile_choices<'b>(
        &mut self,
        choices: impl IntoIterator<Item = &'b Choice>,
    ) -> Result<ChoiceRange, CompileError> {
        let start = ChoiceIndex::new(usize_to_u32("choices", self.choices.len())?);
        let mut len = 0_u32;
        for choice in choices {
            self.compile_choice_row(choice)?;
            len = increment_u32_len("choices", len)?;
        }

        Ok(ChoiceRange::new(start, len))
    }

    fn compile_choice_row(&mut self, choice: &Choice) -> Result<ChoiceIndex, CompileError> {
        let index = ChoiceIndex::new(usize_to_u32("choices", self.choices.len())?);
        let metadata = self.compile_metadata(&choice.metadata)?;
        let condition = choice.condition.as_ref().map(compile_condition_expression);
        let target = choice
            .target
            .as_ref()
            .ok_or_else(|| {
                CompileError::InvalidValidatedInput(
                    "validated choice is missing a compile target".to_owned(),
                )
            })
            .and_then(|target| self.compile_divert_target(&target.target))?;
        let source_map = self.push_source_map(&choice.span)?;
        let id = required_choice_id(choice)?;

        self.choices.push(CompiledChoice {
            id,
            source_text: choice.source_text.text.clone(),
            metadata,
            condition,
            target,
            echo: compile_choice_echo(&choice.echo),
            source_map,
        });

        Ok(index)
    }

    pub(super) fn compile_match_arms(
        &mut self,
        arms: &[MatchArm],
    ) -> Result<MatchArmRange, CompileError> {
        let start = MatchArmIndex::new(usize_to_u32("match arms", self.match_arms.len())?);
        let mut len = 0_u32;
        for arm in arms {
            let statements = self.compile_statement_range(&arm.statements)?;
            let source_map = self.push_source_map(&arm.span)?;
            self.match_arms.push(CompiledMatchArm {
                pattern: compile_match_pattern(&arm.pattern),
                statements,
                source_map,
            });
            len = increment_u32_len("match arms", len)?;
        }

        Ok(MatchArmRange::new(start, len))
    }

    pub(super) fn compile_metadata(
        &mut self,
        metadata: &SourceMetadata,
    ) -> Result<MetadataRange, CompileError> {
        let start = MetadataIndex::new(usize_to_u32("metadata", self.metadata.len())?);
        let mut len = 0_u32;
        for entry in metadata {
            let source_map = entry
                .source_span
                .as_ref()
                .map(|span| self.push_source_map(span))
                .transpose()?;
            self.metadata.push(CompiledMetadataEntry {
                key: entry.key.clone(),
                value: lower_source_metadata_value(&entry.value),
                source_map,
            });
            len = increment_u32_len("metadata", len)?;
        }

        Ok(MetadataRange::new(start, len))
    }

    pub(super) fn compile_effect_row(
        &mut self,
        effect: &Effect,
    ) -> Result<EffectIndex, CompileError> {
        let index = EffectIndex::new(usize_to_u32("effects", self.effects.len())?);
        let source_map = self.push_source_map(&effect.span)?;
        self.effects.push(CompiledEffect {
            id: effect_id_for(effect)?,
            mode: compile_effect_mode(effect.mode),
            function: effect.function.clone(),
            args: effect.args.iter().map(compile_argument).collect(),
            source_map,
        });

        Ok(index)
    }

    pub(super) fn compile_divert_target(
        &self,
        target: &DivertTarget,
    ) -> Result<CompiledDivertTarget, CompileError> {
        match target {
            DivertTarget::Block(reference) => {
                let Some(index) = self.block_indices.get(reference.block_id.as_str()) else {
                    return Err(CompileError::InvalidValidatedInput(format!(
                        "validated block reference `{}` was not indexed",
                        reference.block_id
                    )));
                };
                Ok(CompiledDivertTarget::Block(*index))
            }
            DivertTarget::End => Ok(CompiledDivertTarget::End),
        }
    }

    pub(super) fn push_source_map(
        &mut self,
        span: &SourceSpan,
    ) -> Result<SourceMapIndex, CompileError> {
        let index = SourceMapIndex::new(usize_to_u32("source maps", self.source_maps.len())?);
        let source_file = self.source_file_index(&span.file)?;
        self.source_maps.push(CompiledSourceMapEntry {
            source_file,
            span: span.clone(),
        });

        Ok(index)
    }

    pub(super) fn source_file_index(&self, path: &str) -> Result<SourceFileIndex, CompileError> {
        self.source_file_indices.get(path).copied().ok_or_else(|| {
            CompileError::InvalidValidatedInput(format!(
                "validated source span referenced unindexed source file `{path}`"
            ))
        })
    }

    pub(super) fn intern_speaker(
        &mut self,
        speaker: &SpeakerId,
    ) -> Result<SpeakerIndex, CompileError> {
        if let Some(index) = self.speakers_by_id.get(speaker.as_str()) {
            return Ok(*index);
        }

        let index = SpeakerIndex::new(usize_to_u32("speakers", self.speakers.len())?);
        self.speakers_by_id
            .insert(speaker.as_str().to_owned(), index);
        self.speakers.push(CompiledSpeaker {
            id: speaker.clone(),
        });

        Ok(index)
    }

    pub(super) fn block_lookup(&self) -> Result<BlockLookupTable, CompileError> {
        let mut entries = self
            .blocks
            .iter()
            .enumerate()
            .map(|(index, block)| {
                Ok(BlockLookupEntry {
                    id: block.id.clone(),
                    index: BlockIndex::new(usize_to_u32("blocks", index)?),
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;
        entries.sort_by(|left, right| left.id.cmp(&right.id));

        Ok(BlockLookupTable::new(entries)?)
    }

    pub(super) fn line_lookup(&self) -> Result<LineLookupTable, CompileError> {
        let mut entries = self
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                Ok(LineLookupEntry {
                    id: line.id.clone(),
                    index: LineIndex::new(usize_to_u32("lines", index)?),
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;
        entries.sort_by(|left, right| left.id.cmp(&right.id));

        Ok(LineLookupTable::new(entries)?)
    }

    pub(super) fn choice_lookup(&self) -> Result<ChoiceLookupTable, CompileError> {
        let mut entries = self
            .choices
            .iter()
            .enumerate()
            .map(|(index, choice)| {
                Ok(ChoiceLookupEntry {
                    id: choice.id.clone(),
                    index: ChoiceIndex::new(usize_to_u32("choices", index)?),
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;
        entries.sort_by(|left, right| left.id.cmp(&right.id));

        Ok(ChoiceLookupTable::new(entries)?)
    }
}

fn lower_source_metadata_value(value: &SourceMetadataValue) -> Value {
    match value {
        SourceMetadataValue::Scalar(value) => Value::Scalar(lower_source_metadata_scalar(value)),
        SourceMetadataValue::Array(values) => Value::Array(
            values
                .iter()
                .map(lower_source_metadata_scalar)
                .collect::<Vec<_>>(),
        ),
    }
}

fn lower_source_metadata_scalar(value: &SourceMetadataScalar) -> ScalarValue {
    match value {
        SourceMetadataScalar::Symbol(value) | SourceMetadataScalar::StringLiteral(value) => {
            ScalarValue::String(value.clone())
        }
        SourceMetadataScalar::Integer(value) => ScalarValue::Integer(*value),
        SourceMetadataScalar::Float(value) => ScalarValue::Float(*value),
        SourceMetadataScalar::Bool(value) => ScalarValue::Boolean(*value),
    }
}
