//! Native browsing supplements editable paths; selection never opens a project implicitly.
use super::{Button, tokens as t};
use freya::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PathKind {
    Project,
    Catalogue,
    Schema,
}
impl PathKind {
    fn title(self) -> &'static str {
        match self {
            Self::Project => "Browse for project folder",
            Self::Schema => "Browse for standalone schema source",
            Self::Catalogue => "Browse for PO catalogue",
        }
    }
}
#[derive(Clone, PartialEq)]
pub(crate) struct PathField {
    pub value: State<String>,
    pub id: AccessibilityId,
    pub browse_id: AccessibilityId,
    pub kind: PathKind,
    pub enabled: bool,
    pub submit: EventHandler<()>,
}
impl Component for PathField {
    fn render(&self) -> impl IntoElement {
        let mut value = self.value;
        let mut busy = use_state(|| false);
        let Self {
            id,
            browse_id,
            kind,
            enabled,
            submit,
            ..
        } = self.clone();
        rect()
            .horizontal()
            .content(Content::Flex)
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .child(
                Input::new(value)
                    .a11y_id(id)
                    .width(Size::flex(1.))
                    .placeholder(match kind {
                        PathKind::Project => "Project folder or recite.project.toml",
                        PathKind::Schema => "/path/to/schema.toml",
                        PathKind::Catalogue => "/path/to/fr.po",
                    })
                    .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                        if !enabled || *busy.peek() {
                            return false;
                        }
                        if event.key == Key::Named(NamedKey::Enter) && event.modifiers.is_empty() {
                            submit.call(());
                            event.stop_propagation();
                            false
                        } else {
                            crate::closing::text_input_key(event)
                        }
                    }),
            )
            .child(
                Button::new()
                    .a11y_id(browse_id)
                    .named(kind.title())
                    .enabled(enabled && !*busy.read())
                    .on_press(move |_| {
                        if *busy.peek() {
                            return;
                        }
                        busy.set(true);
                        let path = value.peek().clone();
                        let platform = Platform::get();
                        spawn(async move {
                            let mut dialog = rfd::AsyncFileDialog::new().set_title(kind.title());
                            if !path.is_empty() {
                                let path = std::path::Path::new(&path);
                                dialog = dialog.set_directory(if path.is_dir() {
                                    path
                                } else {
                                    path.parent().unwrap_or(path)
                                });
                            }
                            let parented = platform
                                .post_callback(move |id, renderer| {
                                    match renderer.windows().get(&id) {
                                        Some(window) => dialog.set_parent(window.window()),
                                        None => dialog,
                                    }
                                })
                                .await;
                            let Ok(dialog) = parented else {
                                busy.set(false);
                                return;
                            };
                            let selected = match kind {
                                PathKind::Project => dialog.pick_folder().await,
                                PathKind::Schema => {
                                    dialog
                                        .add_filter("Standalone schema", &["toml"])
                                        .pick_file()
                                        .await
                                }
                                PathKind::Catalogue => {
                                    dialog.add_filter("PO catalogue", &["po"]).pick_file().await
                                }
                            };
                            if let Some(file) = selected {
                                value.set(file.path().to_string_lossy().into_owned());
                            }
                            busy.set(false);
                            id.request_focus();
                        });
                    })
                    .child(if *busy.read() {
                        "Browsing…"
                    } else {
                        "Browse…"
                    }),
            )
    }
}

#[cfg(test)]
mod tests;
