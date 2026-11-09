//! Frontend suites over the shared `recite_writer_app::localisation::tests` fixtures.
#![allow(unused_imports)]
use super::*;
pub(crate) use crate::tests::support;
use crate::{
    Phase, Writer,
    editor_work::AfterSave as SourceAfterSave,
    workspaces::{Screen, text},
};
use recite_core::po::PoEntryId;
pub(crate) use recite_writer_app::localisation::tests::*;
use recite_writer_session::localisation::status::TranslationStatus;
use recite_writer_session::{
    comparison::ComparisonRow,
    localisation::catalogue::{Catalogue, EntryDetails, ExternalDecision, Inspection},
};
use slot::{Identity, Slot};
use std::{path::PathBuf, sync::Arc};
#[path = "source_views/tests.rs"]
mod source_views;
#[path = "visual/tests.rs"]
mod visual;
#[path = "workflow/tests.rs"]
mod workflow;
