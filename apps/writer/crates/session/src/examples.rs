//! Temporary example models retain drafts and history without acquiring files.
use recite_compiler::authoring::{CancellationToken, WorkControl};
use recite_writer_model::{
    EditError, NavigationRequest, PreparedNavigation, WRITER_EXAMPLES, Workbench, WorkbenchError,
};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct ExampleSessions {
    retained: BTreeMap<String, Workbench>,
}
pub struct SwitchRequest {
    origin: NavigationRequest,
    name: String,
    retained: Option<NavigationRequest>,
}
enum Destination {
    Retained(Box<PreparedNavigation>),
    Cold(Option<Box<Workbench>>),
}
pub struct PreparedSwitch {
    origin: NavigationRequest,
    name: String,
    destination: Destination,
    accepted: bool,
}
impl ExampleSessions {
    pub fn dirty(&self, current: &Workbench) -> bool {
        std::iter::once(current)
            .chain(self.retained.values())
            .any(Self::is_modified)
    }
    pub fn is_modified(model: &Workbench) -> bool {
        model.has_draft()
            || WRITER_EXAMPLES
                .iter()
                .find(|example| example.name == model.document().key().as_str())
                .is_none_or(|example| model.document().source() != example.source)
    }

    pub fn request(
        &self,
        current: &Workbench,
        name: &str,
    ) -> Result<SwitchRequest, WorkbenchError> {
        if current.document().key().as_str() == name
            || !WRITER_EXAMPLES.iter().any(|example| example.name == name)
        {
            return Err(EditError::Destination.into());
        }
        Ok(SwitchRequest {
            origin: current.navigation_request(),
            name: name.into(),
            retained: self.retained.get(name).map(Workbench::navigation_request),
        })
    }
    pub fn accept(
        &mut self,
        current: &mut Workbench,
        prepared: &mut PreparedSwitch,
    ) -> Result<(), WorkbenchError> {
        if !prepared.is_current(self, current) {
            return Err(EditError::Stale.into());
        }
        let next = match &mut prepared.destination {
            Destination::Retained(selection) => {
                self.retained
                    .get_mut(&prepared.name)
                    .ok_or(EditError::Destination)?
                    .accept_navigation(selection)?;
                self.retained
                    .remove(&prepared.name)
                    .ok_or(EditError::Destination)?
            }
            Destination::Cold(next) => *next.take().ok_or(EditError::Stale)?,
        };
        let previous = std::mem::replace(current, next);
        self.retained
            .insert(previous.document().key().to_string(), previous);
        prepared.accepted = true;
        Ok(())
    }
    pub fn switch(
        &mut self,
        current: &mut Workbench,
        index: usize,
        select: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
    ) -> Result<(), WorkbenchError> {
        let example = WRITER_EXAMPLES.get(index).ok_or(EditError::Destination)?;
        if current.document().key().as_str() == example.name {
            return Ok(());
        }
        let mut prepared = self
            .request(current, example.name)?
            .prepare(&CancellationToken::new(), select)?;
        self.accept(current, &mut prepared)
    }
}
impl SwitchRequest {
    pub fn prepare(
        self,
        control: &dyn WorkControl,
        select: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
    ) -> Result<PreparedSwitch, WorkbenchError> {
        let destination = if let Some(request) = self.retained {
            let mut prepared = request.prepare(None, control)?;
            select(prepared.model_mut())?;
            Destination::Retained(Box::new(prepared))
        } else {
            let example = WRITER_EXAMPLES
                .iter()
                .find(|example| example.name == self.name)
                .ok_or(EditError::Destination)?;
            let mut next = example.open()?;
            select(&mut next)?;
            Destination::Cold(Some(Box::new(next)))
        };
        Ok(PreparedSwitch {
            origin: self.origin,
            name: self.name,
            destination,
            accepted: false,
        })
    }
}
impl PreparedSwitch {
    pub fn model(&self) -> Option<&Workbench> {
        if self.accepted {
            return None;
        }
        match &self.destination {
            Destination::Retained(prepared) => Some(prepared.model()),
            Destination::Cold(next) => next.as_deref(),
        }
    }
    pub fn is_current(&self, sessions: &ExampleSessions, current: &Workbench) -> bool {
        !self.accepted
            && self.origin.is_current(current)
            && match &self.destination {
                Destination::Retained(prepared) => sessions
                    .retained
                    .get(&self.name)
                    .is_some_and(|model| prepared.is_current(model)),
                Destination::Cold(next) => {
                    next.is_some() && !sessions.retained.contains_key(&self.name)
                }
            }
    }
    pub fn refresh_rules(
        &self,
        rules: &mut recite_writer_model::ReplyRules,
        current: &Workbench,
    ) -> bool {
        match &self.destination {
            Destination::Retained(prepared) => prepared.refresh_rules(rules, current.document()),
            Destination::Cold(_) => false,
        }
    }
}
#[cfg(test)]
mod tests;
