//! Accepted scene/beat search survives its transient picker; requests carry fresh locations.
pub use crate::{Phase, Writer, navigation::Target};
use recite_writer_session::commands::{search::repeat_target, targets};

#[derive(Clone)]
pub struct Search {
    query: String,
    accepted: targets::Target,
}
impl Writer {
    pub(crate) fn lookup(&mut self, query: String, target: targets::Target) {
        let mut location = target.location(self.current_location());
        if self.workspaces.screen == crate::workspaces::Screen::Localisation {
            location.screen = recite_writer_session::location::Screen::Localise;
            location.page = 0;
        }
        self.navigate(Target::Lookup {
            location,
            search: Search {
                query,
                accepted: target,
            },
        });
    }
    pub(crate) fn repeat_lookup(&mut self, forward: bool) {
        if self.phase != Phase::Writing || self.navigation.busy() || self.work.busy() {
            return;
        }
        let Some(search) = &self.lookup else {
            self.status = "Choose a scene or beat from a search before repeating it.".into();
            return;
        };
        let targets = if self.session.project().is_some() {
            Ok(targets::project(self.session.model()))
        } else {
            targets::examples(self.session.model())
        };
        let targets = match targets {
            Ok(targets) => targets,
            Err(error) => {
                self.status = error.to_string();
                return;
            }
        };
        let matches = targets::matching(targets, &search.query);
        let Some(target) = repeat_target(&matches, Some(&search.accepted), forward).cloned() else {
            self.status = "No current scene or beat matches the previous search.".into();
            return;
        };
        self.lookup(search.query.clone(), target);
    }
    pub(crate) fn accept_lookup(&mut self, search: &Search) {
        if !search.query.trim().is_empty() {
            self.lookup = Some(search.clone());
        }
    }
}

#[cfg(test)]
mod tests;
