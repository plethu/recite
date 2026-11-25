//! Describe a session position through authored IDs and statement signatures.

use recite_core::compiled::{
    CompiledMatchPattern, CompiledStatementKind, StatementIndex, StatementRange,
};
use serde::Serialize;

use crate::DialogueError;
use crate::session::StatementFrame;
use crate::session_snapshot::{
    DialogueBranchSnapshot, DialoguePositionAnchorSnapshot, DialogueResumeAnchorSnapshot,
    DialogueResumeFrameSnapshot,
};
use crate::traversal::{AssetView, malformed};

use super::key::{Landmark, Signature, StatementKey, statement_key};

/// Mirrors [`DialogueResumeAnchorSnapshot`].
#[derive(Debug, Serialize)]
pub(crate) struct ResumeAnchor<'a> {
    pub(crate) frames: Vec<FrameAnchor<'a>>,
    pub(crate) position: PositionAnchor<'a>,
}

/// Mirrors [`DialogueResumeFrameSnapshot`].
#[derive(Debug, Serialize)]
pub(crate) struct FrameAnchor<'a> {
    pub(crate) position: PositionAnchor<'a>,
    pub(crate) branch: Branch<'a>,
}

/// Mirrors [`DialogueBranchSnapshot`].
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Branch<'a> {
    Then,
    Else,
    Arm {
        variant: Option<&'a str>,
        occurrence: u32,
    },
}

/// Mirrors [`DialoguePositionAnchorSnapshot`].
#[derive(Debug, Serialize)]
pub(crate) struct PositionAnchor<'a> {
    pub(crate) landmark: Option<Landmark<'a>>,
    pub(crate) trail: Vec<Signature<'a>>,
}

/// Anchors a position: the continuation frames from the block's root range,
/// the active range and the next statement within it.
pub(crate) fn resume_anchor<'a>(
    asset: AssetView<'a>,
    frames: &[StatementFrame],
    current_range: StatementRange,
    next_statement: StatementIndex,
) -> Result<ResumeAnchor<'a>, DialogueError> {
    let mut anchors = Vec::with_capacity(frames.len());
    for (index, frame) in frames.iter().enumerate() {
        let child = frames
            .get(index + 1)
            .map_or(current_range, |next| next.range);
        anchors.push(FrameAnchor {
            position: position_anchor(asset, frame.range, frame.next_statement)?,
            branch: branch(asset, frame.next_statement, child)?,
        });
    }
    Ok(ResumeAnchor {
        frames: anchors,
        position: position_anchor(asset, current_range, next_statement)?,
    })
}

fn position_anchor(
    asset: AssetView<'_>,
    range: StatementRange,
    next_statement: StatementIndex,
) -> Result<PositionAnchor<'_>, DialogueError> {
    let start = range.start.as_u32();
    let next = next_statement.as_u32();
    let mut landmark = None;
    let mut trail_start = start;
    for index in (start..next).rev() {
        if let StatementKey::Landmark(found) = statement_key(asset, StatementIndex::new(index))? {
            landmark = Some(found);
            trail_start = index + 1;
            break;
        }
    }
    let mut trail = Vec::with_capacity((next - trail_start) as usize);
    for index in trail_start..next {
        match statement_key(asset, StatementIndex::new(index))? {
            StatementKey::Signature(signature) => trail.push(signature),
            StatementKey::Landmark(_) => {
                return Err(malformed(
                    "statement trail contains a landmark after the latest landmark".to_owned(),
                ));
            }
        }
    }
    Ok(PositionAnchor { landmark, trail })
}

fn branch(
    asset: AssetView<'_>,
    continuation: StatementIndex,
    child: StatementRange,
) -> Result<Branch<'_>, DialogueError> {
    let branch_statement = continuation
        .as_u32()
        .checked_sub(1)
        .map(StatementIndex::new)
        .ok_or_else(|| malformed("continuation frame resumes before statement zero".to_owned()))?;
    match &asset.statement_at(branch_statement)?.kind {
        CompiledStatementKind::If {
            then_statements,
            else_statements,
            ..
        } => {
            if *then_statements == child {
                Ok(Branch::Then)
            } else if *else_statements == child {
                Ok(Branch::Else)
            } else {
                Err(malformed(
                    "continuation frame does not enter a branch of its `if`".to_owned(),
                ))
            }
        }
        CompiledStatementKind::Match { arms, .. } => {
            let arms = asset.match_arms(*arms)?;
            let Some(position) = arms.iter().position(|arm| arm.statements == child) else {
                return Err(malformed(
                    "continuation frame does not enter an arm of its `match`".to_owned(),
                ));
            };
            let pattern = &arms[position].pattern;
            let occurrence = arms[..position]
                .iter()
                .filter(|arm| arm.pattern == *pattern)
                .count();
            Ok(Branch::Arm {
                variant: match pattern {
                    CompiledMatchPattern::Variant(variant) => Some(variant.as_str()),
                    CompiledMatchPattern::Wildcard => None,
                },
                occurrence: u32::try_from(occurrence).unwrap_or(u32::MAX),
            })
        }
        _ => Err(malformed(
            "continuation frame does not follow a branching statement".to_owned(),
        )),
    }
}

impl ResumeAnchor<'_> {
    pub(crate) fn to_snapshot(&self) -> DialogueResumeAnchorSnapshot {
        DialogueResumeAnchorSnapshot {
            frames: self
                .frames
                .iter()
                .map(|frame| DialogueResumeFrameSnapshot {
                    position: frame.position.to_snapshot(),
                    branch: frame.branch.to_snapshot(),
                })
                .collect(),
            position: self.position.to_snapshot(),
        }
    }
}

impl PositionAnchor<'_> {
    fn to_snapshot(&self) -> DialoguePositionAnchorSnapshot {
        DialoguePositionAnchorSnapshot {
            landmark: self.landmark.map(Landmark::to_snapshot),
            trail: self.trail.iter().map(Signature::to_snapshot).collect(),
        }
    }
}

impl Branch<'_> {
    fn to_snapshot(self) -> DialogueBranchSnapshot {
        match self {
            Self::Then => DialogueBranchSnapshot::Then,
            Self::Else => DialogueBranchSnapshot::Else,
            Self::Arm {
                variant,
                occurrence,
            } => DialogueBranchSnapshot::Arm {
                variant: variant.map(str::to_owned),
                occurrence,
            },
        }
    }
}
