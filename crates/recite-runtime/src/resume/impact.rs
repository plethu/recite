//! Classify every save point of a shipped asset against a patched one.
//!
//! The analysis saves a session at each host-observable boundary of the
//! baseline and resumes it against the candidate through [`resume_session`],
//! so tooling reports exactly what players' saves will do.

use std::sync::Arc;

use recite_core::{
    BlockId, ChoiceId, LineId, SourceSpan,
    compiled::{
        BlockIndex, CompiledDialogue, CompiledEffectMode, CompiledStatementKind, StatementIndex,
        StatementRange,
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

use super::outcome::{
    PromptResume, ResumeFallbackReason, ResumeLevel, ResumeOptions, SessionResume,
};
use super::relocate::resume_session;

/// Every save point in a baseline asset, classified against a candidate.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct SaveImpactReport {
    /// The candidate is byte-for-byte the baseline payload.
    pub identical: bool,
    pub schema_changed: bool,
    pub compiler_changed: bool,
    /// Save points in baseline block-table order, then source order.
    pub points: Vec<SavePointImpact>,
}

/// Counts of save points by outcome.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SaveImpactSummary {
    pub total: usize,
    pub exact: usize,
    pub unchanged_block: usize,
    pub text_edited: usize,
    pub relocated: usize,
    /// Resumed points whose prompt is evaluated again from patched content.
    pub reevaluated_prompts: usize,
    pub fallback: usize,
}

impl SaveImpactReport {
    #[must_use]
    pub fn summary(&self) -> SaveImpactSummary {
        let mut summary = SaveImpactSummary {
            total: self.points.len(),
            ..SaveImpactSummary::default()
        };
        for point in &self.points {
            match &point.outcome {
                SavePointOutcome::Resumes { level, prompt } => {
                    match level {
                        ResumeLevel::Exact => summary.exact += 1,
                        ResumeLevel::UnchangedBlock => summary.unchanged_block += 1,
                        ResumeLevel::TextEdited => summary.text_edited += 1,
                        ResumeLevel::Relocated => summary.relocated += 1,
                    }
                    if *prompt == PromptResume::Reevaluated {
                        summary.reevaluated_prompts += 1;
                    }
                }
                SavePointOutcome::Fallback { .. } => summary.fallback += 1,
            }
        }
        summary
    }

    pub fn fallbacks(&self) -> impl Iterator<Item = &SavePointImpact> {
        self.points
            .iter()
            .filter(|point| matches!(point.outcome, SavePointOutcome::Fallback { .. }))
    }
}

/// One place a player can save, and what that save does after the patch.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct SavePointImpact {
    pub block_id: BlockId,
    pub kind: SavePointKind,
    /// Where the save point is in the baseline source.
    pub baseline_span: SourceSpan,
    /// The saved block's header in the candidate source, when it still exists.
    pub candidate_block_span: Option<SourceSpan>,
    pub outcome: SavePointOutcome,
}

/// The host-observable boundary a save was taken at.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SavePointKind {
    BlockStart,
    AfterLine {
        line: LineId,
    },
    Prompt {
        line: Option<LineId>,
        choices: Vec<ChoiceId>,
    },
    BlockingEffect {
        function: String,
    },
    AfterImmediateEffect {
        function: String,
    },
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SavePointOutcome {
    Resumes {
        level: ResumeLevel,
        prompt: PromptResume,
    },
    Fallback {
        reason: ResumeFallbackReason,
        block_available: bool,
    },
}

/// Classifies every save point of `baseline` against `candidate`.
///
/// Both assets must share an asset ID. The result does not depend on game
/// state: prompts are saved with every choice available, and saved
/// availability does not affect where a save resumes.
pub fn analyze_save_impact(
    baseline: &CompiledDialogue,
    candidate: &CompiledDialogue,
) -> Result<SaveImpactReport, DialogueError> {
    let baseline_view = AssetView::new(baseline)?;
    let candidate_view = AssetView::new(candidate)?;
    let payload = baseline
        .content_fingerprint()
        .map_err(|error| DialogueError::MalformedCompiledAsset {
            reason: error.to_string(),
        })?
        .clone();
    let identical = candidate.content_fingerprint().ok() == Some(&payload);
    let mut walker = Walker {
        baseline,
        view: baseline_view,
        candidate,
        candidate_view,
        payload,
        frames: Vec::new(),
        points: Vec::new(),
    };
    for index in 0..baseline.blocks.len() {
        walker.block(BlockIndex::new(u32::try_from(index).unwrap_or(u32::MAX)))?;
    }
    Ok(SaveImpactReport {
        identical,
        schema_changed: baseline.header.schema_fingerprint != candidate.header.schema_fingerprint,
        compiler_changed: baseline.header.compiler_version != candidate.header.compiler_version,
        points: walker.points,
    })
}

struct Walker<'a> {
    baseline: &'a CompiledDialogue,
    view: AssetView<'a>,
    candidate: &'a CompiledDialogue,
    candidate_view: AssetView<'a>,
    payload: recite_core::compiled::ContentFingerprint,
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

impl Walker<'_> {
    fn block(&mut self, index: BlockIndex) -> Result<(), DialogueError> {
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
        self.range(index, block.statements)
    }

    fn range(&mut self, block: BlockIndex, range: StatementRange) -> Result<(), DialogueError> {
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
                    self.nested(block, range, next, then_statements)?;
                    self.nested(block, range, next, else_statements)?;
                }
                CompiledStatementKind::Match { arms, .. } => {
                    for arm in self.view.match_arms(*arms)? {
                        self.nested(block, range, next, arm.statements)?;
                    }
                }
                CompiledStatementKind::Divert(_) | CompiledStatementKind::End => {}
            }
        }
        Ok(())
    }

    fn nested(
        &mut self,
        block: BlockIndex,
        parent: StatementRange,
        continuation: StatementIndex,
        child: StatementRange,
    ) -> Result<(), DialogueError> {
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
