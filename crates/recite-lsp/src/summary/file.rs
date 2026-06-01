use std::path::{Path, PathBuf};

use lsp_types::Uri;
use recite_core::{
    Block, Choice, ConditionCall, ConditionExpression, Diagnostic, DivertTarget, Effect, IfBranch,
    Line, MatchBranch, SourceFile, SourceMetadata, SourceSpan, Statement,
};
use recite_parser::parse;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct FileSummary {
    pub(crate) identity: FileIdentity,
    pub(crate) version: Option<i32>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) completeness: FileSummaryCompleteness,
    pub(crate) blocks: Vec<SpannedName>,
    pub(crate) block_references: Vec<BlockReferenceSummary>,
    pub(crate) line_ids: Vec<SpannedName>,
    pub(crate) choice_ids: Vec<SpannedName>,
    pub(crate) missing_ids: Vec<MissingIdSummary>,
    pub(crate) metadata_keys: Vec<MetadataKeySummary>,
    pub(crate) condition_functions: Vec<FunctionReferenceSummary>,
    pub(crate) effect_functions: Vec<FunctionReferenceSummary>,
}

impl FileSummary {
    pub(crate) fn from_text(identity: FileIdentity, version: Option<i32>, text: &str) -> Self {
        let parse = parse(identity.uri().as_str(), text);
        let lowered = parse.lower_source_file();
        let diagnostics = unique_diagnostics(lowered.diagnostics.clone());
        let mut collector = FileSummaryCollector::new();
        collector.collect_source_file(&lowered.source_file);
        let complete_source_model = lowered.diagnostics.is_empty();

        Self {
            identity,
            version,
            diagnostics,
            completeness: FileSummaryCompleteness {
                block_definitions: complete_source_model,
                block_references: complete_source_model,
                stable_ids: complete_source_model,
                metadata: complete_source_model,
                condition_functions: complete_source_model,
                effect_functions: complete_source_model,
                inline_markup: false,
                recoverable_regions: false,
            },
            blocks: collector.blocks,
            block_references: collector.block_references,
            line_ids: collector.line_ids,
            choice_ids: collector.choice_ids,
            missing_ids: collector.missing_ids,
            metadata_keys: collector.metadata_keys,
            condition_functions: collector.condition_functions,
            effect_functions: collector.effect_functions,
        }
    }

    pub(crate) fn uri(&self) -> &Uri {
        self.identity.uri()
    }

    pub(crate) fn saved_path(&self) -> Option<&Path> {
        self.identity.saved_path()
    }

    pub(crate) fn project_relative_path(&self) -> Option<&str> {
        self.identity.project_relative_path()
    }
}

fn unique_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut unique = Vec::new();
    for diagnostic in diagnostics {
        if !unique.contains(&diagnostic) {
            unique.push(diagnostic);
        }
    }

    unique
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FileIdentity {
    Saved(SavedFileIdentity),
    Open(OpenFileIdentity),
}

impl FileIdentity {
    pub(crate) fn uri(&self) -> &Uri {
        match self {
            Self::Saved(identity) => &identity.uri,
            Self::Open(identity) => &identity.uri,
        }
    }

    pub(crate) fn saved_path(&self) -> Option<&Path> {
        match self {
            Self::Saved(identity) => Some(&identity.canonical_path),
            Self::Open(identity) => identity.saved_path.as_deref(),
        }
    }

