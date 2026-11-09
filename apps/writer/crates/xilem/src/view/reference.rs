//! Both reading aids reuse one bounded read-only block without mounting more editors.
use super::*;
use crate::WriterLayout;
use crate::presentation::{group::named, manuscript_text, print};
use recite_writer_model::{PassageKind, ScriptEntry};
use recite_writer_session::{choices::display_name, reading::Excerpt};
use xilem::{
    AnyWidgetView,
    masonry::properties::LineBreaking,
    style::Padding,
    view::{GridExt, GridTrackSize, MainAxisAlignment, grid, portal},
};

pub(super) fn pinned(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let pinned = state.reading_context.pinned.as_ref()?;
    if state.workspaces.screen != crate::workspaces::Screen::Write
        || state.visible_workspace() == crate::map::Workspace::Map
    {
        return None;
    }
    let scale = state.ui_scale();
    let available = (state.viewport.width - state.navigation_width() - 32.).max(280.);
    let inner = available - 24.;
    let action = 120. * f64::from(scale);
    let caption = (inner - 2. * action - 16.).max(80.);
    let width = state.script_width().min(inner - 24.);
    let height = if pinned.collapsed {
        28. * f64::from(scale) + 12.
    } else {
        (state.viewport.height * 0.24).clamp(100., 180.)
    };
    Some(
        flex_row((sized_box(named(
            "Reference pane · read only",
            flex_col((
                grid((
                    print::action(
                        scale,
                        if pinned.collapsed {
                            "Show reference"
                        } else {
                            "Hide reference"
                        },
                        state.phase() != Phase::Writing,
                        |state: &mut Writer| {
                            if let Some(pinned) = &mut state.reading_context.pinned {
                                pinned.collapsed = !pinned.collapsed;
                            }
                        },
                    )
                    .padding(Padding::from_vh(
                        (2. * f64::from(scale)).px(),
                        (8. * f64::from(scale)).px(),
                    ))
                    .grid_pos(0, 0),
                    print::meta(
                        scale,
                        format!(
                            "Pinned snapshot · {} · {}",
                            display_name(&pinned.snapshot.block().id),
                            pinned.snapshot.document(),
                        ),
                    )
                    .text_size(11. * scale)
                    .prop(LineBreaking::WordWrap)
                    .color(state.palette.muted)
                    .grid_pos(1, 0),
                    print::action(
                        scale,
                        "Unpin reference",
                        state.phase() != Phase::Writing,
                        |state: &mut Writer| state.reading_context.pinned = None,
                    )
                    .padding(Padding::from_vh(
                        (2. * f64::from(scale)).px(),
                        (8. * f64::from(scale)).px(),
                    ))
                    .grid_pos(2, 0),
                ))
                .columns([action, caption, action].map(|width| GridTrackSize::Fixed(width.px())))
                .rows([GridTrackSize::Auto])
                .gap(8.px()),
                (!pinned.collapsed).then(|| {
                    portal(entries(state, &pinned.snapshot.block().entries, width))
                        .constrain_horizontal(true)
                        .flex(1.)
                }),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(8.px()),
        ))
        .dims((available.px(), height.px()))
        .padding(Padding::from_vh(6.px(), 12.px())),))
        .main_axis_alignment(MainAxisAlignment::Center)
        .background_color(state.palette.inset),
    )
}

pub(crate) fn destination(
    state: &Writer,
    target: &str,
    width: f64,
) -> Option<Box<AnyWidgetView<Writer>>> {
    if !state.destination_expanded(target) {
        return None;
    }
    let block = state
        .reading
        .script()
        .ok()?
        .iter()
        .find(|block| block.id == target);
    let Some(block) = block else {
        return Some(
            paragraph(
                state.ui_scale(),
                format!("Destination {target} is unavailable. Repair the source in your editor."),
                width,
            )
            .boxed(),
        );
    };
    let target = target.to_owned();
    Some(
        sized_box(
            flex_col((
                print::meta(
                    state.ui_scale(),
                    format!(
                        "Destination preview · read only · {}",
                        display_name(&block.id)
                    ),
                )
                .color(state.palette.muted),
                entries(state, &block.entries, width - 24.),
                print::action(
                    state.ui_scale(),
                    format!("Edit {}", display_name(&block.id)),
                    state.phase() != Phase::Writing || state.navigation.busy() || state.work.busy(),
                    move |state: &mut Writer| state.inspect(target.clone()),
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        )
        .width(width.px())
        .padding(12.px())
        .background_color(state.palette.inset)
        .boxed(),
    )
}

fn entries(state: &Writer, items: &[ScriptEntry], width: f64) -> impl WidgetView<Writer> + use<> {
    let mut excerpt = Excerpt::new(items);
    let rows = excerpt
        .by_ref()
        .map(|row| {
            let indent = row.depth.min(4) as f64 * 12. * f64::from(state.ui_scale());
            flex_row((
                sized_box(flex_col(())).width(indent.px()),
                entry_view(state, row.entry, (width - indent).max(80.)),
            ))
            .gap(0.px())
        })
        .collect::<Vec<_>>();
    flex_col((
        rows,
        excerpt.limited().then(|| {
            paragraph(
                state.ui_scale(),
                "Preview limited to 32 entries. Open the beat to read the rest.",
                width,
            )
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn entry_view(state: &Writer, entry: &ScriptEntry, width: f64) -> Box<AnyWidgetView<Writer>> {
    let scale = state.ui_scale();
    match entry {
        ScriptEntry::Passage(passage) => {
            let caption_width = (120. * f64::from(scale)).min(width * 0.3);
            let caption = match &passage.kind {
                PassageKind::Dialogue { speaker } => speaker
                    .as_deref()
                    .map(display_name)
                    .unwrap_or_else(|| "Dialogue".into()),
                PassageKind::Choice { destination } => destination
                    .as_deref()
                    .map(|target| format!("→ {}", display_name(target)))
                    .unwrap_or_else(|| "Reply".into()),
            };
            flex_row((
                print::meta(scale, caption)
                    .text_size(11. * scale)
                    .prop(LineBreaking::WordWrap)
                    .width(caption_width.px())
                    .color(state.palette.muted),
                manuscript_text(0.75 * state.reading_size(), passage.text.clone())
                    .width((width - caption_width - 8.).px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(4.px())
            .boxed()
        }
        ScriptEntry::Group { heading, .. } => label(scale, heading.clone()).boxed(),
        ScriptEntry::Jump(target) => label(scale, format!("→ {}", display_name(target))).boxed(),
        ScriptEntry::Effect(text) => paragraph(scale, format!("Effect · {text}"), width).boxed(),
        ScriptEntry::Source(text) => {
            paragraph(scale, format!("Source fragment · {text}"), width).boxed()
        }
    }
}
