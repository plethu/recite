use super::*;
use recite_config::ProducerRegistration;
use std::fmt::Write;
#[derive(Default)]
pub(super) struct Panel {
    pub pages: Vec<String>,
    pub page: usize,
}
pub(in crate::workspaces) fn description(
    registration: &Result<Option<ProducerRegistration>, String>,
) -> Vec<String> {
    let text = match registration {
        Ok(Some(registration)) => {
            let command = registration.generate();
            let mut text = format!(
                "Executable: {:?}\nWorking directory: {:?}\nArguments (passed separately):",
                command.program(),
                command.directory()
            );
            for (index, argument) in command.args().iter().enumerate() {
                let _ = write!(text, "\n{}: {argument:?}", index + 1);
            }
            text
        }
        Err(error) => error.clone(),
        Ok(None) => {
            "No producer is registered. Generated declarations remain available for inspection."
                .into()
        }
    };
    super::super::text::pages(&text)
}
pub(super) fn controls(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let session = writer.declaration_session();
    let registration = session.and_then(|session| session.registration.as_ref().ok()?.as_ref());
    let panel = &writer.workspaces.declarations.producer;
    flex_col((
        label(scale, "Schema producer").text_size(18. * scale),
        label(
            scale,
            format!(
                "Producer command · part {} of {}",
                panel.page + 1,
                panel.pages.len()
            ),
        ),
        paragraph(
            scale,
            panel.pages.get(panel.page).cloned().unwrap_or_default(),
            640.,
        ),
        flex_row((
            text_button(scale, "Previous command detail", |writer: &mut Writer| {
                writer.workspaces.declarations.producer.page = writer
                    .workspaces
                    .declarations
                    .producer
                    .page
                    .saturating_sub(1)
            })
            .disabled(panel.page == 0),
            text_button(scale, "More command detail", |writer: &mut Writer| {
                writer.workspaces.declarations.producer.page += 1
            })
            .disabled(panel.page + 1 >= panel.pages.len()),
        )),
        flex_row((
            text_button(scale, "Regenerate schema", Writer::regenerate_schema).disabled(
                writer.declaration_action_blocked()
                    || registration.is_none()
                    || session.is_some_and(|session| session.dirty()),
            ),
            writer.workspaces.schema_refresh.is_some().then(|| {
                text_button(
                    scale,
                    "Retry schema analysis",
                    Writer::refresh_schema_context,
                )
                .disabled(writer.workspaces.tools.busy() || writer.work.busy())
            }),
        )),
        super::outcome::panel(writer),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
}
