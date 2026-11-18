//! Frontend views and native glue over `recite_writer_app::workspaces::builds`.
use super::*;
mod result;
mod view;
pub(crate) use result::result;
pub(super) use view::panel;
