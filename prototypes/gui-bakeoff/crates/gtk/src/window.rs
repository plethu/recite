mod scene;

use std::{cell::RefCell, rc::Rc};

use gtk::{accessible::Property, prelude::*};
use recite_bakeoff_authoring::{FIXTURE, PassageKind, View, Workbench, WorkbenchError};

type Action = Box<dyn Fn(&mut Workbench) -> Result<(), WorkbenchError>>;

struct Window {
    model: RefCell<Workbench>,
    window: gtk::ApplicationWindow,
    navigation: gtk::Box,
    scene: gtk::Box,
    field: gtk::Box,
    attributes: gtk::Box,
    preview: gtk::Box,
    heading: gtk::Label,
    details: gtk::Label,
    editor: gtk::TextView,
    status: gtk::Label,
    diagnostics: gtk::Label,
}

pub fn build(application: &gtk::Application) {
    let model = match Workbench::new(FIXTURE) {
        Ok(model) => model,
        Err(error) => {
            eprintln!("Cannot load bake-off fixture: {error}");
            return;
        }
    };
    let root = column();
    root.set_margin_start(24);
    root.set_margin_end(24);
    root.set_margin_top(20);
    root.set_margin_bottom(20);
    let toolbar = row();
    root.append(&toolbar);
    let title = gtk::Label::new(Some("recite."));
    title.add_css_class("brand");
    toolbar.append(&title);
    let body = row();
    body.set_vexpand(true);
    root.append(&body);
    let navigation = column();
    navigation.set_width_request(220);
    body.append(
        &gtk::ScrolledWindow::builder()
            .min_content_width(240)
            .child(&navigation)
            .build(),
    );
    let field = column();
    field.set_hexpand(true);
    let scene = column();
    let reading = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&scene)
        .build();
    body.append(&reading);
    let heading = gtk::Label::new(None);
    heading.set_xalign(0.);
    heading.add_css_class("heading");
    field.append(&heading);
    let attributes = row();
    field.append(&attributes);
    let details = gtk::Label::new(None);
    details.set_selectable(true);
    details.set_wrap(true);
    let disclosure = gtk::Expander::builder()
        .label("Line / choice details")
        .child(&details)
        .build();
    field.append(&disclosure);
    let editor = gtk::TextView::new();
    editor.set_wrap_mode(gtk::WrapMode::WordChar);
    editor.set_accepts_tab(false);
    editor.set_hexpand(true);
    editor.set_vexpand(true);
    editor.set_top_margin(16);
    editor.set_bottom_margin(16);
    editor.set_left_margin(16);
    editor.set_right_margin(16);
    editor.update_property(&[Property::Label("Dialogue or source draft")]);
    editor.buffer().set_enable_undo(true);
    field.append(
        &gtk::ScrolledWindow::builder()
            .min_content_height(110)
            .vexpand(true)
            .child(&editor)
            .build(),
    );
    let actions = row();
    field.append(&actions);
    let preview = column();
    field.append(&preview);
    let diagnostics = gtk::Label::new(None);
    diagnostics.set_wrap(true);
    diagnostics.set_xalign(0.);
    field.append(&diagnostics);
    let status = gtk::Label::new(Some(
        "Choose a passage and start writing. Session changes are temporary.",
    ));
    status.set_wrap(true);
    status.set_xalign(0.);
    root.append(&status);
    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("Recite — GTK bake-off")
        .default_width(1280)
        .default_height(900)
        .child(&root)
        .build();
    window.add_css_class("recite-candidate");
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("style.css"));
    gtk::style_context_add_provider_for_display(
        &WidgetExt::display(&window),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let ui = Rc::new(Window {
        model: RefCell::new(model),
        window,
        navigation,
        scene,
        field,
        attributes,
        preview,
        heading,
        details,
        editor,
        status,
        diagnostics,
    });
    for (caption, action) in [
        ("Script", Box::new(Workbench::show_script) as Action),
        (
            "Source",
            Box::new(|m: &mut Workbench| m.select(View::Source)) as Action,
        ),
        ("Undo", Box::new(Workbench::undo)),
        ("Redo", Box::new(Workbench::redo)),
    ] {
        toolbar.append(&ui.button(caption, action));
    }
    let theme = gtk::Button::with_label("Dark");
    let weak = Rc::downgrade(&ui);
    theme.connect_clicked(move |button| {
        if let Some(ui) = weak.upgrade() {
            if ui.window.has_css_class("dark") {
                ui.window.remove_css_class("dark");
                button.set_label("Dark");
            } else {
                ui.window.add_css_class("dark");
                button.set_label("Light");
            }
            super::syntax::highlight(
                &ui.editor,
                matches!(ui.model.borrow().view(), View::Source),
                ui.window.has_css_class("dark"),
            );
        }
    });
    toolbar.append(&theme);
    for (caption, action) in [
        ("Apply draft", Box::new(Workbench::apply) as Action),
        (
            "Discard draft",
            Box::new(|m: &mut Workbench| {
                m.discard();
                Ok(())
            }),
        ),
        ("Add choice", Box::new(Workbench::add_choice)),
        ("Try scene", Box::new(Workbench::start_preview)),
    ] {
        actions.append(&ui.button(caption, action));
    }
    let weak = Rc::downgrade(&ui);
    ui.editor.buffer().connect_changed(move |buffer| {
        if let Some(ui) = weak.upgrade() {
            let source = matches!(ui.model.borrow().view(), View::Source);
            super::syntax::highlight(&ui.editor, source, ui.window.has_css_class("dark"));
            let text = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), true)
                .to_string();
            ui.model.borrow_mut().set_draft(text);
            if ui.model.borrow().has_draft() {
                ui.status
                    .set_text("Draft not applied · existing preview may be out of date.");
            }
        }
    });
    ui.refresh();
    ui.window.present();
    // Window owns the controller through its destroy handler; callbacks use weak refs.
    let keep_alive = ui.clone();
    ui.window.connect_destroy(move |_| {
        let _ = &keep_alive;
    });
}

