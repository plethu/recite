//! Structural reading view derived from the parser, never a second dialogue grammar.
use crate::{Document, EditError, Passage};
use recite_core::{DivertTarget, SourceId, Statement};
use recite_parser::parse;

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptBlock {
    pub id: String,
    pub is_default: bool,
    pub entries: Vec<ScriptEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScriptEntry {
    Passage(Passage),
    Jump(String),
    Effect(String),
    Group {
        heading: String,
        entries: Vec<ScriptEntry>,
    },
    Source(String),
}

impl Document {
    pub fn script(&self) -> Result<Vec<ScriptBlock>, EditError> {
        let parsed = parse(self.key().as_str(), self.source()).lower_source_file();
        if !parsed.diagnostics.is_empty() {
            return Err(EditError::SourceRequired("repair the syntax diagnostics"));
        }
        let passages = self.passages()?;
        Ok(parsed
            .source_file
            .blocks
            .iter()
            .map(|block| ScriptBlock {
                id: block.id.to_string(),
                is_default: block.is_default,
                entries: entries(&block.statements, self.source(), &passages),
            })
            .collect())
    }
}

fn entries(statements: &[Statement], source: &str, passages: &[Passage]) -> Vec<ScriptEntry> {
    let mut result = Vec::new();
    let heading = |line: u32| {
        source
            .lines()
            .nth(line.saturating_sub(1) as usize)
            .unwrap_or_default()
            .trim()
            .to_owned()
    };
    for statement in statements {
        match statement {
            Statement::Line(line) => {
                if let Some(passage) = find(&line.source_id, passages) {
                    result.push(ScriptEntry::Passage(passage));
                } else {
                    result.push(ScriptEntry::Source(format!(
                        "{}\n{}",
                        heading(line.span.start.line()),
                        line.source_text.text
                    )));
                    if let Some(plural) = &line.plural_source_text {
                        result.push(ScriptEntry::Source(plural.text.clone()));
                    }
                }
                result.extend(entries(&line.statements, source, passages));
            }
            Statement::Choice(choice) => {
                let mut content = Vec::new();
                if let Some(passage) = find(&choice.source_id, passages) {
                    content.push(ScriptEntry::Passage(passage));
                }
                content.extend(entries(&choice.statements, source, passages));
                if choice.availability_requirement.is_some()
                    || choice.availability_reason_override.is_some()
                {
                    result.push(ScriptEntry::Group {
                        heading: heading(choice.span.start.line()),
                        entries: content,
                    });
                } else {
                    result.extend(content);
                }
            }
            Statement::Divert(divert) => {
                result.push(ScriptEntry::Jump(destination(&divert.target)))
            }
            Statement::Effect(effect) => {
                result.push(ScriptEntry::Effect(heading(effect.span.start.line())))
            }
            Statement::If(branch) => {
                result.push(ScriptEntry::Group {
                    heading: heading(branch.span.start.line()),
                    entries: entries(&branch.then_statements, source, passages),
                });
                if !branch.else_statements.is_empty() {
                    result.push(ScriptEntry::Group {
                        heading: "Otherwise".into(),
                        entries: entries(&branch.else_statements, source, passages),
                    });
                }
            }
            Statement::Match(branch) => result.push(ScriptEntry::Group {
                heading: heading(branch.span.start.line()),
                entries: branch
                    .arms
                    .iter()
                    .map(|arm| ScriptEntry::Group {
                        heading: heading(arm.span.start.line()),
                        entries: entries(&arm.statements, source, passages),
                    })
                    .collect(),
            }),
            Statement::Comment(_) => {}
        }
    }
    result
}

fn find(id: &SourceId, passages: &[Passage]) -> Option<Passage> {
    let SourceId::Frozen { anchor, .. } = id else {
        return None;
    };
    passages
        .iter()
        .find(|passage| passage.id == anchor.as_str())
        .cloned()
}

fn destination(target: &DivertTarget) -> String {
    match target {
        DivertTarget::End => "END".into(),
        DivertTarget::Block(reference) => reference.file.as_ref().map_or_else(
            || reference.block_id.to_string(),
            |file| format!("{file}::{}", reference.block_id),
        ),
    }
}
