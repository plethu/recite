//! Live work status shares the workbench's accepted diagnostic counts.
use crate::design::{material, tokens as t};
use freya::prelude::*;

pub(super) fn render(
    id: AccessibilityId,
    saved: String,
    problems: String,
    preview_status: &str,
    diagnostic_count: usize,
) -> Element {
    let status_text = format!("{saved} · {problems}{preview_status}");
    let p = t::colors();
    rect()
        .horizontal()
        .width(Size::fill())
        .cross_align(Alignment::Center)
        .padding(Gaps::new(6., 16., 6., 16.))
        .spacing(t::SPACE_SM)
        .background(p.status_bar)
        .border(
            Border::new()
                .width(BorderWidth {
                    top: 1.,
                    ..Default::default()
                })
                .fill(p.status_edge),
        )
        .a11y_id(id)
        .a11y_role(AccessibilityRole::Status)
        .a11y_alt(status_text)
        .a11y_builder(|node| node.set_live(accesskit::Live::Polite))
        .child(material::registration(p.status_text, 12.))
        .child(t::meta(saved).color(p.status_text))
        .child(t::meta("·").color(p.status_text))
        .child(t::meta(problems).color(if diagnostic_count == 0 {
            p.status_signal
        } else {
            p.status_text
        }))
        .maybe(!preview_status.is_empty(), |bar| {
            bar.child(t::meta("·").color(p.status_text))
                .child(t::meta(preview_status).color(p.status_text))
        })
        .into_element()
}
