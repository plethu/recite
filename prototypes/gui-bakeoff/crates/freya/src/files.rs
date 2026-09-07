use crate::{
    editing::{Session, editor_data},
    project::ProjectFiles,
};
use freya::{code_editor::CodeEditorData, prelude::*};
use recite_bakeoff_authoring::{View, Workbench};

#[derive(Clone, Copy)]
pub(crate) struct Buffers {
    pub model: Session,
    pub editor: State<CodeEditorData>,
    pub prose: State<String>,
}

impl Buffers {
    fn load(mut self, project: &ProjectFiles, dark: bool) -> Result<(), String> {
        let next = Workbench::open(
            project.document_name().map_err(|e| e.to_string())?,
            project.source(),
        )
        .map_err(|e| e.to_string())?;
        self.editor.set(editor_data(
            next.draft(),
            next.view() == &View::Source,
            dark,
        ));
        self.prose.set(next.draft().to_owned());
        self.model.set(Ok(next));
        Ok(())
    }

    fn harvest(mut self) {
        if let Ok(model) = self.model.write().as_mut() {
            model.set_draft(if model.view() == &View::Source {
                self.editor.peek().rope.to_string()
            } else {
                self.prose.peek().clone()
            });
        }
    }

    fn can_leave(self, files: Option<&ProjectFiles>) -> bool {
        self.harvest();
        self.model
            .peek()
            .as_ref()
            .is_ok_and(|m| !m.has_draft() && files.is_none_or(|f| !f.dirty(m.document().source())))
    }
}

pub(crate) fn controls(buffers: Buffers, mut message: State<String>, dark: bool) -> Element {
    let path = use_state(|| std::env::args().nth(2).unwrap_or_default());
    let mut files = use_state(|| None::<ProjectFiles>);
    let current = files.read();
    let mut panel = rect().spacing(8.).child(
        rect().horizontal().spacing(8.)
            .child(Input::new(path).width(Size::px(460.)).placeholder("Project folder or recite.project.toml"))
            .child(Button::new().on_press(move |_| {
                if !buffers.can_leave(files.peek().as_ref()) {
                    message.set("Save changes and apply or discard the draft before opening another project.".into());
                    return;
                }
                let result = ProjectFiles::open(std::path::Path::new(path.peek().as_str()));
                match result {
                    Ok(project) => match buffers.load(&project, dark) {
                        Ok(()) => { files.set(Some(project)); message.set("Project opened.".into()); }
                        Err(error) => message.set(error),
                    },
                    Err(error) => message.set(error.to_string()),
                }
            }).child("Open project"))
            .child(Button::new().enabled(current.is_some()).on_press(move |_| {
                buffers.harvest();
                let mut model = buffers.model;
                let mut state = model.write();
                let Ok(workbench) = state.as_mut() else { return; };
                if let Err(error) = workbench.apply() {
                    message.set(error.to_string());
                    return;
                }
                let mut opened = files.write();
                if let Some(project) = opened.as_mut() {
                    match project.save(workbench.document().source()) {
                        Ok(()) => message.set("Saved. The previous file is retained beside it as a hidden backup.".into()),
                        Err(error) => message.set(error.to_string()),
                    }
                }
            }).child("Save"))
    );
    if let Some(project) = current.as_ref() {
        let dirty = buffers
            .model
            .read()
            .as_ref()
            .is_ok_and(|m| m.has_draft() || project.dirty(m.document().source()));
        panel = panel.child(label().text(format!(
            "{}{}",
            project.current.display(),
            if dirty {
                " · Unsaved"
            } else {
                " · No local edits"
            }
        )));
        let mut list = rect().spacing(4.);
        for target in &project.paths {
            let target = target.clone();
            let caption = target
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            list = list.child(Button::new().on_press(move |_| {
                if !buffers.can_leave(files.peek().as_ref()) {
                    message.set("Save changes and apply or discard the draft before changing files.".into());
                    return;
                }
                let mut opened = files.write();
                if let Some(project) = opened.as_mut() {
                    match project.select(&target) {
                        Ok(()) => match buffers.load(project, dark) {
                            Ok(()) => message.set("File opened.".into()),
                            Err(error) => message.set(error),
                        },
                        Err(error) => message.set(error.to_string()),
                    }
                }
            }).child(label().text(caption)));
        }
        panel = panel.child(ScrollView::new().height(Size::px(72.)).child(list));
    } else {
        panel = panel.child(label().text("Example scene · Open a project to edit files."));
    }
    panel.child(label().text("Save before closing. Preview currently runs this file only, without project schema or cross-file links.")).into_element()
}
