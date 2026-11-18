//! Frontend views and native glue over `recite_writer_app::workspaces::rename`.
use super::Writer;
pub(crate) mod inline;
mod view;
pub(super) use view::panel;
pub(crate) use view::recovery;
