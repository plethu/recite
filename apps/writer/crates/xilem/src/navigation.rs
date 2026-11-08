//! Frontend views and native glue over `recite_writer_app::navigation`.
use super::Writer;
pub(crate) use recite_writer_app::navigation::*;
pub(super) mod repair;
pub(super) mod worker;
pub(crate) use worker::worker;

#[cfg(test)]
pub(crate) mod tests;
