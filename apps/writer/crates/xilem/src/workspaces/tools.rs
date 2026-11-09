//! Frontend views and native glue over `recite_writer_app::workspaces::tools`.
use super::Writer;
pub(crate) use recite_writer_app::workspaces::tools::*;
mod declaration;
mod project;
mod settings;
mod worker;
pub(crate) use worker::worker;

#[cfg(test)]
pub(crate) mod tests;
