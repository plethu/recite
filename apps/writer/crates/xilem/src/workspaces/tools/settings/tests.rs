//! Frontend suites over the shared `recite_writer_app::workspaces::tools::settings::tests` fixtures.
#![allow(unused_imports)]
use super::super::*;
use super::*;
pub(crate) use crate::tests::support;
use crate::{
    external_editor::Request as EditorRequest,
    workspaces::{ContextUpdate, text},
};
pub(crate) use recite_writer_app::workspaces::tools::settings::tests::fixture;
pub(crate) use recite_writer_app::workspaces::tools::settings::tests::*;
use recite_writer_session::project::settings::{
    SettingsAccepted, SettingsAction, SettingsCommit, SettingsOperation, SettingsReceipt,
};
#[path = "visual/tests.rs"]
mod visual;
#[path = "workflow/tests.rs"]
mod workflow;