    pub(crate) fn project_relative_path(&self) -> Option<&str> {
        match self {
            Self::Saved(identity) => Some(&identity.project_relative_path),
            Self::Open(identity) => identity.project_relative_path.as_deref(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SavedFileIdentity {
    pub(crate) uri: Uri,
    pub(crate) canonical_path: PathBuf,
    pub(crate) project_relative_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OpenFileIdentity {
    pub(crate) uri: Uri,
    pub(crate) saved_path: Option<PathBuf>,
    pub(crate) project_relative_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileSummaryCompleteness {
    pub(crate) block_definitions: bool,
    pub(crate) block_references: bool,
    pub(crate) stable_ids: bool,
    pub(crate) metadata: bool,
    pub(crate) condition_functions: bool,
    pub(crate) effect_functions: bool,
    pub(crate) inline_markup: bool,
    pub(crate) recoverable_regions: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SpannedName {
    pub(crate) name: String,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BlockReferenceSummary {
    pub(crate) file: Option<String>,
    pub(crate) block_id: String,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MissingIdSummary {
    pub(crate) kind: MissingIdKind,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MissingIdKind {
    Line,
    Choice,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MetadataKeySummary {
    pub(crate) key: String,
    pub(crate) key_span: Option<SourceSpan>,
    pub(crate) entry_span: Option<SourceSpan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FunctionReferenceSummary {
    pub(crate) name: String,
    pub(crate) span: SourceSpan,
}

#[derive(Default)]
struct FileSummaryCollector {
    blocks: Vec<SpannedName>,
    block_references: Vec<BlockReferenceSummary>,
    line_ids: Vec<SpannedName>,
    choice_ids: Vec<SpannedName>,
    missing_ids: Vec<MissingIdSummary>,
    metadata_keys: Vec<MetadataKeySummary>,
    condition_functions: Vec<FunctionReferenceSummary>,
    effect_functions: Vec<FunctionReferenceSummary>,
}

impl FileSummaryCollector {
    fn new() -> Self {
        Self::default()
    }

    fn collect_source_file(&mut self, source_file: &SourceFile) {
        for block in &source_file.blocks {
            self.collect_block(block);
        }
    }

    fn collect_block(&mut self, block: &Block) {
        self.blocks.push(SpannedName {
            name: block.id.as_str().to_owned(),
            span: block.span.clone(),
        });
        self.collect_metadata(&block.metadata);
        for statement in &block.statements {
            self.collect_statement(statement);
        }
    }

    fn collect_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Line(line) => self.collect_line(line),
            Statement::Choice(choice) => self.collect_choice(choice),
            Statement::Divert(divert) => self.collect_divert_target(&divert.target, &divert.span),
            Statement::If(branch) => self.collect_if_branch(branch),
            Statement::Match(branch) => self.collect_match_branch(branch),
            Statement::Effect(effect) => self.collect_effect(effect),
            Statement::Comment(_) => {}
        }
    }

    fn collect_line(&mut self, line: &Line) {
        match &line.id {
            Some(id) => self.line_ids.push(SpannedName {
                name: id.as_str().to_owned(),
                span: line.span.clone(),
            }),
            None => self.missing_ids.push(MissingIdSummary {
                kind: MissingIdKind::Line,
                span: line.span.clone(),
            }),
        }
        self.collect_metadata(&line.metadata);
        for statement in &line.statements {
            self.collect_statement(statement);
        }
    }

    fn collect_choice(&mut self, choice: &Choice) {
        match &choice.id {
            Some(id) => self.choice_ids.push(SpannedName {
                name: id.as_str().to_owned(),
                span: choice.span.clone(),
            }),
            None => self.missing_ids.push(MissingIdSummary {
                kind: MissingIdKind::Choice,
                span: choice.span.clone(),
            }),
        }
        self.collect_metadata(&choice.metadata);
        if let Some(condition) = &choice.condition {
            self.collect_condition_expression(condition);
        }
        if let Some(target) = &choice.target {
            self.collect_divert_target(&target.target, &target.span);
        }
        for statement in &choice.statements {
            self.collect_statement(statement);
        }
    }

    fn collect_if_branch(&mut self, branch: &IfBranch) {
        self.collect_condition_expression(&branch.condition);
        for statement in &branch.then_statements {
            self.collect_statement(statement);
        }
        for statement in &branch.else_statements {
            self.collect_statement(statement);
        }
    }

    fn collect_match_branch(&mut self, branch: &MatchBranch) {
        self.collect_condition_call(&branch.scrutinee);
        for arm in &branch.arms {
            for statement in &arm.statements {
                self.collect_statement(statement);
            }
        }
    }

    fn collect_effect(&mut self, effect: &Effect) {
        self.effect_functions.push(FunctionReferenceSummary {
            name: effect.function.clone(),
            span: effect
                .function_span
                .clone()
                .unwrap_or_else(|| effect.span.clone()),
        });
    }

    fn collect_divert_target(&mut self, target: &DivertTarget, span: &SourceSpan) {
        if let DivertTarget::Block(reference) = target {
            self.block_references.push(BlockReferenceSummary {
                file: reference.file.clone(),
                block_id: reference.block_id.as_str().to_owned(),
                span: span.clone(),
            });
        }
    }

    fn collect_condition_expression(&mut self, expression: &ConditionExpression) {
        match expression {
            ConditionExpression::Call(call) => self.collect_condition_call(call),
            ConditionExpression::And(group) | ConditionExpression::Or(group) => {
                for expression in &group.expressions {
                    self.collect_condition_expression(expression);
                }
            }
            ConditionExpression::Not(unary) | ConditionExpression::Grouped(unary) => {
                self.collect_condition_expression(&unary.expression);
            }
        }
    }

    fn collect_condition_call(&mut self, call: &ConditionCall) {
        self.condition_functions.push(FunctionReferenceSummary {
            name: call.function.clone(),
            span: call
                .function_span
                .clone()
                .unwrap_or_else(|| call.span.clone()),
        });
    }

    fn collect_metadata(&mut self, metadata: &SourceMetadata) {
        for entry in metadata {
            self.metadata_keys.push(MetadataKeySummary {
                key: entry.key.clone(),
                key_span: entry.key_span.clone(),
                entry_span: entry.source_span.clone(),
            });
        }
    }
}
