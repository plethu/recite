//! Frontend views and native glue over `recite_writer_app::external_editor`.
use super::Writer;
pub(crate) use recite_writer_app::external_editor::*;
use std::sync::Arc;
#[cfg(test)]
pub(crate) mod tests;
mod worker;
pub(super) use worker::worker;
