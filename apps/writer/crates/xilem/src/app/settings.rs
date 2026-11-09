//! Settings views over `recite_writer_app::shell::settings`.
use super::*;
use recite_config::UserConfigEdit;
pub(crate) use recite_writer_app::shell::settings::*;
mod view;
pub(super) use view::settings;
