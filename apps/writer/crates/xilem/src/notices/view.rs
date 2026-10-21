use super::{Evidence, Target, Writer};
use crate::{
    Phase, navigation,
    presentation::{label, paragraph, text_button},
};
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::layout::AsUnit,
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row},
};

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
                .map(|warning| paragraph(writer.ui_scale(), warning.clone(), 640.)),
            notice(writer),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
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
    let blocked = writer.phase != Phase::Writing || writer.project_action_blocked();
    let current = match evidence {
        Evidence::Source(observed) => writer
            .session
            .project()
            .is_some_and(|project| observed.is_current(project.files())),
        Evidence::Catalogue(observed) => observed.is_current(&writer.localisation),
    };
    Some(
        flex_col((
            label(
                scale,
                format!(
                    "Changed on disk · {} {}",
                    writer.notices.found.len(),
                    if writer.notices.found.len() == 1 {
                        "file"
                    } else {
                        "files"
                    }
                ),
            ),
            paragraph(scale, path.display().to_string(), 640.),
            actions(writer, blocked || !current),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
fn actions(writer: &Writer, disabled: bool) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let actions = (
        text_button(
            scale,
            "Compare changed file",
            Writer::compare_noticed_change,
        )
        .disabled(disabled),
        text_button(scale, "Dismiss notice", |writer: &mut Writer| {
            if let Some((target, _)) = writer.notices.selected() {
                let target = target.clone();
                writer.notices.found.remove(&target);
            }
        }),
        (writer.notices.found.len() > 1).then(|| {
            text_button(scale, "Next changed file", |writer: &mut Writer| {
                writer.notices.next()
            })
        }),
    );
    if writer.viewport.width < 480. * f64::from(scale) + 48. {
        Either::A(
            flex_col(actions)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(8.px()),
        )
    } else {
        Either::B(flex_row(actions).gap(8.px()))
    }
}
impl Writer {
    pub(super) fn compare_noticed_change(&mut self) {
        if self.phase != Phase::Writing || self.project_action_blocked() {
            return;
        }
        match self.notices.selected() {
            Some((Target::Source(path), Evidence::Source(observation)))
                if self
                    .session
                    .project()
                    .is_some_and(|project| observation.is_current(project.files())) =>
            {
                let path = path.clone();
                if self
                    .session
                    .project()
                    .is_some_and(|project| project.current().path() == path)
                {
                    self.compare_disk_changes();
                } else if let Err(error) =
                    self.begin_navigation(navigation::Target::CompareFile(path))
                {
                    self.status = error;
                }
            }
            Some((_, Evidence::Catalogue(observation)))
                if observation.is_current(&self.localisation) =>
            {
                let scope = observation.scope().clone();
                self.compare_noticed_catalogue(&scope);
            }
            _ => (),
        }
    }
}
