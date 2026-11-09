//! Frontend suites over the shared `recite_writer_app::workspaces::tools::tests` fixtures.
#![allow(unused_imports)]
use super::super::{Phase, Writer, declarations::Row};
use super::*;
pub(crate) use crate::tests::support;
pub(crate) use recite_writer_app::workspaces::tools::tests::*;
use recite_writer_session::{
    declarations::{
        DeclarationAction, DeclarationFinish, DeclarationFinished, DeclarationReceipt,
        DeclarationWork, Session, entries,
        session::{GenerationReceipt, GenerationStart, GenerationStarted, GenerationWork},
    },
    project::{
        PreparedRefresh, RefreshRequest,
        settings::{SettingsPublication, SettingsWork, SettingsWritten},
    },
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
#[path = "visual/tests.rs"]
mod visual;
