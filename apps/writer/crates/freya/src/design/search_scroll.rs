//! Reveal a changed search selection without overriding subsequent manual scrolling.
use freya::prelude::*;

pub(crate) fn use_list_reveal(
    query: Option<State<String>>,
    active: State<Option<usize>>,
    mut scroll: ScrollController,
    row_height: f32,
    leading_rows: usize,
) {
    let mut previous = use_state(|| None::<(Option<String>, Option<usize>)>);
    use_after_side_effect(move || {
        let request = (query.map(|query| query.read().clone()), *active.read());
        if previous.peek().as_ref() == Some(&request) {
            return;
        }
        let index = request.1.unwrap_or(0);
        previous.set(Some(request));
        // Freya reads reactive bounds while clamping. A bounds update must not
        // repeat this request after the user has scrolled elsewhere.
        scroll.scroll_to_y(-(index.saturating_sub(leading_rows) as f32 * row_height) as i32);
    });
}
