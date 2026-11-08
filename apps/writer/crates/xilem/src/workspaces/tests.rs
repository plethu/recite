//! Frontend suites over the shared `recite_writer_app::workspaces::tests` fixtures.
#![allow(unused_imports)]
use super::*;
pub(crate) use crate::tests::support;
use crate::{Phase, Writer};
pub(crate) use recite_writer_app::workspaces::tests::*;
#[path = "builds/tests.rs"]
mod builds;
#[path = "visual/tests.rs"]
mod visual;
