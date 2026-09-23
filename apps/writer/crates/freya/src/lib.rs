mod builds;
mod commands;
mod declarations;
mod document_tabs;
mod external;
mod presentation;
mod rename;
mod rules;
mod source_editor;
use crate::design::tokens as t;
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
mod workspace;

use editing::editor_data;
use freya::prelude::*;
use recite_writer_model::{FIXTURE, View, WRITER_EXAMPLES, Workbench};

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

fn workbench(mode: AppMode) -> Element {
    let initial = use_try_consume::<InitialSource>();
    let mut model = use_state(move || {
        if mode == AppMode::Regression
            && let Some(initial) = initial
        {
            return Workbench::new(&initial.0);
        }
        if mode == AppMode::Examples {
            WRITER_EXAMPLES[0].open()
        } else {
            Workbench::new(FIXTURE)
        }
    });
    let store = use_try_consume::<Result<recite_config::UserConfigStore, String>>();
    let preferences = use_state(move || preferences::Preferences::load(store));
    let mut reduced_motion = use_state(|| preferences.peek().config.writer.reduced_motion);
    use_provide_context(|| design::ReducedMotion(reduced_motion));
    use_side_effect(move || {
        reduced_motion.set_if_modified(preferences.read().config.writer.reduced_motion)
    });
    use_provide_context(move || presentation::Typography(preferences));
    let mut shortcut_modifiers = use_state(Modifiers::empty);
    use_provide_context(|| commands::Hints(shortcut_modifiers));
    use_side_effect(move || {
        if !*Platform::get().is_app_focused.read() {
            shortcut_modifiers.set_if_modified(Modifiers::empty());
        }
    });
    let mut previous_monochrome = use_state(|| preferences.peek().config.writer.monochrome);
    let initially_dark = preferences.peek().config.writer.theme == recite_config::WriterTheme::Dark;
    let mut dark = use_state(move || initially_dark);
    let mut theme = use_init_theme(move || palette::theme(initially_dark));
    let settings_open = use_state(|| false);
    let modal_count = use_state(|| 0usize);
    use_provide_context(move || design::ModalState(modal_count));
    let map_focus = use_a11y();
    let inspector_focus = use_a11y();
    let sidebar_focus = use_a11y();
    let search_focus = use_a11y();
    let selection = use_state(|| None::<String>);
    let search = use_state(String::new);
    let message = crate::feedback::Feedback::new();
    let editor = use_state(move || {
        let state = model.peek();
        editor_data(
            state.as_ref().map_or("", Workbench::draft),
            state.as_ref().is_ok_and(|m| m.view() == &View::Source),
            false,
        )
    });
    let prose = use_state(move || {
        model
            .peek()
            .as_ref()
            .map_or("", Workbench::draft)
            .to_owned()
    });
    use_side_effect(move || {
        let draft = if model
            .peek()
            .as_ref()
            .is_ok_and(|m| matches!(m.view(), View::Source))
        {
            editor.read().rope.to_string()
        } else {
            prose.read().clone()
        };
        let differs = model
            .peek()
            .as_ref()
            .is_ok_and(|session| session.draft() != draft);
        if differs && let Ok(session) = model.write().as_mut() {
            session.set_draft(draft);
        }
    });
    let editor_id = use_a11y();
    let rules = use_state(|| None);
    let buffers = buffers::Buffers {
        bookmarks: buffers::Bookmarks::new(),
        rules,
        model,
        editor,
        prose,
    };
    use_side_effect(move || {
        let next = preferences.read().config.writer.theme == recite_config::WriterTheme::Dark;
        let monochrome = preferences.read().config.writer.monochrome;
        if next != *dark.peek() || monochrome != *previous_monochrome.peek() {
            previous_monochrome.set(monochrome);
            dark.set(next);
            theme.set(palette::theme(next));
        }
    });
    let night = *dark.read();
    let scroll = use_scroll_controller(ScrollConfig::default);
    let pane = use_state(|| editing::Pane::Map);
    let mut previous_pane = use_state(|| None);
    use_after_side_effect(move || {
        let current = *pane.read();
        let previous = *previous_pane.peek();
        previous_pane.set_if_modified(Some(current));
        if previous.is_some_and(|pane| pane != current) {
            if current == editing::Pane::Script {
                inspector_focus.request_focus();
            } else if current == editing::Pane::Map {
                if model
                    .peek()
                    .as_ref()
                    .is_ok_and(|m| m.view() == &View::Source)
                {
                    editor_id.request_focus();
                } else {
                    map_focus.request_focus();
                }
            }
        }
    });
    let expanded = use_state(std::collections::BTreeSet::new);
    let files = use_state(|| None);
    let examples = use_state(std::collections::BTreeMap::new);
    let queue = navigation::Queue {
        search: use_state(String::new),
        attention: use_state(|| false),
        page: use_state(|| 0),
    };
    let reference = use_state(|| None::<reading_context::Reference>);
    let localisation = use_state(localisation::Localisation::default);
    let writer = editing::Writer {
        vim: commands::Navigation::new(),
        command_search: commands::Search::new(),
        source_viewport: source_editor::EditorViewport::new(),
        layout: presentation::Layout::new(preferences),
        trial: preview_panel::TrialInputs::new(),
        localisation,
        files,
        examples,
        queue,
        reference,
        scroll,
        pane,
        expanded,
        buffers,
        message,
        dark: night,
        preferences,
        settings_open,
        map_focus,
        sidebar_focus,
        inspector_focus,
        search_focus,
        selection,
        search,
    };
    use_after_side_effect(move || {
        if *modal_count.read() > 0
            || writer.preferences.read().config.ui.keymap != recite_config::Keymap::Vim
            || *writer.settings_open.read()
            || writer.localisation.read().modal_open()
            || writer.command_search.mode.read().is_some()
        {
            writer.vim.cancel();
        }
    });
    let navigation_visible = writer.layout.navigation;
    let file_chrome = (mode == AppMode::Project).then(|| files::controls(writer, message, night));
    let example_scenes = (mode == AppMode::Examples).then(|| examples::navigation(writer, message));
    let empty_files = use_state(|| None);
    use_hook(move || {
        let _ = writer.scene_opened();
    });
    navigation::track(writer, mode);
    let close_prompt = closing::controls(
        writer,
        file_chrome
            .as_ref()
            .map_or(empty_files, |chrome| chrome.files),
    );
    let state = model.read();
    let session = match state.as_ref() {
        Ok(session) => session,
        Err(error) => return label().text(error.to_string()).into_element(),
    };
    let source = session.view() == &View::Source;
    let colors = theme.read().colors.clone();
    let scene_name = palette::display_name(session.document().key().as_str());
    let scenes = if let Some(chrome) = &file_chrome {
        chrome.scenes.clone()
    } else if let Some(scenes) = example_scenes {
        scenes
    } else {
        scene_navigation::SceneNavigation {
            writer,
            scenes: vec![scene_navigation::SceneBranch {
                caption: scene_name.clone(),
                active: true,
                open: EventHandler::new(|()| {}),
            }],
        }
        .into_element()
    };
    let toolbar = chrome::toolbar(
        writer,
        file_chrome.as_ref().map(|chrome| chrome.actions.clone()),
    );
    let map_identity = file_chrome
        .as_ref()
        .and_then(|chrome| {
            chrome
                .files
                .read()
                .as_ref()
                .map(|files| format!("file:{:?}", files.current))
        })
        .unwrap_or_else(|| format!("example:{}", session.document().key().as_str()));
    let map = scene_map::SceneMap {
        writer,
        scene: map_identity,
    }
    .into_element();
    let active = field::render(writer, editor_id);
    let mut reading = rect()
        .width(Size::fill())
        .padding((design::tokens::SPACE_XS, design::tokens::SPACE_SM))
        .spacing(t::SPACE_SM)
        .child(scene::reading_surface(writer, active.clone()));
    let diagnostics = session.document().diagnostics();
    for diagnostic in &diagnostics {
        reading = reading.child(diagnostics::row(writer, editor_id, diagnostic));
    }
    let body = rect()
        .content(Content::Flex)
        .horizontal()
        .width(Size::fill())
        .height(Size::flex(1.))
        .child(workspace::Workspace {
            writer,
            files: file_chrome.as_ref().map_or(empty_files, |c| c.files),
            source,
            navigation_visible,
            scenes,
            toolbar,
            map,
            reading: reading.into_element(),
            active,
        });
    let saved = file_chrome
        .as_ref()
        .map_or("Temporary example", |chrome| chrome.status.as_str());
    let diagnostic_status = if diagnostics.is_empty() {
        "No diagnostics".into()
    } else {
        format!("{} diagnostics", diagnostics.len())
    };
    let preview_status = if session.preview_page().is_some() && session.preview_stale() {
        " · Preview is out of date"
    } else {
        ""
    };
    let notice_in_localisation = {
        let localisation = writer.localisation.read();
        localisation.modal_open()
            || (localisation.active && localisation.view == localisation::CatalogueView::Updates)
    };
    let mut root = t::interface()
        .expanded()
        .on_key_down(move |event: Event<KeyboardEventData>| commands::vim_keyboard(writer, &event))
        .on_global_pointer_down(move |_| writer.vim.cancel())
        .on_global_key_up(move |event: Event<KeyboardEventData>| {
            shortcut_modifiers.set_if_modified(event.modifiers);
        })
        .on_global_key_down(move |event: Event<KeyboardEventData>| {
            shortcut_modifiers.set_if_modified(event.modifiers);
            if Platform::get().focused_accessibility_id.peek().0 == 0 {
                commands::vim_keyboard(writer, &event);
            }
            commands::keyboard(writer, source, editor_id, event);
        })
        .on_pointer_down(move |event: Event<PointerEventData>| {
            writer.vim.cancel();
            let mut completion = writer.source_viewport.completion;
            completion.set_if_modified(false);
            if matches!(
                event.button(),
                Some(MouseButton::Back | MouseButton::Forward)
            ) {
                writer.history_step(event.button() == Some(MouseButton::Forward));
                event.stop_propagation();
            }
        })
        .content(Content::Flex)
        .maybe_child(writer.vim.hint(writer))
        .font_size(t::body())
        .background(colors.background)
        .color(colors.text_primary)
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(1.))
                .background(palette::rule(night)),
        );
    if let Some(chrome) = &file_chrome {
        root = root
            .child(chrome.services.clone())
            .child(chrome.project_panel.clone());
    }
    root = root.child(body).child(
        rect()
            .width(Size::fill())
            .padding(Gaps::new(8., 16., 8., 16.))
            .spacing(t::SPACE_XS)
            .border(
                Border::new()
                    .width(BorderWidth {
                        top: 1.,
                        ..Default::default()
                    })
                    .fill(palette::rule(night)),
            )
            .child(
                label()
                    .text(format!("{saved} · {diagnostic_status}{preview_status}"))
                    .color(colors.text_secondary),
            )
            .maybe(!message.is_empty() && !notice_in_localisation, |status| {
                status.child(crate::feedback::NoticeView { feedback: message })
            }),
    );
    root = root
        .child(commands::Palette { writer })
        .child(close_prompt)
        .child(settings::render(
            writer,
            file_chrome.as_ref().map_or(empty_files, |c| c.files),
        ));
    root.into_element()
}

mod feedback;

mod diagnostics;

mod field_completion;

mod field_actions;

/// Native production-component specimen; does not open files or save preferences.
pub fn design_app() -> Element {
    design::specimen::app()
}
