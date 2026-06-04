use std::collections::BTreeMap;

use recite_core::{
    AvailabilityReasonArgBinding, BlockIndex, Choice, CompiledAssetHeader,
    CompiledAvailabilityReason, CompiledAvailabilityReasonArgBinding,
    CompiledAvailabilityReasonArgValue, CompiledBlock, CompiledChoice,
    CompiledConditionAvailabilityReason, CompiledDialogue, CompiledEffect, CompiledLine,
    CompiledMatchArm, CompiledMetadataEntry, CompiledSourceFile, CompiledSourceMapEntry,
    CompiledSpeaker, CompiledStatement, DivertTarget, Effect, IfBranch, Line, ProjectSchema,
    SchemaLiteralValue, SourceFileIndex, SpeakerIndex, canonical_source_fingerprint,
};

use super::CompileError;
use super::api::CompileOptions;
use super::lowered::LoweredInput;
use super::table::usize_to_u32;

mod rows;
mod statements;

pub(super) fn build_dialogue(
    inputs: &[LoweredInput],
    options: CompileOptions,
    schema: Option<&ProjectSchema>,
) -> Result<CompiledDialogue, CompileError> {
    AssetBuilder::new(inputs, options, schema).compile()
}

struct ReservedStatement<'a> {
    index: usize,
    plan: StatementPlan<'a>,
}

