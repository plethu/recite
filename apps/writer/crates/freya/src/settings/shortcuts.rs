//! Action-first binding list, with recording and sequential modifier entry.
use crate::{
    commands::{Command, CommandExt},
    design::{Button, tokens as t},
    editing::Writer,
};
use freya::prelude::*;
use recite_config::{UserConfigEdit, WriterShortcut, WriterShortcuts};
mod capture;

#[derive(Clone, Copy)]
pub(super) struct Shortcuts {
    command: State<Option<Command>>,
    draft: State<String>,
    display: State<String>,
    separate: State<bool>,
    modifiers: State<Modifiers>,
    status: State<String>,
    rows: State<Vec<AccessibilityId>>,
    ids: [AccessibilityId; 9],
}
impl Shortcuts {
    pub fn new() -> Self {
        let command = use_state(|| None);
        let modifiers = use_state(Modifiers::empty);
        let separate = use_state(|| false);
        let ids = std::array::from_fn(|_| use_a11y());
        use_after_side_effect(move || {
            if command.read().is_some() {
                ids[0].request_focus();
            }
        });
        Self {
            command,
            ids,
            draft: use_state(String::new),
            display: use_state(String::new),
            separate,
            modifiers,
            status: use_state(String::new),
            rows: use_state(|| {
                Command::ALL
                    .iter()
                    .map(|_| AccessibilityId::new_unique())
                    .collect()
            }),
        }
    }
    pub fn editing(self) -> bool {
        self.command.read().is_some()
    }
    pub fn cancel(mut self) {
        if let Some(command) = *self.command.peek()
            && let Some(index) = Command::ALL.iter().position(|c| *c == command)
        {
            self.rows.peek()[index].request_focus();
        }
        self.command.set(None);
        self.status.set(String::new());
    }
    pub fn focus_order(self) -> Vec<AccessibilityId> {
        if self.editing() {
            let mut ids = vec![self.ids[0], self.ids[1]];
            if *self.separate.read() {
                ids.extend_from_slice(&self.ids[2..5]);
            }
            if !self.draft.read().is_empty() {
                ids.push(self.ids[5]);
            }
            ids.extend_from_slice(&self.ids[6..8]);
            ids
        } else {
            let mut ids: Vec<_> = Command::ALL
                .iter()
                .zip(self.rows.read().iter())
                .filter(|(c, _)| c.rebindable())
                .map(|(_, id)| *id)
                .collect();
            ids.push(self.ids[8]);
            ids
        }
    }
    fn conflict(self, writer: Writer) -> Option<Command> {
        let binding = self.draft.read();
        if binding.is_empty() {
            return None;
        }
        Command::ALL.iter().copied().find(|c| {
            Some(*c) != *self.command.read()
                && writer
                    .preferences
                    .read()
                    .config
                    .writer
                    .shortcuts
                    .binding(*c)
                    == binding.as_str()
        })
    }
    fn apply(mut self, mut writer: Writer, binding: String, replace: bool) {
        let Some(command) = *self.command.peek() else {
            return;
        };
        let result = WriterShortcut::try_from(binding)
            .and_then(|binding| {
                let mut shortcuts = writer.preferences.peek().config.writer.shortcuts.clone();
                if replace && let Some(other) = self.conflict(writer) {
                    shortcuts.rebind(other, WriterShortcut::default())?;
                }
                shortcuts.rebind(command, binding)?;
                Ok(shortcuts)
            })
            .map_err(|e| e.to_string())
            .and_then(|shortcuts| {
                writer
                    .preferences
                    .write()
                    .update(UserConfigEdit::WriterShortcuts(shortcuts))
            });
        match result {
            Ok(()) => {
                self.cancel();
                self.status.set("Shortcut saved.".into());
            }
            Err(error) => self.status.set(error),
        }
    }
    pub fn render(mut self, writer: Writer) -> Element {
        let mut content = rect()
            .width(Size::fill())
            .padding((0., t::SPACE_SM, 0., 0.))
            .spacing(t::SPACE_MD);
        if let Some(command) = *self.command.read() {
            content = content.child(self.editor(writer, command));
        } else {
            content = content.child(
                label().text("Select a binding to change it. Unassigned actions have no shortcut."),
            );
            for (index, command) in Command::ALL
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, c)| c.rebindable())
            {
                let binding = command.shortcut(writer);
                let binding = if binding.is_empty() {
                    "Unassigned".into()
                } else {
                    binding
                };
                content = content.child(
                    Button::new()
                        .a11y_id(self.rows.read()[index])
                        .named(format!("Rebind {}: {binding}", command.label(writer)))
                        .width(Size::fill())
                        .child(
                            rect()
                                .horizontal()
                                .width(Size::fill())
                                .cross_align(Alignment::Center)
                                .content(Content::Flex)
                                .child(label().width(Size::flex(1.)).text(command.label(writer)))
                                .child(label().font_family("monospace").text(binding)),
                        )
                        .on_press(move |_| {
                            self.draft.set(String::new());
                            self.display.set(String::new());
                            self.status.set(String::new());
                            self.separate.set(false);
                            self.modifiers.set(Modifiers::empty());
                            self.command.set(Some(command));
                        }),
                );
            }
            content = content.child(Button::new().flat().a11y_id(self.ids[8])
                .named("Restore default shortcuts").child("Restore default shortcuts")
                .on_press(move |_| {
                    let mut preferences = writer.preferences;
                    self.status.set(match preferences.write().update(UserConfigEdit::WriterShortcuts(WriterShortcuts::default())) {
                        Ok(()) => "Default shortcuts restored.".into(), Err(error) => error,
                    });
                }))
                .child(label().text("F6 / Shift+F6 moves between workspace regions. Vim: : commands (w / wa / q), / find, n / N matches, Ctrl+o / Ctrl+i history, Ctrl+w then h / j / k / l panes. Navigation gestures are additional defaults; the bindings above assign alternative chords. Undo, redo, and apply use the focused editor’s keys."));
        }
        content
            .child(
                label()
                    .text(self.status.read().clone())
                    .a11y_role(AccessibilityRole::Status)
                    .a11y_builder(|node| node.set_live(accesskit::Live::Polite)),
            )
            .into_element()
    }
    fn editor(mut self, writer: Writer, command: Command) -> Element {
        let conflict = self.conflict(writer);
        let separate = *self.separate.read();
        let mut content = rect().width(Size::fill()).spacing(t::SPACE_MD)
            .child(label().text(format!("Rebind {}", command.label(writer))).font_size(t::title()))
            .child(label().text(format!("Current binding: {}", {
                let binding = command.shortcut(writer);
                if binding.is_empty() { "Unassigned".into() } else { binding }
            })))
            .child(label().text(if separate { "Select modifiers below, then press one letter, number, comma, or function key in the field." }
                else { "Press a new shortcut in the field. Escape cancels. Tab moves to the buttons." }))
            .child(Input::new(self.display).a11y_id(self.ids[0]).placeholder("Press keys…").width(Size::fill())
                .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                    if event.key == Key::Named(NamedKey::Tab) { return false; }
                    event.stop_propagation(); event.prevent_default();
                    if event.key == Key::Named(NamedKey::Escape) { self.cancel(); }
                    else if let Some(result) = capture::binding(&event, if *self.separate.peek() { Some(*self.modifiers.peek()) } else { None }) {
                        match result {
                            Ok(binding) => { self.display.set(binding.replace("Primary", capture::primary_name()));
                                self.draft.set(binding); self.status.set(String::new()); }
                            Err(error) => { self.draft.set(String::new()); self.display.set(String::new()); self.status.set(error); },
                        }
                    }
                    false
                }))
            .child(Button::new().a11y_id(self.ids[1]).checkable(separate).selected(separate)
                .named("Choose keys separately").child("Choose keys separately")
                .on_press(move |_| { let next = !*self.separate.peek(); self.separate.set(next); }));
        if separate {
            let mut row = rect().horizontal().spacing(t::SPACE_SM);
            for (index, (name, modifier)) in [
                (capture::primary_name(), capture::primary()),
                ("Alt", Modifiers::ALT),
                ("Shift", Modifiers::SHIFT),
            ]
            .into_iter()
            .enumerate()
            {
                row = row.child(
                    Button::new()
                        .a11y_id(self.ids[index + 2])
                        .checkable(self.modifiers.read().contains(modifier))
                        .selected(self.modifiers.read().contains(modifier))
                        .named(name)
                        .child(name)
                        .on_press(move |_| {
                            let mut modifiers = *self.modifiers.peek();
                            modifiers.toggle(modifier);
                            self.modifiers.set(modifiers);
                            self.draft.set(String::new());
                            self.display.set(String::new());
                        }),
                );
            }
            content = content.child(row);
        }
        content = content.child(label().text("Use Ctrl/Cmd with letters and numbers, or F1–F12 (except F6). Text-editing and system shortcuts are reserved."));
        if let Some(other) = conflict {
            content = content.child(label().text(format!(
                "{} already uses this shortcut. Replacing it will leave that action unassigned.",
                other.label(writer)
            )));
        }
        content
            .child(
                rect()
                    .horizontal()
                    .spacing(t::SPACE_SM)
                    .child(
                        Button::new()
                            .a11y_id(self.ids[5])
                            .enabled(!self.draft.read().is_empty())
                            .named(if conflict.is_some() {
                                "Replace binding"
                            } else {
                                "Save binding"
                            })
                            .child(if conflict.is_some() {
                                "Replace binding"
                            } else {
                                "Save binding"
                            })
                            .on_press(move |_| self.apply(writer, self.draft.peek().clone(), true)),
                    )
                    .child(
                        Button::new()
                            .a11y_id(self.ids[6])
                            .named("Unassign")
                            .child("Unassign")
                            .on_press(move |_| self.apply(writer, String::new(), false)),
                    )
                    .child(
                        Button::new()
                            .a11y_id(self.ids[7])
                            .named("Reset binding")
                            .child("Reset binding")
                            .on_press(move |_| {
                                self.draft.set(command.default_shortcut().into());
                                self.display.set(
                                    command
                                        .default_shortcut()
                                        .replace("Primary", capture::primary_name()),
                                );
                                self.status.set(String::new());
                            }),
                    ),
            )
            .into_element()
    }
}
