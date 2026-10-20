//! Selection validation shared by history, links and native workspace adapters.
use super::{Location, Screen};
use recite_writer_model::{View, Workbench, WorkbenchError};

enum SourceEntry {
    Freeform,
    PreserveRules,
}

pub fn select(
    model: &mut Workbench,
    location: &Location,
) -> Result<Option<recite_writer_model::ReplyRules>, WorkbenchError> {
    select_with_source_entry(model, location, SourceEntry::Freeform)
}

fn select_with_source_entry(
    model: &mut Workbench,
    location: &Location,
    source: SourceEntry,
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
        Some(model.open_reply_rules(passage)?)
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
        if let View::Block(beat) = &view {
            model.inspect_block(beat)?;
        } else if model.view() != &view
            || (view == View::Source
                && location.screen != Screen::Rules
                && matches!(source, SourceEntry::Freeform))
        {
            model.select(view)?;
        }
    } else if model.view() == &View::Source {
        model.show_script()?;
    }
    Ok(rules)
}

pub fn select_reusing_rules(
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
    select_with_source_entry(model, &selection, SourceEntry::PreserveRules).map(|_| ())
}

pub fn catalogue_path(
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
