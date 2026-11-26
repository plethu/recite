//! Locate a compiled traversal position in the source AST, start the
//! reference interpreter there, and describe what led up to it.
//!
//! A compiled statement range holds one statement per source statement,
//! except that comments are dropped and a run of standalone choices is one
//! prompt spanning its first choice. A range is found by the spans of its
//! statements, so a position maps without trusting the compiler's layout.

use recite_core::SourceSpan;
use recite_core::ast::{Block, ConditionExpression, MatchPattern, Statement};

use crate::observation::EffectObservation;
use crate::oracle::{OracleSession, ReferenceProgram, effect_observation};

/// One level of a position: the spans of a compiled range's statements and
/// how many of them have run.
#[derive(Clone, Debug)]
pub struct RangePosition {
    pub spans: Vec<SourceSpan>,
    pub next: usize,
}

/// What a host observed or traversal decided before a position, back to the
/// latest line or prompt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryItem {
    Line(String),
    Prompt {
        line: Option<String>,
        choices: Vec<String>,
    },
    Effect(EffectObservation),
    /// A condition or scrutinee, and the branch traversal entered.
    Branch(String, String),
    /// A condition or scrutinee whose branch traversal entered and left.
    Passed(String),
    BlockStart(String),
}

/// A mapped position: the block, and for each level its statement list and
/// the source index traversal continues at.
pub(crate) struct Located<'a> {
    pub(crate) file: usize,
    pub(crate) block: &'a Block,
    pub(crate) frames: Vec<(&'a [Statement], usize)>,
}

impl ReferenceProgram {
    pub(crate) fn locate(
        &self,
        block_id: &str,
        levels: &[RangePosition],
    ) -> Result<Located<'_>, String> {
        let (file, block) = self
            .files
            .iter()
            .enumerate()
            .find_map(|(file, source)| {
                source
                    .blocks
                    .iter()
                    .find(|block| block.id.as_str() == block_id)
                    .map(|block| (file, block))
            })
            .ok_or_else(|| format!("no block `{block_id}`"))?;
        let mut frames = Vec::with_capacity(levels.len());
        for level in levels {
            let list = if level.spans.is_empty() {
                &[][..]
            } else {
                find_list(&block.statements, &level.spans).ok_or_else(|| {
                    format!(
                        "no statement list in `{block_id}` has spans {:?}",
                        level.spans
                    )
                })?
            };
            let units = units(list);
            let next = units
                .get(level.next)
                .map_or(list.len(), |(index, _)| *index);
            frames.push((list, next));
        }
        Ok(Located {
            file,
            block,
            frames,
        })
    }

    /// A reference session at a position, carrying the deferred effects a
    /// save holds. With `pending`, the session stands at the statement that
    /// emitted the pending prompt or blocking effect, so its next advance
    /// presents it again.
    pub fn resume_at<'a>(
        &'a self,
        answers: &'a crate::Answers,
        block_id: &str,
        levels: &[RangePosition],
        pending: bool,
        deferred: Vec<EffectObservation>,
    ) -> Result<OracleSession<'a>, String> {
        let mut located = self.locate(block_id, levels)?;
        if pending {
            let (list, next) = located.frames.last_mut().ok_or("no position")?;
            *next = units(list)
                .iter()
                .rev()
                .map(|(index, _)| *index)
                .find(|index| index < next)
                .ok_or("no statement before a pending position")?;
        }
        Ok(OracleSession::at(self, answers, located, deferred))
    }

    /// The history before a position, innermost first, back to the latest
    /// line or prompt or to the block start.
    pub fn history(
        &self,
        block_id: &str,
        levels: &[RangePosition],
    ) -> Result<Vec<HistoryItem>, String> {
        let located = self.locate(block_id, levels)?;
        let mut items = Vec::new();
        let mut child: Option<&[Statement]> = None;
        for (list, next) in located.frames.iter().rev() {
            let mut before = &list[..*next];
            if let Some(child) = child {
                // The last statement before the continuation, past any
                // comments, entered the child.
                let Some(index) = before
                    .iter()
                    .rposition(|statement| !matches!(statement, Statement::Comment(_)))
                else {
                    return Err("a frame continues before its branch".to_owned());
                };
                items.push(branch_item(&before[index], child)?);
                before = &before[..index];
            }
            for (index, statement) in before.iter().enumerate().rev() {
                if let Statement::Choice(_) = statement {
                    // A standalone prompt: its whole run of choices.
                    let start = before[..index]
                        .iter()
                        .rposition(|statement| {
                            !matches!(statement, Statement::Choice(_) | Statement::Comment(_))
                        })
                        .map_or(0, |position| position + 1);
                    items.push(HistoryItem::Prompt {
                        line: None,
                        choices: choice_ids(&before[start..=index]),
                    });
                    return Ok(items);
                }
                if let Some(item) = landmark(statement) {
                    items.push(item);
                    return Ok(items);
                }
                match statement {
                    Statement::Effect(effect) => {
                        items.push(HistoryItem::Effect(effect_observation(effect)))
                    }
                    Statement::If(branch) => {
                        items.push(HistoryItem::Passed(expression(&branch.condition)))
                    }
                    Statement::Match(branch) => {
                        items.push(HistoryItem::Passed(call(&branch.scrutinee)))
                    }
                    Statement::Comment(_) => {}
                    other => return Err(format!("unexpected statement in a trail: {other:?}")),
                }
            }
            child = Some(list);
        }
        items.push(HistoryItem::BlockStart(block_id.to_owned()));
        Ok(items)
    }
}

