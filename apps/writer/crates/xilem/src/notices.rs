//! Frontend views and native glue over `recite_writer_app::notices`.
use crate::Writer;
pub(crate) use recite_writer_app::notices::*;
mod view;
mod worker;
pub(crate) use view::panel;
pub(crate) use worker::worker;

#[cfg(test)]
pub(crate) mod tests;
