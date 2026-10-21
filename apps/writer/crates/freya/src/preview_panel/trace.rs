//! Both frontends inspect one bounded, shared trace projection.
use crate::design::{Button, tokens as t};
use freya::prelude::*;
use recite_runtime::preview::PreviewTrace;
use recite_writer_session::preview::trace::{
    Browser, ListPage, Place, Projection, Section, ValuePosition,
};
pub(super) use recite_writer_session::preview::trace::{condition as query, effect};
mod record;
mod value;

pub(super) fn render(trace: &PreviewTrace, mut browser: State<Browser>) -> Element {
    let current = *browser.read();
    let mut body = rect().width(Size::fill()).spacing(t::SPACE_MD).font_size(t::body())
        .child(label().text("Runtime trace").font_size(t::heading()))
        .child(label().text("These records belong to the running preview. Events retain capture order; translation and plural lookups have their own ordered lists."));
    for section in [Section::Events, Section::Localisation, Section::Plurals] {
        body = body.child(
            Button::new()
                .flat()
                .toggle(section == current.section)
                .on_press(move |_| browser.write().select(section))
                .child(section.caption()),
        );
    }
    body.child(match current.project(trace) {
        Projection::List(page) => records(page, browser),
        Projection::Record(page) => record::render(page, browser),
        Projection::Value(page) => value::render(page, browser),
    })
    .into_element()
}
fn records(page: ListPage, browser: State<Browser>) -> Element {
    let mut body = rect()
        .width(Size::fill())
        .spacing(t::SPACE_SM)
        .child(label().text(format!(
            "{} · {} records · page {} of {}",
            page.section.caption(),
            page.total,
            page.page + 1,
            page.pages
        )));
    for row in page.rows {
        body = body.child(navigate(
            format!("Open trace record {} · {}", row.index + 1, row.summary),
            browser,
            Place::Record {
                index: row.index,
                page: 0,
            },
        ));
    }
    if page.total == 0 {
        body = body.child(label().text("No records in this preview."));
    }
    if page.page > 0 {
        body = body.child(navigate(
            "Previous trace records",
            browser,
            Place::List {
                page: page.page - 1,
            },
        ));
    }
    if page.page + 1 < page.pages {
        body = body.child(navigate(
            "More trace records",
            browser,
            Place::List {
                page: page.page + 1,
            },
        ));
    }
    body.into_element()
}
fn navigate(caption: impl Into<String>, mut browser: State<Browser>, place: Place) -> Element {
    Button::new()
        .flat()
        .on_press(move |_| browser.write().place = place)
        .child(caption.into())
        .into_element()
}

#[cfg(test)]
mod tests;