/// The compiled units of a list: each unit's source index and span.
pub(crate) fn units(list: &[Statement]) -> Vec<(usize, SourceSpan)> {
    let mut units = Vec::new();
    let mut in_prompt = false;
    for (index, statement) in list.iter().enumerate() {
        let span = match statement {
            Statement::Comment(_) => continue,
            Statement::Choice(choice) if !in_prompt => {
                in_prompt = true;
                units.push((index, choice.span.clone()));
                continue;
            }
            Statement::Choice(_) => continue,
            Statement::Line(line) => &line.span,
            Statement::Divert(divert) => &divert.span,
            Statement::If(branch) => &branch.span,
            Statement::Match(branch) => &branch.span,
            Statement::Effect(effect) => &effect.span,
        };
        in_prompt = false;
        units.push((index, span.clone()));
    }
    units
}

fn find_list<'a>(list: &'a [Statement], spans: &[SourceSpan]) -> Option<&'a [Statement]> {
    let found = units(list).iter().map(|(_, span)| span).eq(spans.iter());
    if found {
        return Some(list);
    }
    list.iter().find_map(|statement| match statement {
        Statement::If(branch) => find_list(&branch.then_statements, spans)
            .or_else(|| find_list(&branch.else_statements, spans)),
        Statement::Match(branch) => branch
            .arms
            .iter()
            .find_map(|arm| find_list(&arm.statements, spans)),
        _ => None,
    })
}

fn landmark(statement: &Statement) -> Option<HistoryItem> {
    let id = |id: Option<&str>| id.unwrap_or_default().to_owned();
    match statement {
        Statement::Line(line) if line.statements.is_empty() => Some(HistoryItem::Line(id(line
            .id
            .as_ref()
            .map(|id| id.as_str())))),
        Statement::Line(line) => Some(HistoryItem::Prompt {
            line: Some(id(line.id.as_ref().map(|id| id.as_str()))),
            choices: choice_ids(&line.statements),
        }),
        _ => None,
    }
}

fn choice_ids(statements: &[Statement]) -> Vec<String> {
    statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Choice(choice) => choice.id.as_ref().map(|id| id.as_str().to_owned()),
            _ => None,
        })
        .collect()
}

fn branch_item(statement: &Statement, child: &[Statement]) -> Result<HistoryItem, String> {
    let same = |list: &[Statement]| std::ptr::eq(list, child);
    match statement {
        Statement::If(branch) => {
            let taken = if same(&branch.then_statements) {
                "then"
            } else if same(&branch.else_statements) {
                "else"
            } else {
                return Err("a frame entered no branch of its `:if`".to_owned());
            };
            Ok(HistoryItem::Branch(
                expression(&branch.condition),
                taken.to_owned(),
            ))
        }
        Statement::Match(branch) => {
            let arm = branch
                .arms
                .iter()
                .find(|arm| same(&arm.statements))
                .ok_or("a frame entered no arm of its `:match`")?;
            let pattern = match &arm.pattern {
                MatchPattern::Variant(variant) => variant.clone(),
                MatchPattern::Wildcard => "_".to_owned(),
            };
            Ok(HistoryItem::Branch(call(&branch.scrutinee), pattern))
        }
        other => Err(format!("a frame continues after a non-branch: {other:?}")),
    }
}

fn call(call: &recite_core::ast::ConditionCall) -> String {
    let args: Vec<String> = call
        .args
        .iter()
        .map(|argument| format!("{argument:?}"))
        .collect();
    format!("{}({})", call.function, args.join(", "))
}

/// A condition's structure, without spans or grouping parentheses.
fn expression(condition: &ConditionExpression) -> String {
    match condition {
        ConditionExpression::Call(inner) => call(inner),
        ConditionExpression::And(group) => join("and", &group.expressions),
        ConditionExpression::Or(group) => join("or", &group.expressions),
        ConditionExpression::Not(unary) => format!("not({})", expression(&unary.expression)),
        ConditionExpression::Grouped(unary) => expression(&unary.expression),
    }
}

fn join(operator: &str, operands: &[ConditionExpression]) -> String {
    let operands: Vec<String> = operands.iter().map(expression).collect();
    format!("{operator}({})", operands.join(", "))
}
