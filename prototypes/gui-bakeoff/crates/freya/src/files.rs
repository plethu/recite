use crate::{
    editing::{Session, editor_data},
    project::ProjectFiles,
};
use freya::{code_editor::CodeEditorData, prelude::*};
use recite_bakeoff_authoring::View;

#[derive(Clone, Copy)]
pub(crate) struct Buffers {
    pub model: Session,
    pub editor: State<CodeEditorData>,
    pub prose: State<String>,
}

impl Buffers {
    pub(super) fn load(self, project: &mut ProjectFiles, dark: bool) -> Result<(), String> {
        let next = project.workbench().map_err(|e| e.to_string())?;
        self.install(next, dark);
        Ok(())
    }

    fn install(mut self, next: recite_bakeoff_authoring::Workbench, dark: bool) {
        self.editor.set(editor_data(
            next.draft(),
            next.view() == &View::Source,
            dark,
        ));
        self.prose.set(next.draft().to_owned());
        self.model.set(Ok(next));
    }

    pub(super) fn harvest(mut self) {
        if let Ok(model) = self.model.write().as_mut() {
            model.set_draft(if model.view() == &View::Source {
                self.editor.peek().rope.to_string()
            } else {
                self.prose.peek().clone()
            });
        }
    }

    pub(super) fn save(self, mut files: State<Option<ProjectFiles>>) -> Result<(), String> {
        self.harvest();
        let mut model = self.model;
        let mut state = model.write();
        let workbench = state.as_mut().map_err(|error| error.to_string())?;
        let mut opened = files.write();
        let project = opened.as_mut().ok_or("Open a project before saving.")?;
        workbench.apply().map_err(|error| error.to_string())?;
        project
            .save(workbench.document().source())
            .map_err(|error| error.to_string())?;
        project
            .checkpoint(workbench)
            .map_err(|error| error.to_string())
    }

    pub(super) fn can_leave(self, files: Option<&ProjectFiles>) -> bool {
        self.harvest();
        self.model
            .peek()
            .as_ref()
            .is_ok_and(|m| !m.has_draft() && files.is_none_or(|f| !f.dirty(m.document().source())))
    }
}

pub(crate) struct FileChrome {
    pub actions: Element,
    pub project_panel: Element,
    pub scenes: Element,
    pub status: String,
    pub close_prompt: Element,
}

