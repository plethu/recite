//! Frontend suites over the shared `recite_writer_app::workspaces::tools::project::tests` fixtures.
#![allow(unused_imports)]
use super::super::*;
use super::*;
pub(crate) use crate::tests::support;
use crate::{
    session::{Checkout, RecoveryReason},
    workspaces::{
        external::{Inspection, Resolved},
        rename, text,
    },
};
use recite_compiler::authoring::WorkControl;
pub(crate) use recite_writer_app::workspaces::tools::project::tests::fixture;
pub(crate) use recite_writer_app::workspaces::tools::project::tests::*;
use recite_writer_session::{FileError, ProjectSession};
use std::path::PathBuf;
#[path = "external/tests.rs"]
mod external;
#[path = "visual/tests.rs"]
mod visual;
