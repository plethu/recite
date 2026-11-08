//! Frontend views and native glue over `recite_writer_app::rules`.
use crate::Writer;
use recite_core::ast::EffectMode;
pub(crate) use recite_writer_app::rules::*;
use recite_writer_model::{ReplyRules, RuleArgument, RuleExpression};
use rows::Row;
pub(super) mod view;

#[cfg(test)]
pub(crate) mod tests;
