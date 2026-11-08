//! Frontend views and native glue over `recite_writer_app::workspaces`.
use crate::{Phase, Writer};
pub(crate) use recite_writer_app::workspaces::*;
mod builds;
mod declarations;
mod external;
mod rename;
mod settings;
mod tools;
pub(crate) use rename::recovery;
pub(crate) use tools::worker;
use xilem::{
    ViewCtx,
    core::{NoElement, ViewSequence},
};

pub(crate) fn panel(writer: &Writer) -> impl xilem::WidgetView<Writer> + use<> {
    xilem::view::indexed_stack((
        declarations::panel(writer),
        builds::panel(writer),
        settings::panel(writer),
        rename::panel(writer),
        external::panel(writer),
        crate::localisation::panel(writer),
    ))
    .active(match writer.workspaces.screen {
        Screen::Build => 1,
        Screen::Settings => 2,
        Screen::Rename => 3,
        Screen::External => 4,
        Screen::Localisation => 5,
        Screen::Write | Screen::Declarations => 0,
    })
}

// Conditional task lifetime follows active jobs, independently of the selected screen.
pub(crate) fn jobs(writer: &Writer) -> impl ViewSequence<Writer, (), ViewCtx, NoElement> + use<> {
    writer.project_jobs_busy().then(|| {
        xilem::view::task(
            |proxy, _: &mut Writer| async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    if proxy.message(()).is_err() {
                        break;
                    }
                }
            },
            |writer: &mut Writer, ()| writer.poll_project_jobs(),
        )
    })
}

#[cfg(test)]
pub(crate) mod tests;
