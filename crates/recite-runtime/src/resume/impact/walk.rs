//! Walk a baseline asset's reachable save points and classify each one.

use std::collections::BTreeSet;
use std::sync::Arc;

use recite_core::{
    ChoiceId, SourceSpan,
    compiled::{
        BlockIndex, CompiledDialogue, CompiledEffectMode, CompiledMatchPattern,
        CompiledStatementKind, ContentFingerprint, StatementIndex, StatementRange,
    },
};

use crate::session::{
    PendingEffect, PendingPrompt, PendingPromptChoice, SessionPhase, StatementFrame,
};
use crate::session_snapshot::snapshot_session;
use crate::traversal::{
    AssetView, dialogue_effect_request, runtime_effect_request_for_trace_counter,
};
use crate::{ChoiceAvailability, DialogueError, DialogueSession, DialogueSessionOptions};

use super::super::outcome::{ResumeOptions, SessionResume};
use super::super::relocate::resume_session;
use super::{SavePointImpact, SavePointKind, SavePointOutcome};

pub(super) struct Walker<'a> {
    baseline: &'a CompiledDialogue,
    view: AssetView<'a>,
    candidate: &'a CompiledDialogue,
    candidate_view: AssetView<'a>,
    payload: ContentFingerprint,
    frames: Vec<StatementFrame>,
    points: Vec<SavePointImpact>,
}

/// A save point before it is classified.
struct Point {
    kind: SavePointKind,
    span: SourceSpan,
    range: StatementRange,
    next: StatementIndex,
    phase: SessionPhase,
    prompt_choices: Vec<ChoiceId>,
}

impl<'a> Walker<'a> {
    pub(super) fn new(
        baseline: &'a CompiledDialogue,
        view: AssetView<'a>,
        candidate: &'a CompiledDialogue,
        candidate_view: AssetView<'a>,
        payload: ContentFingerprint,
    ) -> Self {
        Self {
            baseline,
            view,
            candidate,
            candidate_view,
            payload,
            frames: Vec::new(),
            points: Vec::new(),
        }
    }

    pub(super) fn into_points(self) -> Vec<SavePointImpact> {
        self.points
    }
}

