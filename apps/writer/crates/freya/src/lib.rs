mod beat_heading;
mod branch_preview;
mod chrome;
mod closing;
mod controls;
mod editing;
mod examples;
mod files;
mod palette;
mod passage_menu;
mod project;
mod project_context;
mod recovery;
mod route_editor;
pub use closing::request_close;
mod field;
mod preferences;
mod preview_panel;
mod prose;
mod scene;
mod scene_map;
mod settings;
mod sidebar;

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
    workbench(AppMode::Examples)
}

/// The original Alice scene retained for interaction regressions.
pub fn regression_app() -> Element {
    workbench(AppMode::Regression)
}

/// Opens the file-backed writer with project discovery and recovery.
pub fn editor_app() -> Element {
    workbench(AppMode::Project)
}

fn workbench(mode: AppMode) -> Element {
    let mut model = use_state(move || {
        if mode == AppMode::Examples {
            WRITER_EXAMPLES[0].open()
        } else {
            Workbench::new(FIXTURE)
        }
    });
    let store = use_try_consume::<Result<recite_config::UserConfigStore, String>>();
    let preferences = use_state(move || preferences::Preferences::load(store));
    let initially_dark = preferences.peek().config.writer.theme == recite_config::WriterTheme::Dark;
    let mut dark = use_state(move || initially_dark);
    let mut theme = use_init_theme(move || palette::theme(initially_dark));
    let settings_open = use_state(|| false);
    let map_focus = use_a11y();
    let inspector_focus = use_a11y();
    let sidebar_focus = use_a11y();
    let search_focus = use_a11y();
    let mut selection = use_state(|| None::<String>);
    let search = use_state(String::new);
    let message = use_state(String::new);
    let mut editor = use_state(move || {
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
    let buffers = files::Buffers {
        model,
        editor,
        prose,
    };
    use_side_effect(move || {
        let next = preferences.read().config.writer.theme == recite_config::WriterTheme::Dark;
        if next != *dark.peek() {
            dark.set(next);
            theme.set(palette::theme(next));
            let mut data = editor.write();
            data.set_theme(palette::syntax(next));
            data.parse();
        }
    });
    let night = *dark.read();
    let scroll = use_scroll_controller(ScrollConfig::default);
    let pane = use_state(|| editing::Pane::Map);
    let mut previous_pane = use_state(|| editing::Pane::Map);
    use_after_side_effect(move || {
        let current = *pane.read();
        if *previous_pane.peek() != current {
            if current == editing::Pane::Script {
                inspector_focus.request_focus();
            } else if current == editing::Pane::Map {
                map_focus.request_focus();
            }
            previous_pane.set(current);
        }
    });
    let expanded = use_state(std::collections::BTreeSet::new);
    let writer = editing::Writer {
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
    let navigation_visible = use_state(|| true);
    let file_chrome = (mode == AppMode::Project).then(|| files::controls(buffers, message, night));
    let example_scenes =
        (mode == AppMode::Examples).then(|| examples::navigation(buffers, message, night));
    let empty_files = use_state(|| None);
    let mut displayed_document = use_state(String::new);
    let document = model
        .peek()
        .as_ref()
        .ok()
        .map(|m| m.document().key().to_string())
        .unwrap_or_default();
    if *displayed_document.peek() != document {
        displayed_document.set(document);
        selection.set(None);
        match preferences.peek().config.writer.view {
            recite_config::WriterView::Map => writer.navigate(Workbench::show_script),
            recite_config::WriterView::Source => writer.navigate(|m| m.select(View::Source)),
        }
    }
    let close_prompt = closing::controls(
        buffers,
        preferences,
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
        label().text(scene_name.clone()).into_element()
    };
    let navigation = sidebar::render(writer, navigation_visible, dark, theme, scenes);
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
        .padding((8., 20.))
        .spacing(8.)
        .child(scene::reading_surface(writer, active.clone()));
    let diagnostics = session.document().diagnostics();
    for diagnostic in &diagnostics {
        let text = if source {
            format!(
                "{}:{} · {} · {}",
                diagnostic.span.file,
                diagnostic.span.start.line(),
                diagnostic.code,
                diagnostic.message
            )
        } else {
            diagnostic.message.clone()
        };
        reading = reading.child(label().text(text).font_size(14.));
    }
    let mut body = rect()
        .content(Content::Flex)
        .horizontal()
        .width(Size::fill())
        .height(Size::flex(1.))
        .child(navigation)
        .child(
            rect()
                .key("scene-editor")
                .content(Content::Flex)
                .width(Size::flex(1.))
                .height(Size::fill())
                .background(palette::reading(night))
                .border(
                    Border::new()
                        .width(BorderWidth {
                            left: 1.,
                            ..Default::default()
                        })
                        .fill(palette::rule(night)),
                )
                .child(toolbar)
                .child(
                    rect()
                        .horizontal()
                        .content(Content::Flex)
                        .width(Size::fill())
                        .height(Size::flex(1.))
                        .maybe_child((!source).then_some(map))
                        .maybe_child((source || *pane.read() == editing::Pane::Script).then(
                            || {
                                rect()
                                    .key("writing-pane")
                                    .a11y_id(inspector_focus)
                                    .a11y_focusable(true)
                                    .a11y_role(AccessibilityRole::Group)
                                    .a11y_alt("Beat editor")
                                    .width(if source { Size::fill() } else { Size::px(520.) })
                                    .height(Size::fill())
                                    .on_key_down(move |event: Event<KeyboardEventData>| {
                                        if event.key == Key::Named(NamedKey::Escape) && !source {
                                            event.stop_propagation();
                                            writer.close_editor();
                                        }
                                    })
                                    .child(if source {
                                        active
                                    } else {
                                        ScrollView::new_controlled(scroll)
                                            .height(Size::fill())
                                            .width(Size::fill())
                                            .child(reading)
                                            .into_element()
                                    })
                            },
                        )),
                ),
        );
    if *pane.read() == editing::Pane::Preview {
        body = body.child(preview_panel::render(writer));
    }
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
    let mut root = rect()
        .expanded()
        .on_global_key_down(move |event: Event<KeyboardEventData>| {
            if *writer.settings_open.peek() {
                return;
            }
            if event.code == Code::F6 {
                let mut regions = vec![sidebar_focus];
                if source {
                    regions.push(editor_id);
                } else {
                    regions.push(map_focus);
                    if *pane.peek() == editing::Pane::Script {
                        regions.push(inspector_focus);
                    }
                }
                let focused = *Platform::get().focused_accessibility_id.peek();
                let index = regions.iter().position(|id| *id == focused).unwrap_or(0);
                let step = if event.modifiers.contains(Modifiers::SHIFT) {
                    regions.len() - 1
                } else {
                    1
                };
                regions[(index + step) % regions.len()].request_focus();
                event.stop_propagation();
                event.prevent_default();
            } else if editing::is_workspace_key(&event) {
                let mut settings = writer.settings_open;
                settings.set(true);
                event.stop_propagation();
                event.prevent_default();
            } else {
                closing::keyboard(event);
            }
        })
        .content(Content::Flex)
        .font_size(14.)
        .background(colors.background)
        .color(colors.text_primary)
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(1.))
                .background(palette::rule(night)),
        );
    if let Some(chrome) = &file_chrome {
        root = root.child(chrome.project_panel.clone());
    }
    root = root.child(body).child(
        rect()
            .width(Size::fill())
            .padding(Gaps::new(8., 16., 8., 16.))
            .spacing(4.)
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
            .maybe(!message.read().is_empty(), |status| {
                status.child(label().text(message.read().clone()))
            }),
    );
    root = root.child(close_prompt).child(settings::render(
        writer,
        file_chrome.as_ref().map_or(empty_files, |c| c.files),
    ));
    root.into_element()
}
