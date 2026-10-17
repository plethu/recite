use super::location::{Location, Screen};
use crate::editing::{Pane, Writer};
use freya::prelude::{AccessibilityIdExt, WritableUtils};
use recite_writer_model::{View, Workbench, WorkbenchError};

pub(super) fn select(model: &mut Workbench, location: &Location) -> Result<(), WorkbenchError> {
    if let (Some(passage), Some(beat)) = (&location.passage, &location.beat)
        && model
            .document()
            .find_passage(passage)?
            .is_none_or(|p| &p.section != beat)
    {
        return Err(recite_writer_model::EditError::Destination.into());
    }
    if location.screen == Screen::Rules {
        let passage = location
            .passage
            .as_deref()
            .ok_or(recite_writer_model::EditError::Destination)?;
        model.document().reply_rules(passage)?;
    }
    let view = if location.source || location.screen == Screen::Rules {
        Some(View::Source)
    } else if let Some(passage) = &location.passage {
        Some(View::Passage(passage.clone()))
    } else {
        location.beat.as_ref().map(|beat| View::Block(beat.clone()))
    };
    if let Some(view) = view {
        if model.view() != &view {
            model.select(view)?;
        }
    } else if model.view() == &View::Source && !crate::scene::has_no_beats(model) {
        model.show_script()?;
    }
    Ok(())
}

pub(super) fn apply(mut writer: Writer, location: &Location) -> Result<(), String> {
    let files = writer.files.peek();
    if let Some(project) = &location.project {
        let linked_root = recite_config::discover_project(project)
            .map_err(|error| format!("The linked project is unavailable: {error}"))?
            .manifest()
            .project_root()
            .to_owned();
        if files.as_ref().is_none_or(|f| f.root() != linked_root) {
            return Err(format!("Open the linked project first: {project}"));
        }
    }
    let path = location
        .catalogue
        .as_ref()
        .map(|catalogue| {
            files.as_ref().map_or_else(
                || Ok(std::path::PathBuf::from(catalogue)),
                |files| catalogue_path(files.root(), catalogue),
            )
        })
        .transpose()?;
    drop(files);
    writer.remember_scene();
    writer.try_navigate(|_| Ok(()))?;
    let current_catalogue = writer
        .localisation
        .peek()
        .catalogue
        .as_ref()
        .map(|c| c.path.clone());
    let catalogue_changed = current_catalogue != path;
    let catalogue = if catalogue_changed {
        if writer.localisation.peek().dirty() {
            return Err(crate::localisation::close_drafts_message());
        }
        if let Some(current) = writer.localisation.write().catalogue.as_mut() {
            current.flush_recovery()?;
        }
        path.as_ref()
            .map(|p| crate::localisation::catalogue::Catalogue::open_recoverable(p))
            .transpose()?
    } else {
        None
    };
    let current = writer
        .buffers
        .model
        .peek()
        .as_ref()
        .map_err(|e| e.to_string())?
        .document()
        .key()
        .to_string();
    if !location.document.is_empty() && location.document != current {
        if writer.files.peek().is_some() {
            let path = writer
                .files
                .peek()
                .as_ref()
                .and_then(|project| project.path_for_document(&location.document))
                .ok_or("The linked scene is not in this project.")?;
            writer
                .buffers
                .switch(writer.files, &path, writer.dark, |m| select(m, location))?;
        } else {
            let index = recite_writer_model::WRITER_EXAMPLES
                .iter()
                .position(|e| e.name == location.document)
                .ok_or("The linked scene is not open.")?;
            crate::examples::select_at(writer, index, |model| select(model, location))?;
        }
    } else {
        writer.try_navigate(|m| select(m, location))?;
    }
    {
        let mut state = writer.localisation.write();
        if catalogue_changed {
            state.install(catalogue)?;
        }
        state.active = !matches!(
            location.screen,
            Screen::Write
                | Screen::Preview
                | Screen::Rules
                | Screen::Declarations
                | Screen::Build
                | Screen::Rename
                | Screen::Disk
        );
        state.entry_context = location.entry.clone();
        state.view = match location.screen {
            Screen::Translations => crate::localisation::CatalogueView::Queue,
            Screen::Updates => crate::localisation::CatalogueView::Updates,
            Screen::Compare => crate::localisation::CatalogueView::Compare,
            Screen::Entry => crate::localisation::CatalogueView::Entry,
            _ => crate::localisation::CatalogueView::Passage,
        };
        if location.screen == Screen::Updates {
            state.update_index = location.page;
        }
    }
    writer.queue.search.set(location.query.clone());
    writer.queue.attention.set(location.attention);
    if location.screen != Screen::Updates {
        writer.queue.page.set(location.page);
    }
    if location.screen == Screen::Rules {
        let passage = location
            .passage
            .as_deref()
            .ok_or("A reply-rules link needs a reply.")?;
        let retained = writer.buffers.rules.peek().as_ref().is_some_and(|r| {
            r.passage == passage
                && writer
                    .buffers
                    .model
                    .peek()
                    .as_ref()
                    .is_ok_and(|m| r.belongs_to(m.document()))
                && r.source()
                    .is_ok_and(|source| source == writer.buffers.editor.peek().rope)
        });
        if !retained {
            crate::rules::open(writer, passage)?;
        }
    }
    if location.screen == Screen::Declarations {
        crate::declarations::try_open(writer)?;
    }
    if location.screen == Screen::Build {
        crate::builds::try_open(writer)?;
    }
    if location.screen == Screen::Rename {
        crate::rename::open(writer);
    }
    if location.screen == Screen::Disk {
        crate::external::try_open(writer)?;
    }
    writer.pane.set(match location.screen {
        Screen::Disk => Pane::Disk,
        Screen::Rename => Pane::Rename,
        Screen::Build => Pane::Build,
        Screen::Declarations => Pane::Declarations,
        Screen::Rules => Pane::Rules,
        Screen::Preview => Pane::Preview,
        _ if location.beat.is_some() || location.passage.is_some() => Pane::Script,
        _ => Pane::Map,
    });
    writer.layout.view.set(if location.source {
        recite_config::WriterView::Source
    } else {
        location.view.unwrap_or(recite_config::WriterView::Map)
    });
    writer.selection.set(location.beat.clone());
    writer.inspector_focus.request_focus();
    writer.restore_scroll();
    Ok(())
}

pub(super) fn catalogue_path(
    root: &std::path::Path,
    relative: &str,
) -> Result<std::path::PathBuf, String> {
    use std::path::Component;
    let relative = std::path::Path::new(relative);
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err("The linked catalogue must be relative to the project.".into());
    }
    let path = std::fs::canonicalize(root.join(relative))
        .map_err(|error| format!("The linked catalogue is unavailable: {error}"))?;
    if !path.starts_with(root) {
        return Err("The linked catalogue is outside the project.".into());
    }
    Ok(path)
}
