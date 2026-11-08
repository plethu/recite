//! Frontend views and native glue over `recite_writer_app::map`.
use crate::{Writer, navigation::Target};
pub(crate) use recite_writer_app::map::*;
mod input;
mod overview;
mod placed;
mod routes;
mod view;
pub(super) use view::panel;

#[cfg(test)]
pub(crate) mod tests;
