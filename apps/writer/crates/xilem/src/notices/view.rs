//! A disk change is a banner at the top of the sheet: "Changed on disk · Compare · Keep mine ·
//! Use disk version". The comparison page opens only from Compare.
use super::{Evidence, Target, Writer};
use crate::WriterLayout;
use crate::presentation::banner::banner;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::tokens::size;
use crate::presentation::{Type, print, space};
use crate::{Phase, presentation::paragraph};
use recite_writer_session::project::ExternalDecision;
use xilem::core::one_of::Either;
use xilem::{
    WidgetView,
    masonry::{layout::AsUnit, parley::style::FontWeight},
    style::Style,
    view::{CrossAxisAlignment, button, flex_col, flex_row},
};

/// A banner action: its visible word, and the full name assistive technology reads.
fn action<F: Fn(&mut Writer) + Send + Sync + 'static>(
    scale: f32,
    (text, name): (&'static str, &'static str),
    disabled: bool,
    callback: F,
) -> impl WidgetView<Writer> + use<F> {
    styled(
        ControlStyle::Tonal,
        button(Type::Control.label(scale, text), callback)
            .accessible_label(name)
            .disabled(disabled),
    )
}

pub(crate) fn panel(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    if writer.notices.found.is_empty() && writer.notices.warning.is_none() {
        return None;
    }
    Some(
        flex_col((
            writer
                .notices
                .warning
                .as_ref()
                .map(|warning| paragraph(writer.ui_scale(), warning.clone(), size::MEASURE)),
            notice(writer),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
}
fn notice(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let (target, evidence) = writer.notices.selected()?;
    let path = match target {
        Target::Source(path) | Target::Catalogue(path) => path,
    };
    let path = writer
        .session
        .project()
        .and_then(|project| path.strip_prefix(project.root()).ok())
        .unwrap_or(path);
    let scale = writer.ui_scale();
    let blocked = writer.phase() != Phase::Writing || writer.project_action_blocked();
    let current = match evidence {
        Evidence::Source(observed) => writer
            .session
            .project()
            .is_some_and(|project| observed.is_current(project.files())),
        Evidence::Catalogue(observed) => observed.is_current(&writer.localisation),
    };
    let count = writer.notices.found.len();
    let width = writer.page_width();
    // The message and the five actions share one line from about 900px of content at 100%.
    let stacked = width < 880. * f64::from(scale);
    Some(banner(
        &writer.palette,
        (
            flex_row((
                Type::Control
                    .label(
                        scale,
                        format!(
                            "Changed on disk · {count} {}",
                            if count == 1 { "file" } else { "files" }
                        ),
                    )
                    .weight(FontWeight::SEMI_BOLD),
                print::id(scale, path.display().to_string()).color(writer.palette.muted),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(space::SIBLINGS.px()),
            actions(writer, blocked || !current, width < 560. * f64::from(scale)),
        ),
        stacked,
        width,
    ))
}
/// The banner's actions in a row, or one under another where even their own line is too narrow.
fn actions(writer: &Writer, disabled: bool, narrow: bool) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let settles = writer.noticed_change_settles();
    let actions = (
        action(
            scale,
            ("Compare", "Compare changed file"),
            disabled,
            Writer::compare_noticed_change,
        ),
        settles.then(|| {
            action(
                scale,
                ("Keep mine", "Keep mine"),
                disabled,
                |writer: &mut Writer| writer.settle_noticed_change(ExternalDecision::KeepWriter),
            )
        }),
        settles.then(|| {
            action(
                scale,
                ("Use disk version", "Use disk version"),
                disabled,
                |writer: &mut Writer| {
                    writer.settle_noticed_change(ExternalDecision::UseInspectedDisk)
                },
            )
        }),
        (writer.notices.found.len() > 1).then(|| {
            action(
                scale,
                ("Next file", "Next changed file"),
                false,
                |writer: &mut Writer| writer.notices.next(),
            )
        }),
        action(
            scale,
            ("Dismiss", "Dismiss notice"),
            false,
            |writer: &mut Writer| {
                if let Some((target, _)) = writer.notices.selected() {
                    let target = target.clone();
                    writer.notices.found.remove(&target);
                }
            },
        ),
    );
    if narrow {
        Either::A(
            flex_col(actions)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::SIBLINGS.px()),
        )
    } else {
        Either::B(flex_row(actions).gap(space::SIBLINGS.px()))
    }
}
