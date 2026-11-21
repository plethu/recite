use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::i18n::{Messages, MsgId};

use super::super::state::TuiState;
use super::controls::controls_for_prompt;

const GAP: usize = 3;
/// Choice IDs are letters, digits and underscores; numbers pick by position.
const TYPED_SELECTION_KEYS: &str = "a-z 0-9";

/// A two-column key table: each key, then what it does, wrapped under its own column so a long
/// key never pushes the description out of line.
pub(super) fn help_overlay_lines(
    state: &TuiState,
    messages: &Messages,
    width: u16,
) -> Vec<Line<'static>> {
    let mut rows = Vec::new();
    if !state.deferred_queue.is_empty() {
        rows.push((
            "Ctrl-D".to_owned(),
            messages.text(MsgId::TuiHelpDescriptionQueue),
        ));
    }
    for control in controls_for_prompt(&state.prompt, state.keymap) {
        // The footer's phrase "an ID or number" names what to type rather than a key; the key
        // column shows the keys themselves.
        let key = if control.action == MsgId::TuiHelpActionInput && control.keys.contains(' ') {
            TYPED_SELECTION_KEYS.to_owned()
        } else {
            control.keys.to_owned()
        };
        rows.push((key, messages.text(control.description)));
    }
    let heading = (
        messages.text(MsgId::TuiHelpKeyHeading),
        messages.text(MsgId::TuiHelpDescriptionHeading),
    );
    let width = usize::from(width).max(20);
    let key_width = rows
        .iter()
        .chain(std::iter::once(&heading))
        .map(|(key, _)| key.width())
        .max()
        .unwrap_or(0)
        .min(width / 3);
    let measure = width.saturating_sub(key_width + GAP).max(12);

    let mut lines = vec![
        Line::from(Span::styled(
            messages.text(MsgId::TuiHelpTitle),
            state.palette.title(),
        )),
        Line::from(""),
    ];
    let heading_style = state.palette.muted().add_modifier(Modifier::BOLD);
    lines.extend(table_row(
        &heading.0,
        &heading.1,
        key_width,
        measure,
        heading_style,
        heading_style,
    ));
    for (key, description) in &rows {
        lines.extend(table_row(
            key,
            description,
            key_width,
            measure,
            state.palette.title(),
            state.palette.plain(),
        ));
    }
    lines
}

fn table_row(
    key: &str,
    description: &str,
    key_width: usize,
    measure: usize,
    key_style: Style,
    description_style: Style,
) -> Vec<Line<'static>> {
    let keys = textwrap::wrap(key, key_width.max(1));
    let descriptions = textwrap::wrap(description, measure);
    let height = keys.len().max(descriptions.len());
    (0..height)
        .map(|index| {
            let key = keys.get(index).map_or("", |key| key.as_ref());
            let description = descriptions
                .get(index)
                .map_or("", |description| description.as_ref());
            let pad = " ".repeat(key_width.saturating_sub(key.width()) + GAP);
            Line::from(vec![
                Span::styled(key.to_owned(), key_style),
                Span::raw(pad),
                Span::styled(description.to_owned(), description_style),
            ])
        })
        .collect()
}