impl Window {
    fn button(self: &Rc<Self>, caption: &str, action: Action) -> gtk::Button {
        let button = gtk::Button::new();
        let label = gtk::Label::new(Some(caption));
        label.set_wrap(true);
        label.set_max_width_chars(28);
        button.set_child(Some(&label));
        let weak = Rc::downgrade(self);
        button.connect_clicked(move |button| {
            let focused = button.is_focus();
            if let Some(ui) = weak.upgrade() {
                let result = action(&mut ui.model.borrow_mut());
                match result {
                    Ok(()) => {
                        ui.refresh();
                        ui.status.set_text(if ui.model.borrow().has_draft() {
                            "Draft not applied · existing preview may be out of date."
                        } else {
                            "Done. Session changes are temporary."
                        });
                        if focused && !button.is_focus() {
                            ui.editor.grab_focus();
                        }
                    }
                    Err(error) => ui.status.set_text(&error.to_string()),
                }
            }
        });
        button
    }

    fn refresh(self: &Rc<Self>) {
        let model = self.model.borrow();
        clear(&self.navigation);
        clear(&self.attributes);
        clear(&self.preview);
        match model.document().passages() {
            Ok(passages) => {
                for passage in passages {
                    let id = passage.id;
                    let caption = match passage.kind {
                        PassageKind::Dialogue { speaker } => {
                            speaker.unwrap_or_else(|| "Narration".into())
                        }
                        PassageKind::Choice { .. } => format!(
                            "Choice: {}",
                            passage.text.chars().take(28).collect::<String>()
                        ),
                    };
                    self.navigation.append(&self.button(
                        &format!(
                            "{} · {}",
                            passage.section.replace('_', " "),
                            caption.replace('_', " ")
                        ),
                        Box::new(move |m| m.select(View::Passage(id.clone()))),
                    ));
                }
            }
            Err(error) => self
                .navigation
                .append(&gtk::Label::new(Some(&error.to_string()))),
        }
        let selected = model.selected().ok().flatten();
        self.heading.set_text(&selected.as_ref().map_or_else(
            || "Source".to_owned(),
            |p| match p.kind {
                PassageKind::Dialogue { .. } => "Dialogue".to_owned(),
                PassageKind::Choice { .. } => "Player choice".to_owned(),
            },
        ));
        self.details.set_text(
            &selected
                .as_ref()
                .map_or_else(String::new, |p| format!("{}@{}", p.label, p.id)),
        );
        if let Some(passage) = selected {
            let (caption, values) = match passage.kind {
                PassageKind::Dialogue { speaker } => (
                    format!("Speaker: {}", speaker.unwrap_or_else(|| "Narration".into())),
                    vec!["alice".to_owned(), "cheshire_cat".to_owned()],
                ),
                PassageKind::Choice { destination } => {
                    let mut values = model.document().sections();
                    values.push("END".into());
                    (
                        format!("Continue to: {}", destination.unwrap_or_default()),
                        values,
                    )
                }
            };
            self.attributes
                .append(&gtk::Label::new(Some(&caption.replace('_', " "))));
            for value in values {
                self.attributes.append(&self.button(
                    &value.replace('_', " "),
                    Box::new(move |m| m.attribute(&value)),
                ));
            }
        }
        if let Some(page) = model.preview_page() {
            self.preview
                .append(&gtk::Label::new(Some(if model.preview_stale() {
                    "Preview is out of date"
                } else {
                    "Preview"
                })));
            let prose = gtk::Label::new(Some(&page.text));
            prose.set_wrap(true);
            self.preview.append(&prose);
            for (index, choice) in page.choices.iter().enumerate() {
                self.preview.append(&self.button(
                    &choice.text,
                    Box::new(move |m| m.advance_preview(Some(index))),
                ));
            }
            if page.choices.is_empty() && !page.ended {
                self.preview
                    .append(&self.button("Continue", Box::new(|m| m.advance_preview(None))));
            }
        }
        self.diagnostics.set_text(
            &model
                .document()
                .diagnostics()
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let source = matches!(model.view(), View::Source);
        self.heading.set_visible(source);
        let draft = model.draft().to_owned();
        drop(model);
        if source {
            self.editor.add_css_class("source");
        } else {
            self.editor.remove_css_class("source");
        }
        let buffer = self.editor.buffer();
        if buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .as_str()
            != draft
        {
            buffer.set_text(&draft);
        }
        super::syntax::highlight(&self.editor, source, self.window.has_css_class("dark"));
        self.refresh_scene();
    }
}

fn clear(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}
fn column() -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, 12)
}
fn row() -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, 12)
}
