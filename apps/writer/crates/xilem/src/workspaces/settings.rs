//! Frontend views and native glue over `recite_writer_app::workspaces::settings`.
use super::Writer;
use recite_writer_session::project::settings::Session;
mod view;
pub(super) use view::panel;
