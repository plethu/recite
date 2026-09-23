//! Commands call the same transition and persistence owners as visible controls.
use super::{Command, SearchMode};
use crate::commands::CommandExt;
use crate::{
    editing::{Pane, Writer},
    messages::{MsgId, text},
};
use freya::prelude::*;
use recite_config::WriterView;
use recite_writer_model::Workbench;
pub(super) fn run(command: Command, mut writer: Writer) {
    writer.source_viewport.completion.set(false);
    if !command.enabled(writer) {
        writer
            .message
            .error(text(MsgId::WriterWorkspaceCommandUnavailable));
        return;
    }
    let result = match command {
        Command::Script | Command::Map | Command::Source => {
            writer.set_view(match command {
                Command::Script => WriterView::Script,
                Command::Map => WriterView::Map,
                _ => WriterView::Source,
            });
            Ok(())
        }
        Command::Split => {
            let p = writer.preferences.peek().config.writer.presentation;
            writer.set_view(WriterView::Script);
            if *writer.layout.view.peek() == WriterView::Script {
                crate::presentation::persist(writer, p.with_split(!p.split()));
            }
            Ok(())
        }
        Command::Focus => {
            let next = !*writer.layout.focus.peek();
            if next {
                let previous = (
                    *writer.pane.peek(),
                    *writer.layout.view.peek(),
                    *Platform::get().focused_accessibility_id.peek(),
                );
                if let Err(error) = writer.ensure_beat() {
                    writer.message.error(error);
                    return;
                }
                writer.layout.before_focus.set(Some(previous));
            } else if let Some((pane, view, focus)) = *writer.layout.before_focus.peek() {
                if view == WriterView::Source
                    && let Err(error) =
                        writer.try_navigate(|m| m.select(recite_writer_model::View::Source))
                {
                    writer.message.error(error);
                    return;
                }
                writer.pane.set(pane);
                writer.layout.view.set(view);
                focus.request_focus();
            }
            writer.layout.focus.set(next);
            if next {
                writer.inspector_focus.request_focus();
            }
            Ok(())
        }
        Command::Commands => {
            writer.command_search.open(SearchMode::Commands);
            Ok(())
        }
        Command::GoTo => {
            writer.command_search.open(SearchMode::GoTo);
            Ok(())
        }
        Command::OpenProject => {
            if let Some(action) = writer.layout.project_controls.peek().clone() {
                action.call(());
            }
            Ok(())
        }
        Command::Save => writer.buffers.save(writer.files),
        Command::SaveAll => writer.buffers.save_all(writer.files),
        Command::Apply => writer.try_perform(Workbench::apply),
        Command::Undo | Command::Redo => {
            if writer.try_navigate(|_| Ok(())).is_ok() {
                crate::rename::history(writer, command == Command::Redo);
            }
            Ok(())
        }
        Command::AddBeat => writer
            .try_navigate(Workbench::add_beat)
            .map(|()| writer.pane.set(Pane::Script)),
        Command::AddLine => writer.try_navigate(Workbench::add_line),
        Command::AddReply => writer.try_navigate(Workbench::add_choice),
        Command::Preview => writer
            .try_navigate(Workbench::start_preview)
            .map(|()| writer.pane.set(Pane::Preview)),
        Command::Localise => crate::localisation::enter(writer),
        Command::Declarations => crate::declarations::try_open(writer),
        Command::Build => crate::builds::try_open(writer),
        Command::Rename => {
            crate::rename::open(writer);
            Ok(())
        }
        Command::Back | Command::Forward => {
            writer.history_step(command == Command::Forward);
            Ok(())
        }
        Command::Find => {
            writer.layout.focus.set(false);
            writer.layout.navigation.set(true);
            writer.vim.insert_requested.set(true);
            Ok(())
        }
        Command::NextMatch | Command::PreviousMatch => {
            writer.vim.step(writer, command == Command::NextMatch);
            Ok(())
        }
        Command::PaneLeft | Command::PaneRight | Command::PaneUp | Command::PaneDown => {
            let (dx, dy) = match command {
                Command::PaneLeft => (-1., 0.),
                Command::PaneRight => (1., 0.),
                Command::PaneUp => (0., -1.),
                _ => (0., 1.),
            };
            writer.vim.focus(writer, dx, dy);
            Ok(())
        }
        Command::Close => {
            crate::closing::close();
            Ok(())
        }
        Command::Settings => {
            crate::settings::open(writer, *Platform::get().focused_accessibility_id.peek());
            Ok(())
        }
    };
    if let Err(error) = result {
        writer.message.error(error);
    }
}
