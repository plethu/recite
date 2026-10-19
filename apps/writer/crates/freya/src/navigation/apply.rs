//! Validate and apply one location to document, catalogue and presentation owners.
use super::location::{Location, Screen};
use crate::editing::{Pane, Writer};
use freya::prelude::*;
use recite_writer_model::{View, Workbench, WorkbenchError};

pub(super) fn select(
    model: &mut Workbench,
    location: &Location,
) -> Result<Option<recite_writer_model::ReplyRules>, WorkbenchError> {
    if let (Some(passage), Some(beat)) = (&location.passage, &location.beat)
        && model
            .document()
            .find_passage(passage)?
            .is_none_or(|p| &p.section != beat)
    {
        return Err(recite_writer_model::EditError::Destination.into());
    }
    let rules = if location.screen == Screen::Rules {
        if model.has_draft() {
            return Err(WorkbenchError::DraftPending);
        }
        let passage = location
            .passage
            .as_deref()
            .ok_or(recite_writer_model::EditError::Destination)?;
        Some(model.document().reply_rules(passage)?)
    } else {
        None
    };
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
    } else if model.view() == &View::Source {
        model.show_script()?;
    }
    Ok(rules)
}

pub(super) fn select_reusing_rules(
    model: &mut Workbench,
    location: &Location,
) -> Result<(), WorkbenchError> {
    let mut selection = location.clone();
    selection.screen = Screen::Write;
    selection.source = true;
    selection.passage = None;
    // Validate the linked beat/passage pair even when preserving staged Rules.
    if let (Some(passage), Some(beat)) = (&location.passage, &location.beat)
        && model
            .document()
            .find_passage(passage)?
            .is_none_or(|p| &p.section != beat)
    {
        return Err(recite_writer_model::EditError::Destination.into());
    }
    select(model, &selection).map(|_| ())
}

pub(super) fn present(mut writer: Writer, location: &Location) -> Result<(), String> {
    {
        let mut state = writer.localisation.write();
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

#[cfg(test)]
mod tests;
