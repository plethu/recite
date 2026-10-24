//! Reveal a changed search selection without overriding subsequent manual scrolling.
use freya::prelude::*;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq)]
struct Geometry {
    row_height: f32,
    leading_rows: usize,
}

#[derive(PartialEq)]
struct RevealRequest {
    query: Option<String>,
    active: Option<usize>,
    geometry: Geometry,
}

pub(crate) fn use_list_reveal(
    query: Option<State<String>>,
    active: State<Option<usize>>,
    mut scroll: ScrollController,
    row_height: f32,
    leading_rows: usize,
) {
    let geometry = use_reactive(&Geometry {
        row_height,
        leading_rows,
    });
    let mut previous = use_state(|| None::<RevealRequest>);
    use_after_side_effect(move || {
        let geometry = *geometry.read();
        let request = RevealRequest {
            query: query.map(|query| query.read().clone()),
            active: *active.read(),
            geometry,
        };
        if previous.peek().as_ref() == Some(&request) {
            return;
        }
        let index = request.active.unwrap_or(0);
        previous.set(Some(request));
        // Freya reads reactive bounds while clamping. A bounds update must not
        // repeat this request after the user has scrolled elsewhere.
        scroll.scroll_to_y(
            -(index.saturating_sub(geometry.leading_rows) as f32 * geometry.row_height) as i32,
        );
    });
}
