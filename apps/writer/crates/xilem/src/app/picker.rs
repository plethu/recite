//! Picker views over `recite_writer_app::shell::picker`.
use super::*;
pub(crate) use recite_writer_app::shell::picker::*;
pub(super) mod view;
pub(crate) use view::{ActiveResult, QueryField};

#[cfg(test)]
mod tests;
