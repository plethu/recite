use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    Block, BlockReference, Choice, Diagnostic, Divert, DivertTarget, Line, SourceFile,
    SourcePosition, SourceSpan, Statement,
};

use crate::diagnostics;

/// Result of semantic validation over one or more Recite source files.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Validate one parsed source file.
#[must_use]
pub fn validate_source_file(source_file: &SourceFile) -> ValidationReport {
    validate_source_files(std::slice::from_ref(source_file))
}

/// Validate parsed source files as one project.
#[must_use]
pub fn validate_source_files(source_files: &[SourceFile]) -> ValidationReport {
    let mut validator = Validator::new(source_files);
    validator.validate();

    ValidationReport {
        diagnostics: validator.diagnostics,
    }
}

struct Validator<'a> {
    source_files: &'a [SourceFile],
    diagnostics: Vec<Diagnostic>,
    blocks: BTreeSet<BlockKey>,
    line_ids: BTreeMap<&'a str, SourceSpan>,
    choice_ids: BTreeMap<&'a str, SourceSpan>,
    first_default: Option<&'a Block>,
    default_count: usize,
}

impl<'a> Validator<'a> {
    fn new(source_files: &'a [SourceFile]) -> Self {
        Self {
            source_files,
            diagnostics: Vec::new(),
            blocks: collect_blocks(source_files),
            line_ids: BTreeMap::new(),
            choice_ids: BTreeMap::new(),
            first_default: None,
            default_count: 0,
        }
    }

    fn validate(&mut self) {
        for source_file in self.source_files {
            self.validate_source_file(source_file);
        }

        if self.default_count == 0 && !self.source_files.is_empty() {
            self.diagnostics
                .push(diagnostics::missing_default_block(first_source_span(
                    self.source_files,
                )));
        }
    }

    fn validate_source_file(&mut self, source_file: &'a SourceFile) {
        for block in &source_file.blocks {
            self.validate_default_block(block);
            for statement in &block.statements {
                self.validate_statement(source_file, statement);
            }
        }
    }

    fn validate_default_block(&mut self, block: &'a Block) {
        if !block.is_default {
            return;
        }

        self.default_count += 1;
        if let Some(first) = self.first_default {
            self.diagnostics
                .push(diagnostics::ambiguous_default_block(block, first));
        } else {
            self.first_default = Some(block);
        }
    }

    fn validate_statement(&mut self, source_file: &'a SourceFile, statement: &'a Statement) {
        match statement {
            Statement::Line(line) => {
                self.validate_line(line);
                for statement in &line.statements {
                    self.validate_statement(source_file, statement);
                }
            }
            Statement::Choice(choice) => {
                self.validate_choice(source_file, choice);
                for statement in &choice.statements {
                    self.validate_statement(source_file, statement);
                }
            }
            Statement::Divert(divert) => self.validate_divert(source_file, divert),
            Statement::If(branch) => {
                for statement in &branch.then_statements {
                    self.validate_statement(source_file, statement);
                }
                for statement in &branch.else_statements {
                    self.validate_statement(source_file, statement);
                }
            }
            Statement::Match(branch) => {
                for arm in &branch.arms {
                    for statement in &arm.statements {
                        self.validate_statement(source_file, statement);
                    }
                }
            }
            Statement::Effect(_) | Statement::Comment(_) => {}
        }
    }

    fn validate_line(&mut self, line: &'a Line) {
        let Some(id) = &line.id else {
            self.diagnostics.push(diagnostics::missing_line_id(line));
            return;
        };

        if let Some(first_span) = self.line_ids.get(id.as_str()) {
            self.diagnostics
                .push(diagnostics::duplicate_line_id(line, first_span.clone()));
        } else {
            self.line_ids.insert(id.as_str(), line.span.clone());
        }
    }

    fn validate_choice(&mut self, source_file: &'a SourceFile, choice: &'a Choice) {
        if let Some(id) = &choice.id {
            if let Some(first_span) = self.choice_ids.get(id.as_str()) {
                self.diagnostics
                    .push(diagnostics::duplicate_choice_id(choice, first_span.clone()));
            } else {
                self.choice_ids.insert(id.as_str(), choice.span.clone());
            }
        } else {
            self.diagnostics
                .push(diagnostics::missing_choice_id(choice));
        }

        if let Some(target) = &choice.target {
            self.validate_reference(source_file, &target.target, &target.span);
        }
    }

    fn validate_divert(&mut self, source_file: &'a SourceFile, divert: &'a Divert) {
        self.validate_reference(source_file, &divert.target, &divert.span);
    }

    fn validate_reference(
        &mut self,
        source_file: &'a SourceFile,
        target: &'a DivertTarget,
        span: &SourceSpan,
    ) {
        let DivertTarget::Block(reference) = target else {
            return;
        };

        if !self
            .blocks
            .contains(&BlockKey::from_reference(source_file, reference))
        {
            self.diagnostics.push(diagnostics::unknown_block_reference(
                reference,
                span.clone(),
            ));
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BlockKey {
    file: String,
    block_id: String,
}

impl BlockKey {
    fn from_reference(source_file: &SourceFile, reference: &BlockReference) -> Self {
        Self {
            file: reference
                .file
                .clone()
                .unwrap_or_else(|| source_file.path.clone()),
            block_id: reference.block_id.as_str().to_owned(),
        }
    }
}

fn collect_blocks(source_files: &[SourceFile]) -> BTreeSet<BlockKey> {
    let mut blocks = BTreeSet::new();
    for source_file in source_files {
        for block in &source_file.blocks {
            blocks.insert(BlockKey {
                file: source_file.path.clone(),
                block_id: block.id.as_str().to_owned(),
            });
        }
    }
    blocks
}

fn first_source_span(source_files: &[SourceFile]) -> SourceSpan {
    source_files
        .iter()
        .find_map(|source_file| source_file.blocks.first().map(|block| block.span.clone()))
        .unwrap_or_else(|| {
            let path = source_files
                .first()
                .map_or_else(String::new, |source_file| source_file.path.clone());
            SourceSpan::point(
                path,
                SourcePosition::new(1, 1).expect("1:1 is a valid source position"),
            )
        })
}
