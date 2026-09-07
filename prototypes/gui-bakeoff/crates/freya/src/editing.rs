use freya::{code_editor::*, prelude::*};
use recite_bakeoff_authoring::{View, Workbench, WorkbenchError};

pub type Session = State<Result<Workbench, WorkbenchError>>;

pub fn editor_data(text: &str, source: bool, dark: bool) -> CodeEditorData {
    let language = source.then(|| {
        EditorLanguage::new(
            recite_bakeoff_grammar::LANGUAGE,
            recite_bakeoff_grammar::HIGHLIGHTS,
        )
    });
    let mut data = CodeEditorData::new(Rope::from_str(text), language);
    data.set_theme(super::palette::syntax(dark));
    data.parse();
    data.measure(
        if source { 15. } else { 20. },
        if source { "monospace" } else { "serif" },
    );
    data
}

pub fn perform(
    mut model: Session,
    mut editor: State<CodeEditorData>,
    mut message: State<String>,
    mut prose: State<String>,
    dark: bool,
    action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
) {
    let mut state = model.write();
    let outcome = match state.as_mut() {
        Ok(session) => {
            session.set_draft(if matches!(session.view(), View::Source) {
                editor.peek().rope.to_string()
            } else {
                prose.peek().clone()
            });
            let previous_view = session.view().clone();
            action(session).map(|()| {
                if session.view() != &previous_view || session.draft() != editor.peek().rope {
                    editor.set(editor_data(
                        session.draft(),
                        matches!(session.view(), View::Source),
                        dark,
                    ));
                }
                prose.set_if_modified(session.draft().to_owned());
                "Done.".to_owned()
            })
        }
        Err(error) => {
            message.set(error.to_string());
            return;
        }
    };
    message.set(match outcome {
        Ok(text) => text,
        Err(error) => error.to_string(),
    });
}
