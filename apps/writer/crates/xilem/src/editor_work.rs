//! Frontend views and native glue over `recite_writer_app::editor_work`.
use super::Writer;
pub(crate) use recite_writer_app::editor_work::*;
use recite_writer_session::{FileError, project::save::sequence::Work};
#[cfg(test)]
mod tests;
mod worker;
pub(super) use worker::worker;
