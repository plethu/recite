mod closing;
mod editing;
mod files;
mod palette;
mod passage_menu;
mod project;
mod project_context;
mod recovery;
pub use closing::request_close;
mod field;
mod preview_panel;
mod scene;

use editing::editor_data;
use freya::prelude::*;
use recite_bakeoff_authoring::{FIXTURE, View, Workbench};

pub fn app() -> Element {
    workbench(false)
}

/// First file-backed workbench; comparison entry remains in-memory.
pub fn editor_app() -> Element {
    workbench(true)
}

fn workbench(file_backed: bool) -> Element {
    let mut model = use_state(|| Workbench::new(FIXTURE));
    let mut dark = use_state(|| false);
    let mut theme = use_init_theme(|| palette::theme(false));
    let message = use_state(String::new);
    let details = use_state(|| false);
    let mut editor = use_state(move || {
        let state = model.peek();
        editor_data(state.as_ref().map_or("", Workbench::draft), false, false)
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
    let night = *dark.read();
    let writer = editing::Writer {
        buffers,
        message,
        dark: night,
    };
    let mut preview_visible = use_state(|| false);
    let mut navigation_visible = use_state(|| true);
    let file_chrome = file_backed.then(|| files::controls(buffers, message, night));
    let state = model.read();
    let session = match state.as_ref() {
        Ok(session) => session,
        Err(error) => return label().text(error.to_string()).into_element(),
    };
    let source = session.view() == &View::Source;
    let colors = theme.read().colors.clone();
    let scene_name = palette::display_name(session.document().key().as_str());
    let mut navigation = rect()
        .width(Size::px(208.))
        .height(Size::fill())
        .padding(16.)
        .spacing(16.)
        .child(
            label()
                .text("Scenes")
                .color(colors.text_secondary)
                .font_size(14.),
        );
    if let Some(chrome) = &file_chrome {
        navigation = navigation.child(chrome.scenes.clone());
    } else {
        navigation = navigation
            .child(palette::navigation_button(true, night).child(label().text(scene_name.clone())));
    }
    navigation = navigation.child(
        label()
            .text("In this scene")
            .font_size(14.)
            .color(colors.text_secondary),
    );
    let mut previous = String::new();
    if let Ok(passages) = session.document().passages() {
        let selected_section = session.selected().ok().flatten().map(|p| p.section);
        for passage in passages {
            if passage.section == previous {
                continue;
            }
            previous = passage.section.clone();
            let selected = selected_section.as_deref() == Some(passage.section.as_str());
            let caption = palette::display_name(&passage.section);
            let id = passage.id;
            navigation = navigation.child(
                palette::navigation_button(selected, night)
                    .on_press(move |_| writer.perform(|m| m.select(View::Passage(id.clone()))))
                    .child(label().text(caption)),
            );
        }
    }
    let mut toolbar = rect()
        .width(Size::fill())
        .height(Size::px(68.))
        .content(Content::Flex)
        .horizontal()
        .cross_align(Alignment::Center)
        .padding(16.)
        .spacing(16.)
        .child(label().text("recite.").font_size(24.))
        .child(
            Button::new()
                .cursor_icon(CursorIcon::Pointer)
                .flat()
                .on_press(move |_| {
                    let next = !*navigation_visible.peek();
                    navigation_visible.set(next);
                })
                .child(if *navigation_visible.read() {
                    "Hide scenes"
                } else {
                    "Show scenes"
                }),
        )
        .child(
            rect()
                .horizontal()
                .spacing(4.)
                .child(
                    Button::new()
                        .cursor_icon(CursorIcon::Pointer)
                        .flat()
                        .theme_colors(ButtonColorsThemePartial {
                            background: Some(Preference::Specific(if source {
                                Color::TRANSPARENT
                            } else {
                                palette::selection(night)
                            })),
                            ..Default::default()
                        })
                        .on_press(move |_| writer.perform(Workbench::show_script))
                        .child("Script"),
                )
                .child(
                    Button::new()
                        .cursor_icon(CursorIcon::Pointer)
                        .flat()
                        .theme_colors(ButtonColorsThemePartial {
                            background: Some(Preference::Specific(if source {
                                palette::selection(night)
                            } else {
                                Color::TRANSPARENT
                            })),
                            ..Default::default()
                        })
                        .on_press(move |_| writer.perform(|m| m.select(View::Source)))
                        .child("Source"),
                ),
        )
        .child(rect().width(Size::flex(1.)))
        .child(
            Button::new()
                .cursor_icon(CursorIcon::Pointer)
                .flat()
                .on_press(move |_| writer.perform(Workbench::undo))
                .child("Undo"),
        )
        .child(
            Button::new()
                .cursor_icon(CursorIcon::Pointer)
                .flat()
                .on_press(move |_| writer.perform(Workbench::redo))
                .child("Redo"),
        )
        .child(
            Button::new()
                .cursor_icon(CursorIcon::Pointer)
                .flat()
                .on_press(move |_| {
                    let next = !*dark.peek();
                    dark.set(next);
                    theme.set(palette::theme(next));
                    let mut data = editor.write();
                    data.set_theme(palette::syntax(next));
                    data.parse();
                })
                .child(if night { "Light" } else { "Dark" }),
        );
    if let Some(chrome) = &file_chrome {
        toolbar = toolbar.child(chrome.actions.clone());
    }
    let active = field::render(writer, details, editor_id);
    let mut reading = rect()
        .width(Size::fill())
        .max_width(Size::px(1040.))
        .padding(Gaps::new(24., 40., 32., 40.))
        .spacing(8.)
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(40.))
                .content(Content::Flex)
                .horizontal()
                .cross_align(Alignment::Center)
                .child(
                    label()
                        .text(format!("Scene / {scene_name}"))
                        .font_size(14.)
                        .color(colors.text_secondary),
                )
                .child(rect().width(Size::flex(1.)))
                .child(
                    Button::new()
                        .cursor_icon(CursorIcon::Pointer)
                        .flat()
                        .on_press(move |_| {
                            writer.perform(Workbench::start_preview);
                            if writer
                                .buffers
                                .model
                                .peek()
                                .as_ref()
                                .is_ok_and(|m| m.preview_page().is_some())
                            {
                                preview_visible.set(true);
                            }
                        })
                        .child("Try scene"),
                ),
        )
        .child(scene::reading_surface(writer, active));
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
        .maybe_child((*navigation_visible.read()).then(|| {
            ScrollView::new()
                .width(Size::px(208.))
                .height(Size::fill())
                .child(navigation)
        }))
        .child(
            rect()
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
                .child(
                    ScrollView::new()
                        .height(Size::fill())
                        .width(Size::fill())
                        .child(reading),
                ),
        );
    if *preview_visible.read() {
        body = body.child(preview_panel::render(writer, preview_visible));
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
        .content(Content::Flex)
        .font_size(14.)
        .background(colors.background)
        .color(colors.text_primary)
        .child(toolbar)
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
    if let Some(chrome) = file_chrome {
        root = root.child(chrome.close_prompt);
    }
    root.into_element()
}
