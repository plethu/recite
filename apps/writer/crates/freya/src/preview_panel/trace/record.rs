use super::*;
use crate::design::tokens::PrintTypography;
use recite_writer_session::preview::trace::{RECORDS, RecordPage};

pub(super) fn render(page: RecordPage, browser: State<Browser>) -> Element {
    let mut body = rect()
        .width(Size::fill())
        .spacing(t::SPACE_MD)
        .child(navigate(
            "Return to trace records",
            browser,
            Place::List {
                page: page.index / RECORDS,
            },
        ))
        .child(
            label()
                .text(format!(
                    "Trace record {} · {}",
                    page.index + 1,
                    page.summary
                ))
                .font_size(t::heading()),
        );
    for field in page.fields {
        let mut row = rect()
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .child(label().text(field.name.clone()))
            .child(
                label()
                    .text(if field.truncated {
                        format!("{}…", field.value)
                    } else {
                        field.value
                    })
                    .font_size(t::body())
                    .meta_font(),
            );
        if field.truncated {
            row = row.child(navigate(
                format!("Read complete trace value · {}", field.name),
                browser,
                Place::Value(ValuePosition {
                    record: page.index,
                    field: field.index,
                    part: 0,
                }),
            ));
        }
        body = body.child(row);
    }
    if page.pages > 1 {
        body = body.child(label().text(format!(
            "Record details · page {} of {}",
            page.page + 1,
            page.pages
        )));
        if page.page > 0 {
            body = body.child(navigate(
                "Previous record details",
                browser,
                Place::Record {
                    index: page.index,
                    page: page.page - 1,
                },
            ));
        }
        if page.page + 1 < page.pages {
            body = body.child(navigate(
                "More record details",
                browser,
                Place::Record {
                    index: page.index,
                    page: page.page + 1,
                },
            ));
        }
    }
    body.into_element()
}
