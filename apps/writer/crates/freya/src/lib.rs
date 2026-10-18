mod activation;
mod builds;
mod commands;
mod declarations;
mod document_tabs;
mod external;
mod presentation;
mod rename;
mod rules;
mod source_editor;
mod startup;
pub use activation::{Activation, ActivationHost, ActivationInbox};
pub use startup::{InitialProject, Startup};
pub fn startup_help() -> &'static str {
    startup::HELP
}
mod beat_heading;
mod branch_preview;
mod buffers;
mod chrome;
mod closing;
mod controls;
mod design;
mod editing;
mod examples;
mod files;
mod new_project;
use design::palette;
mod passage_menu;
mod project;
mod project_context;
mod project_loading;
mod project_search;
mod reading_context;
mod recovery;
mod route_editor;
pub use closing::request_close;
mod field;
mod localisation;
mod messages;
mod navigation;
pub use navigation::InitialRoute;
mod preferences;
mod preview_panel;
mod prose;
mod scene;
mod scene_map;
mod scene_navigation;
mod script_entries;
mod settings;
mod sidebar;
mod workbench;
mod workspace;

use freya::prelude::Element;

#[derive(Clone, Copy, PartialEq)]
enum AppMode {
    Examples,
    Regression,
    Project,
}

pub fn app() -> Element {
    navigation::app(AppMode::Examples)
}

/// Optional source for embedding and repeatable headless workloads.
#[derive(Clone)]
pub struct InitialSource(pub String);

/// The original Alice scene retained for interaction regressions.
pub fn regression_app() -> Element {
    navigation::app(AppMode::Regression)
}

/// Opens the file-backed writer with project discovery and recovery.
pub fn editor_app() -> Element {
    navigation::app(AppMode::Project)
}

mod feedback;

mod diagnostics;

mod field_completion;

mod field_actions;

/// Native production-component specimen; does not open files or save preferences.
pub fn design_app() -> Element {
    design::specimen::app()
}
