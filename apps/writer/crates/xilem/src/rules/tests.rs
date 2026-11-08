//! Frontend suites over the shared `recite_writer_app::rules::tests` fixtures.
#![allow(unused_imports)]
use super::*;
pub(crate) use crate::tests::support;
use crate::{Phase, Writer};
use recite_core::ast::EffectMode;
pub(crate) use recite_writer_app::rules::tests::*;
use recite_writer_model::{ReplyRules, RuleArgument, RuleExpression};
use rows::Row;
use std::sync::Arc;
#[path = "bounds/tests.rs"]
mod bounds;
#[path = "visual/tests.rs"]
mod visual;
