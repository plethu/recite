//! Property-test support for Recite: generated programs with a matching
//! schema, patches between program revisions, and a reference interpreter
//! that compiled traversal is checked against.
//!
//! [`program()`] generates valid programs, [`patch()`] edits them the way a
//! game patch edits dialogue, [`ReferenceProgram`] runs their lowered source
//! AST directly, and [`drive`] plays any [`Dialogue`] with a fixed choice
//! policy so runs of different implementations can be compared step by step.
//! This crate is not published.

mod answers;
mod model_check;
mod normalize;
mod observation;
mod oracle;
mod patch;
mod position;
mod program;
mod render;
mod save_points;
mod strategy;

pub use answers::{Answer, Answers, answers};
pub use observation::{
    Action, ChoiceObservation, Dialogue, EffectObservation, LineObservation, Observation, Outcome,
    Rejection, Step, drive, observations,
};
pub use oracle::{OracleSession, ReferenceProgram};
pub use patch::{PatchOp, apply_patch, patch};
pub use position::{HistoryItem, RangePosition};
pub use program::{
    ASSET_ID, Actor, ArmSpec, BlockSpec, ChoiceSpec, Cond, EffectMode, EffectSpec, FileSpec,
    LineSpec, Program, SCHEMA_JSON, SCHEMA_PATH, Scrutinee, Stmt, Target,
};
pub use render::{call, cond, line_text, scrutinee_call};
pub use save_points::SavePointKey;
pub use strategy::program;
