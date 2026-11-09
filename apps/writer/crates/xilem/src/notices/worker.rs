//! Frontend views and native glue over `recite_writer_app::notices::worker`.
use super::Writer;
use xilem::{
    ViewCtx,
    core::{NoElement, View},
};

pub(crate) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::task(
        |proxy, writer: &mut Writer| {
            writer.connect_external_notices(move |output| proxy.message(output).is_ok())
        },
        Writer::accept_external_notice,
    )
}
