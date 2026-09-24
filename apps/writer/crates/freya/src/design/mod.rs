//! Writer design system: semantic tokens and shared interaction primitives.
mod button;
pub(crate) mod tokens;
pub(crate) use button::Button;

mod checkbox;
mod dialog;
mod focus_scroll;
pub(crate) use checkbox::checkbox;
pub(crate) use dialog::{Dialog, ModalState, actions, modal_open};

mod splitter;
pub(crate) use splitter::Splitter;

mod reveal;
pub(crate) use reveal::Reveal;
pub(crate) mod palette;

mod options;
pub(crate) use options::{Options, Segments};

pub(crate) mod keyboard;
mod submit_action;
pub(crate) use submit_action::SubmitAction;
mod search_picker;
pub(crate) use search_picker::{PickerOption, SearchPicker};

mod search_field;
pub(crate) use search_field::SearchField;

mod path_field;
pub(crate) use path_field::{PathField, PathKind};

mod comparison;
pub(crate) use comparison::{ComparisonRow, ComparisonView};

mod motion;
pub(crate) use motion::ReducedMotion;

mod beat_card;
pub(crate) use beat_card::{BeatCard, DIALOGUE_ZOOM};

pub(crate) mod specimen;

mod material;

mod search_scroll;
pub(crate) use search_scroll::use_list_reveal;

pub(crate) mod list_keys;
