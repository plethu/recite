//! Destination choices capture the owning reply or beat before the checked structural edit.
use crate::{Phase, Writer};
use recite_writer_model::{NavigationStamp, RouteOwner, StructuralCommand};
use recite_writer_session::choices::{self, PickerOption};
#[cfg(test)]
mod tests;

pub struct Request {
    window: crate::WindowKey,
    origin: NavigationStamp,
    owner: RouteOwner,
    pub choices: Vec<PickerOption>,
}
impl Request {
    pub fn is_current(&self, writer: &Writer) -> bool {
        self.window == writer.window
            && self.origin.is_current(writer.session.model())
            && writer.phase == Phase::Writing
            && !writer.work.busy()
            && !writer.navigation.busy()
    }
    pub fn apply(self, writer: &mut Writer, destination: String) {
        if !self.is_current(writer)
            || !self
                .choices
                .iter()
                .any(|option| option.value == destination)
        {
            writer.status = "This route changed. Reopen its destination choice to continue.".into();
            return;
        }
        writer.navigate(crate::navigation::Target::Structure(
            StructuralCommand::Destination {
                owner: self.owner,
                destination,
            },
        ));
    }
}
impl Writer {
    pub fn choose_destination(&mut self, owner: RouteOwner) {
        if self.phase != Phase::Writing || self.navigation.busy() || self.work.busy() {
            return;
        }
        let choices = choices::destinations(self.session.model().document().sections());
        self.pending_choice = Some(crate::choices::Request::Destination(Request {
            window: self.window,
            origin: self.session.model().navigation_stamp(),
            owner,
            choices,
        }));
    }
}
