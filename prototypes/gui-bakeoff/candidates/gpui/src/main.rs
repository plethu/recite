use gpui::{prelude::*, *};
use gpui_component::{
    Root, Theme, ThemeMode,
    button::Button,
    input::{Editor, EditorState, InputEvent, Textarea, TextareaState},
};
use recite_bakeoff_authoring::{FIXTURE, View, Workbench, WorkbenchError};

actions!(recite_writer, [NextControl, PreviousControl]);

struct WorkbenchView {
    model: Workbench,
    prose: Entity<TextareaState>,
    source: Entity<EditorState>,
    status: String,
    dark: bool,
    details: bool,
}
#[derive(Clone)]
enum Command {
    Script,
    Source,
    Apply,
    Discard,
    Undo,
    Redo,
    Select(String),
    AddChoice,
    Attribute(String),
    Preview,
    Advance(Option<usize>),
}
impl WorkbenchView {
    fn new(model: Workbench, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let prose = cx.new(|cx| {
            let mut input = TextareaState::new(window, cx);
            input.set_value(model.draft().to_owned(), window, cx);
            input
        });
        let source = cx.new(|cx| EditorState::new(window, cx).language("recite"));
        cx.subscribe_in(&prose, window, |s, input, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) && s.model.view() != &View::Source {
                s.model.set_draft(input.read(cx).value().to_string());
                cx.notify();
            }
        })
        .detach();
        cx.subscribe_in(&source, window, |s, input, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) && s.model.view() == &View::Source {
                s.model.set_draft(input.read(cx).value().to_string());
                cx.notify();
            }
        })
        .detach();
        Self {
            model,
            prose,
            source,
            status: "Choose a passage. Changes are temporary.".into(),
            dark: false,
            details: false,
        }
    }
    fn command(&mut self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        let composing = if self.model.view() == &View::Source {
            self.source.update(cx, |input, cx| {
                input.marked_text_range(window, cx).is_some()
            })
        } else {
            self.prose.update(cx, |input, cx| {
                input.marked_text_range(window, cx).is_some()
            })
        };
        if composing {
            self.status = "Finish or cancel text composition before continuing.".into();
            cx.notify();
            return;
        }
        let draft = if self.model.view() == &View::Source {
            self.source.read(cx).value()
        } else {
            self.prose.read(cx).value()
        };
        self.model.set_draft(draft.to_string());
        let old_view = self.model.view().clone();
        let old_text = self.model.draft().to_owned();
        let applied = matches!(command, Command::Apply);
        let result: Result<(), WorkbenchError> = match command {
            Command::Script => self.model.show_script(),
            Command::Source => self.model.select(View::Source),
            Command::Apply => self.model.apply(),
            Command::Discard => {
                self.model.discard();
                Ok(())
            }
            Command::Undo => self.model.undo(),
            Command::Redo => self.model.redo(),
            Command::Select(id) => self.model.select(View::Passage(id)),
            Command::AddChoice => self.model.add_choice(),
            Command::Attribute(value) => self.model.attribute(&value),
            Command::Preview => self.model.start_preview(),
            Command::Advance(choice) => self.model.advance_preview(choice),
        };
        match result {
            Ok(()) => {
                self.status = if applied {
                    "Applied. Changes are temporary."
                } else {
                    "Changes are temporary; this prototype does not save files."
                }
                .into();
                let text = self.model.draft().to_owned();
                if self.model.view() != &old_view || text != old_text {
                    if self.model.view() == &View::Source {
                        self.source
                            .update(cx, |s, cx| s.set_value(text, window, cx));
                    } else {
                        self.prose.update(cx, |s, cx| s.set_value(text, window, cx));
                    }
                }
            }
            Err(e) => self.status = e.to_string(),
        }
        cx.notify();
    }
    fn button(
        &self,
        id: &'static str,
        title: &'static str,
        command: Command,
        cx: &Context<Self>,
    ) -> Button {
        Button::new(id)
            .label(title)
            .on_click(cx.listener(move |s, _, window, cx| s.command(command.clone(), window, cx)))
    }
}
mod scene;
mod writer_controls;

fn change_theme(dark: bool, window: &mut Window, cx: &mut App) {
    Theme::change(
        if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        Some(window),
        cx,
    );
    let theme = Theme::global_mut(cx);
    theme.colors.background = rgb(if dark { 0x202124 } else { 0xf5f2ed }).into();
    theme.colors.foreground = rgb(if dark { 0xe8e4de } else { 0x34322e }).into();
    theme.colors.primary = rgb(if dark { 0x9fb59d } else { 0x536d57 }).into();
    theme.colors.ring = theme.colors.primary;
    let highlight = std::sync::Arc::make_mut(&mut theme.highlight_theme);
    highlight.style.editor_background = Some(theme.colors.background);
    highlight.style.editor_foreground = Some(theme.colors.foreground);
    // Recite prose is the main reading material, not an error-coloured literal.
    highlight.style.syntax.string = None;
    highlight.style.syntax.string_special = None;
    Theme::sync_base(cx);
}

fn initialize(cx: &mut App) {
    gpui_component::init(cx);
    cx.bind_keys([
        KeyBinding::new("tab", NextControl, Some("ReciteProse > Input")),
        KeyBinding::new("shift-tab", PreviousControl, Some("ReciteProse > Input")),
    ]);
    use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
    LanguageRegistry::singleton().register(
        "recite",
        &LanguageConfig::new(
            "recite",
            recite_bakeoff_grammar::LANGUAGE.into(),
            vec![],
            recite_bakeoff_grammar::HIGHLIGHTS,
            "",
            "",
        ),
    );
}

#[cfg(test)]
mod tests;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Workbench::new(FIXTURE)?;
    gpui_platform::application().run(move |cx| {
        initialize(cx);
        let result = cx.open_window(
            WindowOptions {
                app_id: std::env::var("RECITE_BAKEOFF_WINDOW_ID").ok(),
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1200.), px(800.)),
                ))),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title("Recite — GPUI bake-off");
                change_theme(false, window, cx);
                let view = cx.new(|cx| WorkbenchView::new(model, window, cx));
                if let Ok(mode) = std::env::var("RECITE_BAKEOFF_CAPTURE_VIEW") {
                    view.update(cx, |state, cx| {
                        if mode.ends_with("dark") {
                            state.dark = true;
                            change_theme(true, window, cx);
                        }
                        if mode.starts_with("source") {
                            state.command(Command::Source, window, cx);
                        }
                    });
                }
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        if let Err(error) = result {
            eprintln!("Cannot open GPUI window: {error}");
            cx.quit();
        }
    });
    Ok(())
}
