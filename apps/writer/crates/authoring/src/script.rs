//! Structural reading view derived from the parser, never a second dialogue grammar.
use crate::{Document, EditError, Passage};
use recite_core::{
    SourceId,
    ast::{DivertTarget, Statement},
};
use recite_parser::parse;
use std::cell::OnceCell;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptBlock {
    pub id: String,
    pub is_default: bool,
    pub entries: Vec<ScriptEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
        Ok(self.script_snapshot()?.to_vec())
    }
    pub(crate) fn project_script(&self) -> Result<Vec<ScriptBlock>, EditError> {
        let parsed = parse(self.key().as_str(), self.source());
        let lowered = parsed.lower_source_file();
        if !lowered.diagnostics.is_empty() {
            return Err(EditError::SourceRequired("repair the syntax diagnostics"));
        }
        let passages = self.passage_snapshot_from(&parsed, &lowered)?;
        let passages: std::collections::BTreeMap<_, _> =
            passages.iter().map(|p| (p.id.as_str(), p)).collect();
        let source_lines = OnceCell::new();
        let heading = |line: u32| {
            source_lines
                .get_or_init(|| self.source().lines().collect::<Vec<_>>())
                .get(line.saturating_sub(1) as usize)
                .copied()
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        Ok(lowered
            .source_file
            .blocks
            .iter()
            .map(|block| ScriptBlock {
                id: block.id.to_string(),
                is_default: block.is_default,
                entries: entries(&block.statements, &heading, &passages),
            })
            .collect())
    }
}

fn entries(
    statements: &[Statement],
    heading: &impl Fn(u32) -> String,
    passages: &std::collections::BTreeMap<&str, &Passage>,
) -> Vec<ScriptEntry> {
    let mut result = Vec::new();
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
                result.extend(entries(&line.statements, heading, passages));
            }
            Statement::Choice(choice) => {
                let mut content = Vec::new();
                if let Some(passage) = find(&choice.source_id, passages) {
                    content.push(ScriptEntry::Passage(passage));
                }
                content.extend(entries(&choice.statements, heading, passages));
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
                    entries: entries(&branch.then_statements, heading, passages),
                });
                if !branch.else_statements.is_empty() {
                    result.push(ScriptEntry::Group {
                        heading: "Otherwise".into(),
                        entries: entries(&branch.else_statements, heading, passages),
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
                        entries: entries(&arm.statements, heading, passages),
                    })
                    .collect(),
            }),
            Statement::Comment(_) => {}
        }
    }
    result
}

fn find(id: &SourceId, passages: &std::collections::BTreeMap<&str, &Passage>) -> Option<Passage> {
    let SourceId::Frozen { anchor, .. } = id else {
        return None;
    };
    passages
        .get(anchor.as_str())
        .map(|passage| (*passage).clone())
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
