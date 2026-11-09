//! Frontend suites over the shared `recite_writer_app::notices::tests` fixtures.
#![allow(unused_imports)]
use super::*;
pub(crate) use crate::tests::support;
use crate::{
    Phase, Writer,
    localisation::{WatchObservation, WatchRequest, WatchScope},
};
pub(crate) use recite_writer_app::notices::tests::*;
use recite_writer_session::{
    external::{Affected, Changes},
    project::{ExternalCheck, ExternalObservation},
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::{mpsc, watch};
#[path = "catalogue/tests.rs"]
mod catalogue;
#[path = "visual/tests.rs"]
mod visual;
