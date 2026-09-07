use floem::{
    prelude::*,
    views::{
        dyn_container,
        editor::text::{SimpleStyling, WrapMethod},
        scroll, text_editor, v_stack_from_iter,
    },
};
use recite_bakeoff_authoring::{FIXTURE, PassageKind, View, Workbench, WorkbenchError};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct Session {
    model: Rc<RefCell<Workbench>>,
    revision: RwSignal<u64>,
    status: RwSignal<String>,
    dark: RwSignal<bool>,
}
impl Session {
    fn action(&self, f: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>) {
        let result = f(&mut self.model.borrow_mut());
        match result {
            Ok(()) => {
                self.status.set("Applied. Changes are temporary.".into());
                self.revision.update(|v| *v += 1);
            }
            Err(error) => self.status.set(error.to_string()),
        }
    }
    fn button(
        &self,
        title: &'static str,
        f: impl Fn(&mut Workbench) -> Result<(), WorkbenchError> + 'static,
    ) -> floem::views::Button {
        let session = self.clone();
        button(title).action(move || session.action(&f))
    }
}
fn app(model: Workbench) -> impl IntoView {
    let session = Session {
        model: Rc::new(RefCell::new(model)),
        revision: RwSignal::new(0),
        status: RwSignal::new("Choose a passage and start writing. Changes are temporary.".into()),
        dark: RwSignal::new(false),
    };
    let dark = session.dark;
    let status = session.status;
    let revision = session.revision;
    let toolbar = h_stack((
        label(|| "recite.").style(|s| s.font_size(30.)),
        session.button("Script", Workbench::show_script),
        session.button("Source", |m| m.select(View::Source)),
        session.button("Undo", Workbench::undo),
        session.button("Redo", Workbench::redo),
        button("Light / dark").action(move || dark.update(|v| *v = !*v)),
    ))
    .style(|s| s.gap(10.));
    v_stack((
        toolbar,
        label(move || status.get()),
        dyn_container(move || revision.get(), move |_| scene(session.clone())),
    ))
    .style(move |s| {
        s.size_full()
            .padding(24.)
            .gap(20.)
            .font_size(16.)
            .background(if dark.get() {
                Color::rgb8(32, 33, 36)
            } else {
                Color::rgb8(245, 242, 237)
            })
            .color(if dark.get() {
                Color::rgb8(232, 228, 222)
            } else {
                Color::rgb8(52, 50, 46)
            })
    })
}
fn scene(session: Session) -> impl IntoView {
    let model = session.model.borrow();
    let passages = model.document().passages().unwrap_or_default();
    let current = model.view().clone();
    let draft = model.draft().to_owned();
    drop(model);
    let navigation = passages
        .iter()
        .map(|p| {
            let id = p.id.clone();
            let title = format!(
                "{} · {}",
                p.section,
                match &p.kind {
                    PassageKind::Dialogue { speaker } =>
                        speaker.clone().unwrap_or_else(|| "Narration".into()),
                    PassageKind::Choice { .. } => "Player choice".into(),
                }
            );
            let session = session.clone();
            button(title)
                .action(move || session.action(|m| m.select(View::Passage(id.clone()))))
                .into_any()
        })
        .collect::<Vec<_>>();
    let editor = text_editor(draft);
    let editing = session.clone();
    let source = current == View::Source;
    let mut styling = SimpleStyling::new();
    styling.set_font_size(if source { 15 } else { 21 });
    styling.set_font_family(vec![if source {
        floem::text::FamilyOwned::Monospace
    } else {
        floem::text::FamilyOwned::Serif
    }]);
    let editor = editor
        .styling(styling)
        .update(move |event| {
            if let Some(editor) = event.editor {
                editing
                    .model
                    .borrow_mut()
                    .set_draft(editor.doc().text().to_string());
            }
        })
        .editor_style(move |s| {
            s.hide_gutter(!source).wrap_method(if source {
                WrapMethod::None
            } else {
                WrapMethod::EditorWidth
            })
        })
        .style(move |s| s.width_full().height(if source { 480. } else { 130. }));
    let mut cards = Vec::new();
    let mut active = Some(editor.into_any());
    if source && let Some(editor) = active.take() {
        cards.push(editor);
    }
    for p in passages {
        if source {
            break;
        }
        let selected = current == View::Passage(p.id.clone());
        let caption = match &p.kind {
            PassageKind::Dialogue { speaker } => {
                speaker.clone().unwrap_or_else(|| "Narration".into())
            }
            PassageKind::Choice { destination } => format!(
                "Player choice · continue to {}",
                destination.as_deref().unwrap_or("next passage")
            ),
        };
        cards.push(label(move || caption.clone()).into_any());
        if selected {
            if let Some(editor) = active.take() {
                cards.push(editor);
            }
        } else {
            cards.push(
                label(move || p.text.clone())
                    .style(|s| s.font_size(21.).padding(12.))
                    .into_any(),
            );
        }
    }
    cards.push(
        h_stack((
            session.button("Apply draft", Workbench::apply),
            session.button("Discard draft", |m| {
                m.discard();
                Ok(())
            }),
            session.button("Add choice", Workbench::add_choice),
        ))
        .style(|s| s.gap(10.))
        .into_any(),
    );
    h_stack((
        v_stack_from_iter(navigation).style(|s| s.width(220.).gap(10.)),
        scroll(v_stack_from_iter(cards).style(|s| s.width_full().gap(16.)))
            .style(|s| s.size_full()),
    ))
    .style(|s| s.size_full().gap(24.))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Workbench::new(FIXTURE)?;
    let title = std::env::var("RECITE_BAKEOFF_WINDOW_ID")
        .unwrap_or_else(|_| "Recite — Floem bake-off".into());
    floem::Application::new()
        .window(
            move |_| app(model),
            Some(floem::window::WindowConfig::default().title(title)),
        )
        .run();
    Ok(())
}