enum StatementPlan<'a> {
    Line(&'a Line),
    StandalonePrompt(Vec<&'a Choice>),
    Divert(&'a DivertTarget),
    If(&'a IfBranch),
    Match(&'a recite_core::MatchBranch),
    Effect(&'a Effect),
}

struct AssetBuilder<'a> {
    inputs: &'a [LoweredInput],
    options: CompileOptions,
    schema: Option<&'a ProjectSchema>,
    source_file_indices: BTreeMap<&'a str, SourceFileIndex>,
    block_indices: BTreeMap<&'a str, BlockIndex>,
    speakers_by_id: BTreeMap<String, SpeakerIndex>,
    blocks: Vec<CompiledBlock>,
    statements: Vec<CompiledStatement>,
    match_arms: Vec<CompiledMatchArm>,
    lines: Vec<CompiledLine>,
    choices: Vec<CompiledChoice>,
    speakers: Vec<CompiledSpeaker>,
    metadata: Vec<CompiledMetadataEntry>,
    effects: Vec<CompiledEffect>,
    source_maps: Vec<CompiledSourceMapEntry>,
}

impl<'a> AssetBuilder<'a> {
    fn new(
        inputs: &'a [LoweredInput],
        options: CompileOptions,
        schema: Option<&'a ProjectSchema>,
    ) -> Self {
        Self {
            inputs,
            options,
            schema,
            source_file_indices: BTreeMap::new(),
            block_indices: BTreeMap::new(),
            speakers_by_id: BTreeMap::new(),
            blocks: Vec::new(),
            statements: Vec::new(),
            match_arms: Vec::new(),
            lines: Vec::new(),
            choices: Vec::new(),
            speakers: Vec::new(),
            metadata: Vec::new(),
            effects: Vec::new(),
            source_maps: Vec::new(),
        }
    }

    fn compile(mut self) -> Result<CompiledDialogue, CompileError> {
        self.index_source_files()?;
        self.index_blocks()?;

        let sources = self.compile_sources()?;
        for input in self.inputs {
            let source_file = &input.source_file;
            let source_file_index = self.source_file_index(&source_file.path)?;
            for block in &source_file.blocks {
                self.compile_block(source_file_index, block)?;
            }
        }

        let default_block = self.default_block_index()?;
        let block_lookup = self.block_lookup()?;
        let line_lookup = self.line_lookup()?;
        let choice_lookup = self.choice_lookup()?;
        let availability_reasons = self.compile_availability_reasons();
        let condition_availability_reasons = self.compile_condition_availability_reasons()?;

        Ok(CompiledDialogue {
            header: CompiledAssetHeader::messagepack_v0(
                self.options.compiler_version,
                self.options.asset_id,
                self.options.source_map_id,
                self.options.schema_fingerprint,
            ),
            default_block,
            sources,
            blocks: self.blocks,
            statements: self.statements,
            match_arms: self.match_arms,
            lines: self.lines,
            choices: self.choices,
            availability_reasons,
            condition_availability_reasons,
            speakers: self.speakers,
            metadata: self.metadata,
            effects: self.effects,
            source_maps: self.source_maps,
            block_lookup,
            line_lookup,
            choice_lookup,
        })
    }

    fn compile_availability_reasons(&self) -> Vec<CompiledAvailabilityReason> {
        let Some(schema) = self.schema else {
            return Vec::new();
        };

        schema
            .availability_reasons
            .iter()
            .map(|(id, reason)| CompiledAvailabilityReason {
                id: id.clone(),
                template: reason.template.clone(),
            })
            .collect()
    }

    fn compile_condition_availability_reasons(
        &self,
    ) -> Result<Vec<CompiledConditionAvailabilityReason>, CompileError> {
        let Some(schema) = self.schema else {
            return Ok(Vec::new());
        };

        let mut mappings = Vec::new();
        for (function, condition) in &schema.conditions {
            let Some(mapping) = &condition.availability_reason else {
                continue;
            };
            let mut args = Vec::new();
            for (name, value) in &mapping.args {
                args.push(CompiledAvailabilityReasonArgBinding {
                    name: name.clone(),
                    value: compiled_reason_arg_value(value, &condition.params)?,
                });
            }
            mappings.push(CompiledConditionAvailabilityReason {
                function: function.clone(),
                reason: mapping.reason.clone(),
                args,
            });
        }

        Ok(mappings)
    }

    fn index_source_files(&mut self) -> Result<(), CompileError> {
        for input in self.inputs {
            let index = SourceFileIndex::new(usize_to_u32(
                "source files",
                self.source_file_indices.len(),
            )?);
            self.source_file_indices
                .insert(input.source_file.path.as_str(), index);
        }

        Ok(())
    }

    fn index_blocks(&mut self) -> Result<(), CompileError> {
        for input in self.inputs {
            for block in &input.source_file.blocks {
                let index = BlockIndex::new(usize_to_u32("blocks", self.block_indices.len())?);
                self.block_indices.insert(block.id.as_str(), index);
            }
        }

        Ok(())
    }

    fn compile_sources(&self) -> Result<Vec<CompiledSourceFile>, CompileError> {
        self.inputs
            .iter()
            .map(|input| {
                Ok(CompiledSourceFile {
                    path: input.source_file.path.clone(),
                    fingerprint: canonical_source_fingerprint(&input.source),
                })
            })
            .collect()
    }

    fn compile_block(
        &mut self,
        source_file: SourceFileIndex,
        block: &recite_core::Block,
    ) -> Result<(), CompileError> {
        let metadata = self.compile_metadata(&block.metadata)?;
        let default_speaker = block
            .default_speaker
            .as_ref()
            .map(|speaker| self.intern_speaker(speaker))
            .transpose()?;
        let statements = self.compile_statement_range(&block.statements)?;
        let source_map = self.push_source_map(&block.span)?;

        self.blocks.push(CompiledBlock {
            id: block.id.clone(),
            source_file,
            statements,
            metadata,
            default_speaker,
            source_map,
        });

        Ok(())
    }

    fn default_block_index(&self) -> Result<BlockIndex, CompileError> {
        self.inputs
            .iter()
            .flat_map(|input| input.source_file.blocks.iter())
            .find(|block| block.is_default)
            .and_then(|block| self.block_indices.get(block.id.as_str()).copied())
            .ok_or_else(|| {
                CompileError::InvalidValidatedInput(
                    "validated project did not contain an indexed default block".to_owned(),
                )
            })
    }
}

fn compiled_reason_arg_value(
    value: &AvailabilityReasonArgBinding,
    params: &[recite_core::ParameterDefinition],
) -> Result<CompiledAvailabilityReasonArgValue, CompileError> {
    match value {
        AvailabilityReasonArgBinding::ConditionParam(name) => {
            let Some(index) = params.iter().position(|param| param.name == *name) else {
                return Err(CompileError::InvalidValidatedInput(format!(
                    "validated availability reason mapping references unknown condition parameter `{name}`"
                )));
            };
            Ok(CompiledAvailabilityReasonArgValue::ConditionArg(
                index.to_string(),
            ))
        }
        AvailabilityReasonArgBinding::Literal(value) => compiled_schema_literal(value),
    }
}

fn compiled_schema_literal(
    value: &SchemaLiteralValue,
) -> Result<CompiledAvailabilityReasonArgValue, CompileError> {
    match value {
        SchemaLiteralValue::String(value) => Ok(CompiledAvailabilityReasonArgValue::LiteralString(
            value.clone(),
        )),
        SchemaLiteralValue::Int(value) => {
            Ok(CompiledAvailabilityReasonArgValue::LiteralInt(*value))
        }
        SchemaLiteralValue::Float(value) => Ok(CompiledAvailabilityReasonArgValue::LiteralFloat(
            value.parse::<f64>().map_err(|_| {
                CompileError::InvalidValidatedInput(format!(
                    "validated availability reason float literal `{value}` is not a float"
                ))
            })?,
        )),
        SchemaLiteralValue::Bool(value) => {
            Ok(CompiledAvailabilityReasonArgValue::LiteralBool(*value))
        }
    }
}
