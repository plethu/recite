//! Frontend views and native glue over `recite_writer_app::workspaces::external`.
use super::Writer;
pub(crate) use recite_writer_app::workspaces::external::*;
#[cfg(test)]
mod tests;
mod view;
pub(super) use view::panel;
