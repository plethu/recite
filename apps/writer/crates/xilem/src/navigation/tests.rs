//! Frontend suites over the shared `recite_writer_app::navigation::tests` fixtures.
#![allow(unused_imports)]
use super::super::{Writer, feedback, session::Session};
use super::*;
pub(crate) use crate::tests::support;
use crate::{attention::EditingTarget, workspaces::Screen as WorkspaceScreen};
use recite_compiler::authoring::{CancellationToken, Interrupted, WorkControl};
pub(crate) use recite_writer_app::navigation::tests::*;
use recite_writer_model::StructuralCommand;
use recite_writer_model::{EditOperation, View};
use recite_writer_session::FileError;
use recite_writer_session::location::history as location_history;
use recite_writer_session::location::{Location, Screen as LinkedScreen};
use std::sync::atomic;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
#[path = "repair/tests.rs"]
mod repair;
#[path = "visual/tests.rs"]
mod visual;
