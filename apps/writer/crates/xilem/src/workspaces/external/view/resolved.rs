//! A completed decision replaces the review rather than leaving disabled alternatives behind.
use super::*;
use crate::attention::PaneEntry;
use xilem::{masonry::properties::LineBreaking, view::FlexExt};

pub(super) fn panel(writer: &Writer, resolved: &Resolved) -> impl WidgetView<Writer> + use<> {
    let (heading, explanation) = match resolved.resolution {
        Resolution::KeepWriter => (
            "Writer version kept",
            "Your accepted document, field draft and Undo history are retained. This decision did not write the source file. Save writes your accepted result over the inspected disk version.",
        ),
        Resolution::UseInspectedDisk => (
            "Inspected disk version loaded",
            "The inspected disk version is now the accepted document. The previous session is archived, and Undo restores the previous accepted source. This decision did not write the source file.",
        ),
        Resolution::Merged => (
            "Reviewed merge adopted",
            "The validated merge is now the accepted document as one Undo step. This decision did not write the source file. Save writes the accepted merge over the inspected disk version.",
        ),
    };
    let scale = writer.ui_scale();
    let path = writer
        .session
        .project()
        .and_then(|project| resolved.path.strip_prefix(project.root()).ok())
        .unwrap_or(&resolved.path);
    flex_col((
        sized_box(
            flex_row((
                label(scale, heading)
                    .text_size(24. * scale)
                    .prop(LineBreaking::WordWrap)
                    .flex(1.),
                text_button(scale, "Return to writing", Writer::return_to_writing),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(12.px())
            .prop(PaneEntry(true)),
        )
        .width(writer.content_width().px()),
        paragraph(scale, path.display().to_string(), writer.content_width()),
        report(writer),
        paragraph(scale, explanation, writer.content_width().min(640.)),
        paragraph(
            scale,
            "A complete recovery copy was kept; its location is available above.",
            writer.content_width().min(640.),
        ),
        text_button(
            scale,
            "Inspect current disk version",
            Writer::compare_disk_changes,
        )
        .disabled(writer.project_action_blocked()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
    .prop(Region(Some(Pane::External)))
}
