//! Frontend suites over the shared `recite_writer_app::localisation::location::tests` fixtures.
#![allow(unused_imports)]
use super::super::{
    presentation::Presentation,
    slot::{Checkout, Slot},
    *,
};
use super::*;
pub(crate) use crate::tests::support;
pub(crate) use recite_writer_app::localisation::location::tests::*;
use recite_writer_session::{
    ProjectSession,
    location::{Location, Screen as LinkedScreen},
};
use std::path::Path;
#[path = "visual/tests.rs"]
mod visual;
