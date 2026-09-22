//! Commands call the same transition and persistence owners as visible controls.
use super::{Command, SearchMode};
use crate::{
    editing::{Pane, Writer},
    messages::{MsgId, text},
};
use freya::prelude::*;
use recite_config::WriterView;
use recite_writer_model::Workbench;
impl Command {
    pub fn run(self, mut writer: Writer) {
        writer.source_viewport.completion.set(false);
        if !self.enabled(writer) {
            writer
                .message
                .error(text(MsgId::WriterWorkspaceCommandUnavailable));
            return;
        }
        let result = match self {
            Self::Script | Self::Map | Self::Source => {
                writer.set_view(match self {
                    Self::Script => WriterView::Script,
                    Self::Map => WriterView::Map,
                    _ => WriterView::Source,
                });
                Ok(())
            }
            Self::Split => {
                let p = writer.preferences.peek().config.writer.presentation;
                writer.set_view(WriterView::Script);
                if *writer.layout.view.peek() == WriterView::Script {
                    crate::presentation::persist(writer, p.with_split(!p.split()));
                }
                Ok(())
            }
            Self::Focus => {
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
            Self::Commands => {
                writer.command_search.open(SearchMode::Commands);
                Ok(())
            }
            Self::GoTo => {
                writer.command_search.open(SearchMode::GoTo);
                Ok(())
            }
            Self::OpenProject => {
                if let Some(action) = writer.layout.project_controls.peek().clone() {
                    action.call(());
                }
                Ok(())
            }
            Self::Save => writer.buffers.save(writer.files),
            Self::SaveAll => writer.buffers.save_all(writer.files),
            Self::Apply => writer.try_perform(Workbench::apply),
            Self::Undo | Self::Redo => {
                if writer.try_navigate(|_| Ok(())).is_ok() {
                    crate::rename::history(writer, self == Self::Redo);
                }
                Ok(())
            }
            Self::AddBeat => writer
                .try_navigate(Workbench::add_beat)
                .map(|()| writer.pane.set(Pane::Script)),
            Self::AddLine => writer.try_navigate(Workbench::add_line),
            Self::AddReply => writer.try_navigate(Workbench::add_choice),
            Self::Preview => writer
                .try_navigate(Workbench::start_preview)
                .map(|()| writer.pane.set(Pane::Preview)),
            Self::Localise => crate::localisation::enter(writer),
            Self::Declarations => crate::declarations::try_open(writer),
            Self::Build => crate::builds::try_open(writer),
            Self::Rename => {
                crate::rename::open(writer);
                Ok(())
            }
            Self::Settings => {
                writer.settings_open.set(true);
                Ok(())
            }
        };
        if let Err(error) = result {
            writer.message.error(error);
        }
    }
}
