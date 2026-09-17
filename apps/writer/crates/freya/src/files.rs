use crate::design::Button;
use crate::design::tokens as t;
use crate::project::ProjectFiles;
use freya::prelude::*;

pub(crate) struct FileChrome {
    pub actions: Element,
    pub project_panel: Element,
    pub scenes: Element,
    pub status: String,
    pub files: State<Option<ProjectFiles>>,
}

pub(crate) fn controls(
    writer: crate::editing::Writer,
    mut message: crate::feedback::Feedback,
    dark: bool,
) -> FileChrome {
    let buffers = writer.buffers;
    let path = use_state(|| {
        std::env::args()
            .collect::<Vec<_>>()
            .windows(2)
            .find(|args| args[0] == "--project")
            .map_or_else(String::new, |args| args[1].clone())
    });
    let path_id = use_a11y();
    let browse_id = use_a11y();
    let mut files = writer.files;
    let mut project_panel_open = use_state(|| path.peek().is_empty());
    let job = use_state(|| None::<crate::project_loading::LoadJob>);
    use_hook(move || {
        if !path.peek().is_empty() {
            crate::project_loading::start(
                job,
                path.peek().as_str().into(),
                message,
                writer.buffers,
            );
        }
    });
    use_side_effect(move || {
        let state = buffers.model.read();
        let Ok(workbench) = state.as_ref() else {
            return;
        };
        if let Some(project) = files.write().as_mut()
            && let Err(error) = project.queue_checkpoint(workbench)
        {
            message.error_with_action(
                format!("Draft recovery failed: {error}. Keep this window open and retry Save."),
                "Retry save".into(),
                EventHandler::new(move |()| message.report(buffers.save(files), "Saved.".into())),
            );
        }
    });
    let current = files.read();
    let actions = rect()
        .horizontal()
        .spacing(t::SPACE_SM)
        .on_global_key_down(move |event: Event<KeyboardEventData>| {
            if crate::editing::is_save_key(&event) {
                message.report(buffers.save(files), "Saved.".into());
            }
        })
        .child(
            Button::new()
                .flat()
                .on_press(move |_| {
                    let next = !*project_panel_open.peek();
                    project_panel_open.set(next);
                })
                .child("Project"),
        )
        .child(
            Button::new()
                .filled()
                .enabled(current.is_some())
                .on_press(move |_| {
                    message.report(buffers.save(files), "Saved.".into());
                })
                .child("Save"),
        )
        .child(crate::recovery::status::RecoveryStatus { files })
        .child(crate::project_loading::Loading {
            writer,
            files,
            job,
            panel: project_panel_open,
        })
        .into_element();
    let mut panel = rect()
        .width(Size::fill())
        .spacing(t::SPACE_MD)
        .padding(t::SPACE_LG);
    if *project_panel_open.read() {
        let open = EventHandler::new(move |()| {
            if !buffers.can_leave(files.peek().as_ref()) {
                message.error(
                    "Save changes and apply or discard the draft before opening another project."
                        .into(),
                );
                return;
            }
            crate::project_loading::start(
                job,
                path.peek().as_str().into(),
                message,
                writer.buffers,
            );
        });
        let submit = open.clone();
        panel = panel.child(
            rect()
                .horizontal()
                .content(Content::Flex)
                .width(Size::fill())
                .spacing(t::SPACE_SM)
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .child(crate::design::PathField {
                            value: path,
                            id: path_id,
                            browse_id,
                            kind: crate::design::PathKind::Project,
                            enabled: job.read().is_none(),
                            submit,
                        }),
                )
                .child(
                    Button::new()
                        .enabled(!path.read().trim().is_empty() && job.read().is_none())
                        .on_press(move |_| open.call(()))
                        .child("Open project"),
                ),
        );
        if let Some(project) = current.as_ref() {
            panel = panel.child(label().text(project.current.display().to_string()).color(crate::palette::muted(dark)))
                .child(rect().horizontal().spacing(t::SPACE_SM)
                    .child(Button::new().flat().on_press(move |_| {
                        buffers.harvest();
                        let mut model = buffers.model;
                        let mut state = model.write();
                        if let Ok(workbench) = state.as_mut() && let Some(project) = files.write().as_mut() {
                            message.report(project.refresh(workbench).map_err(|error| error.to_string()), "Project refreshed. Restart preview to use the updated sources and schema.".into());
                        }
                    }).child("Refresh project context"))
                    .child(Button::new().flat().on_press(move |_| {
                        buffers.harvest();
                        let state = buffers.model.peek();
                        let Ok(workbench) = state.as_ref() else { return; };
                        let result = files.write().as_mut().map(|project| project.reload(workbench));
                        drop(state);
                        if let Some(result) = result {
                            match result {
                                Ok((copy, next)) => {
                                    buffers.install(next, dark);
                                    message.info(format!("Loaded the disk version. Your previous session is in {}", copy.display()));
                                },
                                Err(error) => message.error(error.to_string()),
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
    let mut scenes = Vec::new();
    let status = if let Some(project) = current.as_ref() {
        for target in &project.paths {
            let target = target.clone();
            let caption = crate::palette::display_name(
                &target.file_name().unwrap_or_default().to_string_lossy(),
            );
            scenes.push(crate::scene_navigation::SceneBranch {
                caption,
                active: target == project.current,
                open: EventHandler::new(move |()| {
                    if !buffers.can_leave(files.peek().as_ref()) {
                        message.error(
                            "Save changes and apply or discard the draft before changing files."
                                .into(),
                        );
                        return;
                    }
                    if let Some(project) = files.write().as_mut() {
                        match project.select(&target) {
                            Ok(next) => {
                                buffers.install(next, dark);
                                if writer.scene_opened().is_ok() {
                                    message.info("Scene opened.".into());
                                }
                            }
                            Err(error) => message.error(error.to_string()),
                        }
                    }
                }),
            });
        }
        let dirty = buffers
            .model
            .read()
            .as_ref()
            .is_ok_and(|m| m.has_draft() || project.dirty(m.document().source()));
        if dirty {
            "Unsaved changes"
        } else {
            "Saved to disk"
        }
    } else {
        "Example · Open a project to save"
    }
    .to_owned();
    FileChrome {
        actions,
        project_panel,
        scenes: rect()
            .width(Size::fill())
            .height(Size::flex(1.))
            .content(Content::Flex)
            .child(crate::project_search::ProjectSearch { writer, files })
            .child(crate::scene_navigation::SceneNavigation { writer, scenes })
            .into_element(),
        status,
        files,
    }
}