pub(crate) fn controls(buffers: Buffers, mut message: State<String>, dark: bool) -> FileChrome {
    let path = use_state(|| {
        if std::env::args().nth(1).as_deref() == Some("--project") {
            std::env::args().nth(2).unwrap_or_default()
        } else {
            String::new()
        }
    });
    let mut files = use_state(|| None::<ProjectFiles>);
    let mut project_panel_open = use_state(|| path.peek().is_empty());
    use_hook(move || {
        if !path.peek().is_empty() {
            let result = ProjectFiles::open(std::path::Path::new(path.peek().as_str()));
            match result {
                Ok(mut project) => match buffers.load(&mut project, dark) {
                    Ok(()) => {
                        files.set(Some(project));
                    }
                    Err(error) => {
                        message.set(error);
                        project_panel_open.set(true);
                    }
                },
                Err(error) => {
                    message.set(error.to_string());
                    project_panel_open.set(true);
                }
            }
        }
    });
    use_side_effect(move || {
        let state = buffers.model.read();
        let Ok(workbench) = state.as_ref() else {
            return;
        };
        if let Some(project) = files.write().as_mut()
            && let Err(error) = project.checkpoint(workbench)
        {
            message.set(format!(
                "Draft recovery failed: {error}. Keep this window open and retry Save."
            ));
        }
    });
    let close_prompt = crate::closing::controls(buffers, files);
    let current = files.read();
    let actions = rect()
        .horizontal()
        .spacing(8.)
        .on_global_key_down(move |event: Event<KeyboardEventData>| {
            if event.code == Code::KeyS
                && (event.modifiers.contains(Modifiers::CONTROL)
                    || event.modifiers.contains(Modifiers::META))
            {
                message.set(match buffers.save(files) {
                    Ok(()) => "Saved.".into(),
                    Err(error) => error,
                });
            }
        })
        .child(
            Button::new()
                .cursor_icon(CursorIcon::Pointer)
                .flat()
                .on_press(move |_| {
                    let next = !*project_panel_open.peek();
                    project_panel_open.set(next);
                })
                .child("Project"),
        )
        .child(
            Button::new()
                .cursor_icon(CursorIcon::Pointer)
                .filled()
                .enabled(current.is_some())
                .on_press(move |_| {
                    message.set(match buffers.save(files) {
                        Ok(()) => "Saved.".into(),
                        Err(error) => error,
                    });
                })
                .child("Save"),
        )
        .into_element();
    let mut panel = rect().width(Size::fill()).spacing(12.).padding(16.);
    if *project_panel_open.read() {
        panel = panel.child(rect().horizontal().spacing(8.)
            .child(Input::new(path).width(Size::px(440.)).placeholder("Project folder or recite.project.toml"))
            .child(Button::new().cursor_icon(CursorIcon::Pointer).on_press(move |_| {
                if !buffers.can_leave(files.peek().as_ref()) {
                    message.set("Save changes and apply or discard the draft before opening another project.".into());
                    return;
                }
                match ProjectFiles::open(std::path::Path::new(path.peek().as_str())) {
                    Ok(mut project) => match buffers.load(&mut project, dark) {
                        Ok(()) => {
                            let recovered = project.has_recovery();
                            files.set(Some(project));
                            project_panel_open.set(false);
                            message.set(if recovered { "Recovered your previous session." } else { "Project opened." }.into());
                        },
                        Err(error) => message.set(error),
                    },
                    Err(error) => message.set(error.to_string()),
                }
            }).child("Open project")));
        if let Some(project) = current.as_ref() {
            panel = panel.child(label().text(project.current.display().to_string()).color(crate::palette::muted(dark)))
                .child(rect().horizontal().spacing(8.)
                    .child(Button::new().cursor_icon(CursorIcon::Pointer).flat().on_press(move |_| {
                        buffers.harvest();
                        let mut model = buffers.model;
                        let mut state = model.write();
                        if let Ok(workbench) = state.as_mut() && let Some(project) = files.write().as_mut() {
                            message.set(match project.refresh(workbench) {
                                Ok(()) => "Project refreshed. Restart preview to use the updated sources and schema.".into(),
                                Err(error) => error.to_string(),
                            });
                        }
                    }).child("Refresh project context"))
                    .child(Button::new().cursor_icon(CursorIcon::Pointer).flat().on_press(move |_| {
                        buffers.harvest();
                        let state = buffers.model.peek();
                        let Ok(workbench) = state.as_ref() else { return; };
                        let result = files.write().as_mut().map(|project| project.reload(workbench));
                        drop(state);
                        if let Some(result) = result {
                            match result {
                                Ok((copy, next)) => {
                                    buffers.install(next, dark);
                                    message.set(format!("Loaded the disk version. Your previous session is in {}", copy.display()));
                                },
                                Err(error) => message.set(error.to_string()),
                            }
                        }
                    }).child("Keep recovery copy and reload disk")));
        }
    }
    let project_panel = if *project_panel_open.read() {
        panel.into_element()
    } else {
        rect().into_element()
    };
    let mut scenes = rect().width(Size::fill()).spacing(4.);
    let status = if let Some(project) = current.as_ref() {
        for target in &project.paths {
            let target = target.clone();
            let caption = crate::palette::display_name(&target.file_name().unwrap_or_default().to_string_lossy());
            scenes = scenes.child(crate::palette::navigation_button(target == project.current, dark).on_press(move |_| {
                if !buffers.can_leave(files.peek().as_ref()) {
                    message.set("Save changes and apply or discard the draft before changing files.".into());
                    return;
                }
                if let Some(project) = files.write().as_mut() {
                    match project.select(&target) {
                        Ok(next) => { buffers.install(next, dark); message.set("Scene opened.".into()); },
                        Err(error) => message.set(error.to_string()),
                    }
                }
            }).child(label().text(caption)));
        }
        let dirty = buffers.model.read().as_ref().is_ok_and(|m| m.has_draft() || project.dirty(m.document().source()));
        if dirty { "Unsaved changes" } else { "Saved to disk" }
    } else { "Example · Open a project to save" }.to_owned();
    FileChrome {
        actions,
        project_panel,
        scenes: scenes.into_element(),
        status,
        close_prompt,
    }
}
