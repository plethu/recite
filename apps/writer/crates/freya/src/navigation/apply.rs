//! Validate and apply one location to document, catalogue and presentation owners.
use super::location::{Location, Screen};
use crate::editing::{Pane, Writer};
use freya::prelude::*;

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
