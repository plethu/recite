//! Frontend suites over the shared `recite_writer_app::map::tests` fixtures.
#![allow(unused_imports)]
use super::*;
pub(crate) use crate::tests::support;
use crate::{Writer, navigation::Target};
use kurbo::{BezPath, Point};
use recite_core::DocumentKey;
pub(crate) use recite_writer_app::map::tests::*;
use recite_writer_session::map::synopsis;
use recite_writer_session::map::{
    camera::{Camera, Framing},
    layout::Node,
    navigation,
    placement::Positions,
    scope,
    topology::Topology,
};
use std::collections::btree_map;
use std::{collections::BTreeMap, sync::Arc};
#[path = "bounds/tests.rs"]
mod bounds;
#[path = "native/tests.rs"]
mod native;
#[path = "overview/tests.rs"]
mod overview;
#[path = "panning/tests.rs"]
mod panning;
#[path = "visual/tests.rs"]
mod visual;
#[path = "workflow/tests.rs"]
mod workflow;
