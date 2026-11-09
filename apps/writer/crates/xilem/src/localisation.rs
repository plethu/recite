//! Frontend views and native glue over `recite_writer_app::localisation`.
use crate::{Phase, Writer};
pub(crate) use recite_writer_app::localisation::*;
use recite_writer_session::localisation::status::TranslationStatus;
use recite_writer_session::{comparison::ComparisonRow, localisation::catalogue::ExternalDecision};
mod beat;
mod keyboard;
pub(crate) mod location;
mod view;
pub(crate) use keyboard::TranslationField;
pub(crate) use view::{close_details, panel};

#[cfg(test)]
pub(crate) mod tests;
