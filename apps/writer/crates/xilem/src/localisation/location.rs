//! Linked catalogue destinations resolve stable contexts against one captured owner.
mod preparing;
mod snapshot;
#[cfg(test)]
mod tests;
use super::{
    presentation::Presentation,
    slot::{Checkout, Slot},
    *,
};
use recite_writer_session::{
    ProjectSession,
    location::{Location, Screen as LinkedScreen},
};
use std::path::Path;

enum Installation {
    Keep,
    Close,
    Open(PathBuf),
}
enum Target {
    Hidden,
    Passage(beat::Selection),
    Queue,
    Entry(String),
    Compare,
    Updates(usize),
}
pub(crate) struct Plan {
    installation: Installation,
    target: Target,
    query: Query,
}
pub(crate) struct Work {
    plan: Plan,
    checkout: Checkout,
}
pub(crate) struct Prepared {
    checkout: Checkout,
    ready: Result<Ready, String>,
    retired: Option<Retired>,
}
struct Ready {
    installation: Install,
    projection: Projection,
    query: Query,
}
enum Install {
    Keep,
    Replace(Slot),
}
enum Projection {
    Hidden,
    Queue(Presentation),
    Entry(Presentation),
    Passage(Option<beat::Beat>),
    Compare(Option<Comparison>),
    Updates(Option<refresh::Refresh>),
}
struct Retired {
    _entry: Option<Entry>,
    _comparison: Option<Comparison>,
    _refresh: Option<refresh::Refresh>,
    _passage: Option<beat::Beat>,
}
impl Localisation {
    pub(crate) fn catalogue_path(&self) -> Option<&Path> {
        self.slot.path()
    }
    pub(crate) fn linked_catalogue_preflight(&self, changed: bool) -> Result<(), String> {
        if self.slot.read_only() || self.saving() {
            return Err("Finish the current catalogue action before opening a link.".into());
        }
        if changed && !self.can_leave() {
            return Err(
                "Save or discard translation drafts before following a link to another catalogue."
                    .into(),
            );
        }
        Ok(())
    }
}
impl Plan {
    pub(crate) fn capture(
        writer: &Writer,
        location: &Location,
        changed: bool,
        path: Option<PathBuf>,
    ) -> Result<Self, String> {
        writer.localisation.linked_catalogue_preflight(changed)?;
        let installation = if changed {
            path.map_or(Installation::Close, Installation::Open)
        } else {
            Installation::Keep
        };
        let target = match location.screen {
            LinkedScreen::Localise => Target::Passage(beat::Selection {
                name: location.beat.clone(),
                anchor: location.passage.clone(),
                page: location.page,
            }),
            LinkedScreen::Translations => Target::Queue,
            LinkedScreen::Entry => Target::Entry(
                location
                    .entry
                    .clone()
                    .ok_or("A translation link needs an entry context.")?,
            ),
            LinkedScreen::Compare => Target::Compare,
            LinkedScreen::Updates => Target::Updates(location.page),
            LinkedScreen::Write
            | LinkedScreen::Preview
            | LinkedScreen::Rules
            | LinkedScreen::Declarations
            | LinkedScreen::Build
            | LinkedScreen::Rename
            | LinkedScreen::Disk => Target::Hidden,
        };
        Ok(Self {
            installation,
            target,
            query: Query {
                text: location.query.clone(),
                filter: if location.attention {
                    Filter::Attention
                } else {
                    Filter::All
                },
                page: location.page,
            },
        })
    }
    pub(crate) fn needs_work(&self) -> bool {
        !matches!(self.installation, Installation::Keep) || !matches!(self.target, Target::Hidden)
    }
    pub(crate) fn checkout(self, writer: &mut Writer) -> Result<Work, String> {
        let selected = writer.localisation.mounted_entries();
        Ok(Work {
            plan: self,
            checkout: writer.localisation.slot.checkout(selected)?,
        })
    }
}
impl Work {
    pub(crate) fn prepare(
        mut self,
        project: &ProjectSession,
        control: &dyn recite_compiler::authoring::WorkControl,
    ) -> Prepared {
        let ready = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(owner) = self.checkout.owner.as_mut() { owner.flush_recovery()?; }
            self.plan.prepare(&self.checkout, project, control)
        })).unwrap_or_else(|_| {
            if let Some(owner) = self.checkout.owner.as_mut() { owner.quarantine(); }
            Err("Catalogue link preparation stopped unexpectedly. Reopen the catalogue before editing translations.".into())
        });
        Prepared {
            checkout: self.checkout,
            ready,
            retired: None,
        }
    }
    pub(crate) fn not_started(self, error: String) -> Prepared {
        Prepared {
            checkout: self.checkout,
            ready: Err(error),
            retired: None,
        }
    }
}
impl Prepared {
    pub(crate) fn restore(&mut self, writer: &mut Writer) -> Result<(), String> {
        writer.localisation.slot.restore(&mut self.checkout)
    }
    pub(crate) fn require_ready(&self) -> Result<(), String> {
        self.ready.as_ref().map(|_| ()).map_err(Clone::clone)
    }
    pub(crate) fn present(&mut self, writer: &mut Writer) -> Result<(), String> {
        let ready = self.ready.as_mut().map_err(|error| error.clone())?;
        if matches!(ready.installation, Install::Keep)
            && matches!(ready.projection, Projection::Hidden)
        {
            return Ok(());
        }
        if let Install::Replace(slot) = &mut ready.installation {
            std::mem::swap(&mut writer.localisation.slot, slot);
            writer.localisation.queue = Queue::default();
            writer.localisation.files_open = false;
            writer.localisation.destination = Destination::Queue;
        }
        let state = &mut writer.localisation;
        self.retired = Some(Retired {
            _entry: state.entry.take(),
            _comparison: state.comparison.take(),
            _refresh: state.refresh.take(),
            _passage: state.passage.take(),
        });
        state.query = ready.query.clone();
        match &mut ready.projection {
            Projection::Hidden => return Ok(()),
            Projection::Queue(presentation) | Projection::Entry(presentation) => {
                state.queue = std::mem::take(&mut presentation.queue);
                state.query.page = state.queue.page;
                std::mem::swap(&mut state.entry, &mut presentation.entry);
                state.destination = if state.entry.is_some() {
                    Destination::Entry
                } else {
                    Destination::Queue
                };
            }
            Projection::Passage(beat) => {
                state.passage = beat.take();
                state.destination = Destination::Passage;
            }
            Projection::Compare(comparison) => {
                state.comparison = Some(
                    comparison
                        .take()
                        .ok_or("The linked comparison was already accepted.")?,
                );
                state.destination = Destination::Comparison;
            }
            Projection::Updates(refresh) => {
                state.refresh = Some(
                    refresh
                        .take()
                        .ok_or("The linked source review was already accepted.")?,
                );
                state.destination = Destination::Refresh;
            }
        }
        writer.workspaces.screen = Screen::Localisation;
        Ok(())
    }
}
