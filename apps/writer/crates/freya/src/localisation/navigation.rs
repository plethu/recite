//! Deliver a shared translation destination through Freya navigation.
use crate::editing::Writer;
pub(super) use recite_writer_session::localisation::navigation::Destination;
pub(super) fn open(
    destination: &Destination,
    writer: Writer,
    completed: freya::prelude::EventHandler<Result<(), String>>,
) {
    crate::navigation::open_passage(
        writer,
        destination.document.as_deref(),
        &destination.beat,
        &destination.anchor,
        completed,
    )
}
