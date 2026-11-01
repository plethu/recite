//! Project failure recovery keeps technical diagnostics behind explicit disclosure.
use super::App;
use crate::presentation::{group, label, paragraph, text_button};
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{CrossAxisAlignment, flex_col},
};

pub(in crate::app) enum ProjectAction {
    Open,
    Create,
}
pub(in crate::app) struct Problem {
    action: ProjectAction,
    diagnostic: String,
    expanded: bool,
}
impl Problem {
    fn heading(&self) -> &'static str {
        match self.action {
            ProjectAction::Open => "Project didn’t open",
            ProjectAction::Create => "Project wasn’t created",
        }
    }
    fn guidance(&self) -> &'static str {
        match self.action {
            ProjectAction::Open => {
                "Choose the folder containing recite.project.toml, or create a new project."
            }
            ProjectAction::Create => {
                "Choose a writable folder and a project name. Existing project files are preserved."
            }
        }
    }
}
impl App {
    pub(in crate::app) fn clear_project_problem(&mut self) {
        if self.welcome_problem.take().is_some() {
            self.status.clear();
        }
    }
    pub(in crate::app) fn project_failed(&mut self, action: ProjectAction, diagnostic: String) {
        self.status = diagnostic.clone();
        self.welcome_problem = Some(Problem {
            action,
            diagnostic,
            expanded: false,
        });
    }
}

pub(super) fn panel(app: &App, scale: f32, width: f64) -> Option<impl WidgetView<App> + use<>> {
    let problem = app.welcome_problem.as_ref()?;
    Some(group::status(
        problem.heading(),
        flex_col((
            label(scale, problem.heading())
                .text_size(16. * scale)
                .color(app.theme.palette.accent),
            paragraph(scale, problem.guidance(), width),
            text_button(
                scale,
                if problem.expanded {
                    "Hide project error details"
                } else {
                    "Show project error details"
                },
                |app: &mut App| {
                    if let Some(problem) = &mut app.welcome_problem {
                        problem.expanded = !problem.expanded;
                    }
                },
            ),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    ))
}

pub(super) fn details(app: &App, scale: f32, width: f64) -> Option<impl WidgetView<App> + use<>> {
    let problem = app
        .welcome_problem
        .as_ref()
        .filter(|problem| problem.expanded)?;
    Some(group::named(
        "Project error details",
        flex_col((
            label(scale, "Error details").color(app.theme.palette.muted),
            paragraph(scale, problem.diagnostic.clone(), width),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    ))
}