impl Walker<'_> {
    pub(super) fn block(&mut self, index: BlockIndex) -> Result<(), DialogueError> {
        let block = self.view.block_at(index)?;
        self.frames.clear();
        self.classify(
            index,
            Point {
                kind: SavePointKind::BlockStart,
                span: self.view.source_map_at(block.source_map)?.span.clone(),
                range: block.statements,
                next: block.statements.start,
                phase: SessionPhase::Running,
                prompt_choices: Vec::new(),
            },
        )?;
        self.range(index, block.statements).map(|_| ())
    }

    /// Walks a range, returning whether traversal always leaves it before its
    /// end, so statements after the branch that holds it are unreachable.
    fn range(&mut self, block: BlockIndex, range: StatementRange) -> Result<bool, DialogueError> {
        for position in self.view.statement_range(range)? {
            let index = StatementIndex::new(u32::try_from(position).unwrap_or(u32::MAX));
            let next = StatementIndex::new(index.as_u32() + 1);
            let statement = self.view.statement_at(index)?;
            let span = self.view.source_map_at(statement.source_map)?.span.clone();
            let point = |kind, phase, prompt_choices| Point {
                kind,
                span: span.clone(),
                range,
                next,
                phase,
                prompt_choices,
            };
            match &statement.kind {
                CompiledStatementKind::Line(line) => {
                    let line = self.view.line_at(*line)?.id.clone();
                    self.classify(
                        block,
                        point(
                            SavePointKind::AfterLine { line },
                            SessionPhase::Running,
                            Vec::new(),
                        ),
                    )?;
                }
                CompiledStatementKind::Prompt { line, choices } => {
                    let line = line
                        .map(|line| self.view.line_at(line).map(|line| line.id.clone()))
                        .transpose()?;
                    let choices = self.view.choices(*choices)?;
                    let ids = choices
                        .iter()
                        .map(|choice| choice.id.clone())
                        .collect::<Vec<_>>();
                    let pending = PendingPrompt {
                        statement: index,
                        choices: choices
                            .iter()
                            .map(|choice| PendingPromptChoice {
                                id: choice.id.clone(),
                                target: choice.target.clone(),
                                availability: ChoiceAvailability::available(),
                            })
                            .collect(),
                        reemit_on_next: false,
                    };
                    self.classify(
                        block,
                        point(
                            SavePointKind::Prompt {
                                line,
                                choices: ids.clone(),
                            },
                            SessionPhase::AwaitingChoice(pending),
                            ids,
                        ),
                    )?;
                    // Every choice leaves for its target, so later statements
                    // in this range are unreachable.
                    return Ok(true);
                }
                CompiledStatementKind::Effect(effect) => {
                    let effect = self.view.effect_at(*effect)?;
                    let function = effect.function.clone();
                    match effect.mode {
                        CompiledEffectMode::Blocking => {
                            let request = runtime_effect_request_for_trace_counter(
                                dialogue_effect_request(self.view, effect)?,
                                1,
                            )?;
                            self.classify(
                                block,
                                point(
                                    SavePointKind::BlockingEffect { function },
                                    SessionPhase::AwaitingEffect(PendingEffect {
                                        statement: index,
                                        request,
                                        reemit_on_next: false,
                                    }),
                                    Vec::new(),
                                ),
                            )?;
                        }
                        CompiledEffectMode::Immediate => self.classify(
                            block,
                            point(
                                SavePointKind::AfterImmediateEffect { function },
                                SessionPhase::Running,
                                Vec::new(),
                            ),
                        )?,
                        CompiledEffectMode::Deferred => {}
                    }
                }
                CompiledStatementKind::If {
                    then_statements,
                    else_statements,
                    ..
                } => {
                    let (then_statements, else_statements) = (*then_statements, *else_statements);
                    let then_leaves = self.nested(block, range, next, then_statements)?;
                    let else_leaves = self.nested(block, range, next, else_statements)?;
                    if then_leaves && else_leaves {
                        return Ok(true);
                    }
                }
                CompiledStatementKind::Match { arms, .. } => {
                    // A match always selects an arm, so it leaves when every
                    // arm it can select does. Like traversal, it selects the
                    // first arm for a variant and the first wildcard, so a
                    // repeated arm never runs.
                    let mut every_arm_leaves = true;
                    let mut variants = BTreeSet::new();
                    let mut wildcard = false;
                    for arm in self.view.match_arms(*arms)? {
                        let first = match &arm.pattern {
                            CompiledMatchPattern::Variant(variant) => variants.insert(variant),
                            CompiledMatchPattern::Wildcard => {
                                !std::mem::replace(&mut wildcard, true)
                            }
                        };
                        if first {
                            every_arm_leaves &= self.nested(block, range, next, arm.statements)?;
                        }
                    }
                    if every_arm_leaves {
                        return Ok(true);
                    }
                }
                // Nothing after a divert or an end in this range can run.
                CompiledStatementKind::Divert(_) | CompiledStatementKind::End => return Ok(true),
            }
        }
        Ok(false)
    }

    fn nested(
        &mut self,
        block: BlockIndex,
        parent: StatementRange,
        continuation: StatementIndex,
        child: StatementRange,
    ) -> Result<bool, DialogueError> {
        self.frames.push(StatementFrame {
            range: parent,
            next_statement: continuation,
        });
        let result = self.range(block, child);
        self.frames.pop();
        result
    }

    fn classify(&mut self, block_index: BlockIndex, point: Point) -> Result<(), DialogueError> {
        let block = self.view.block_at(block_index)?;
        let mut session = DialogueSession::new(
            &self.baseline.header,
            &self.baseline.sources,
            block_index,
            block.statements,
            self.payload.clone(),
            DialogueSessionOptions::new(),
        );
        session.current_range = point.range;
        session.next_statement = point.next;
        session.continuation_stack = self.frames.clone();
        session.trace_counter = 1;
        session.previous_prompt_choices = point.prompt_choices;
        session.phase = point.phase;
        session.deferred_effects = Arc::new(Vec::new());
        let snapshot = snapshot_session(self.baseline, &session)?;
        let outcome = match resume_session(self.candidate, snapshot, &ResumeOptions::new())? {
            SessionResume::Resumed { report, .. } => SavePointOutcome::Resumes {
                level: report.level,
                prompt: report.prompt,
            },
            SessionResume::Fallback(fallback) => SavePointOutcome::Fallback {
                reason: fallback.reason,
                block_available: fallback.block_available,
            },
        };
        let candidate_block_span = match self.candidate_view.lookup_block(block.id.as_str()) {
            Ok(index) => {
                let candidate_block = self.candidate_view.block_at(index)?;
                Some(
                    self.candidate_view
                        .source_map_at(candidate_block.source_map)?
                        .span
                        .clone(),
                )
            }
            Err(DialogueError::UnknownBlock { .. }) => None,
            Err(error) => return Err(error),
        };
        self.points.push(SavePointImpact {
            block_id: block.id.clone(),
            kind: point.kind,
            baseline_span: point.span,
            candidate_block_span,
            outcome,
        });
        Ok(())
    }
}
