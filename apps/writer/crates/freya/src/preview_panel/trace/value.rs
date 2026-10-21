use super::*;
use crate::design::tokens::PrintTypography;
use recite_writer_session::preview::trace::{FIELDS, ValuePage};

pub(super) fn render(page: ValuePage, browser: State<Browser>) -> Element {
    let position = page.position;
    let mut body = rect()
        .width(Size::fill())
        .spacing(t::SPACE_MD)
        .child(navigate(
            "Return to trace record",
            browser,
            Place::Record {
                index: position.record,
                page: position.field / FIELDS,
            },
        ))
        .child(
            label()
                .text(format!(
                    "Trace record {} · {}",
                    position.record + 1,
                    page.summary
                ))
                .font_size(t::heading()),
        )
        .child(label().text(page.value.name.into_owned()))
        .child(label().text(format!(
            "Trace value part {} of {}",
            page.value.part + 1,
            page.value.parts
        )))
        .child(
            label()
                .text(page.value.value)
                .font_size(t::body())
                .meta_font(),
        );
    if page.value.parts > 1 {
        if position.part > 0 {
            body = body.child(navigate(
                "Previous trace value part",
                browser,
                Place::Value(ValuePosition {
                    part: position.part - 1,
                    ..position
                }),
            ));
        }
        if position.part + 1 < page.value.parts {
            body = body.child(navigate(
                "More trace value",
                browser,
                Place::Value(ValuePosition {
                    part: position.part + 1,
                    ..position
                }),
            ));
        }
    }
    body.into_element()
}
