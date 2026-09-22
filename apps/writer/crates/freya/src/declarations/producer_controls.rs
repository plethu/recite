use super::*;
use crate::design::SubmitAction;
#[derive(Clone, Copy)]
pub(super) struct ProducerActions {
    pub writer: Writer,
}
impl PartialEq for ProducerActions {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for ProducerActions {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let id = use_a11y();
        let files = writer.files.read();
        let Some(session) = files.as_ref().and_then(|f| f.declarations.as_ref()) else {
            return rect();
        };
        let mut body = rect().width(Size::fill()).spacing(t::SPACE_SM);
        let reload = Button::new()
            .flat()
            .on_press(move |_| {
                if let Some(s) = writer
                    .files
                    .write()
                    .as_mut()
                    .and_then(|f| f.declarations.as_mut())
                {
                    s.reload_registration();
                }
            })
            .child(text(MsgId::WriterReloadProducer));
        let registration = match &session.registration {
            Ok(Some(registration)) => registration,
            Ok(None)
                if session
                    .schema
                    .producer_metadata
                    .as_ref()
                    .and_then(|m| m.producer.as_ref())
                    .is_some_and(|p| p.kind() == "standalone") =>
            {
                return body.child(reload);
            }
            Ok(None) => {
                return body
                    .child(
                        paragraph()
                            .width(Size::fill())
                            .span(Span::new(text(MsgId::WriterProducerNotRegistered))),
                    )
                    .child(reload);
            }
            Err(error) => {
                return body
                    .child(
                        paragraph()
                            .width(Size::fill())
                            .span(Span::new(error.clone())),
                    )
                    .child(reload);
            }
        };
        if session.busy() {
            return body
                .child(
                    label()
                        .text(text(MsgId::WriterGenerating))
                        .a11y_role(AccessibilityRole::Status),
                )
                .child(
                    Button::new()
                        .on_press(move |_| {
                            if let Some(job) = writer
                                .files
                                .peek()
                                .as_ref()
                                .and_then(|f| f.declarations.as_ref())
                                .and_then(|s| s.job.as_ref())
                            {
                                job.cancel();
                            }
                        })
                        .child(text(MsgId::WriterCancel)),
                );
        }
        let command = registration.generate();
        body = body.child(
            paragraph().width(Size::fill()).span(
                Span::new(format!(
                    "{} {:?} · {}",
                    command.program(),
                    command.args(),
                    command.directory()
                ))
                .font_family("monospace"),
            ),
        );
        let submit = SubmitAction {
            id,
            caption: text(MsgId::WriterRegenerate),
            enabled: !session.dirty(),
            action: EventHandler::new(move |()| {
                if let Some(s) = writer
                    .files
                    .write()
                    .as_mut()
                    .and_then(|f| f.declarations.as_mut())
                    && let Err(e) = s.start_generation()
                {
                    writer.message.error(e);
                }
            }),
        };
        let keyboard = submit.clone();
        body.on_key_down(move |e: Event<KeyboardEventData>| {
            if crate::design::keyboard::submit_key(&e) {
                e.prevent_default();
                e.stop_propagation();
                keyboard.run();
            }
        })
        .child(
            rect()
                .horizontal()
                .spacing(t::SPACE_MD)
                .child(submit.button())
                .child(reload)
                .child(
                    Button::new()
                        .flat()
                        .on_press(move |_| {
                            let result = (|| {
                                let mut files = writer.files.write();
                                let project = files.as_mut().ok_or("Open a project first.")?;
                                project
                                    .declarations
                                    .as_mut()
                                    .ok_or("Open declarations first.")?
                                    .reload_generated()
                                    .map_err(|e| e.to_string())?;
                                if let Ok(model) = writer.buffers.model.write().as_mut() {
                                    project.refresh(model).map_err(|e| e.to_string())?;
                                }
                                Ok(())
                            })();
                            writer
                                .message
                                .report(result, text(MsgId::WriterProducerFinished));
                        })
                        .child(text(MsgId::WriterReloadGenerated)),
                ),
        )
    }
}
#[derive(Clone, Copy)]
pub(crate) struct ProducerPoll {
    pub writer: Writer,
}
impl PartialEq for ProducerPoll {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for ProducerPoll {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let mut tick = freya::sdk::use_timeout(|| std::time::Duration::from_millis(100));
        if tick.elapsed() {
            tick.reset();
            let result = writer
                .files
                .peek()
                .as_ref()
                .and_then(|p| p.declarations.as_ref())
                .and_then(|s| s.job.as_ref())
                .and_then(|job| job.poll());
            if let Some(result) = result {
                let mut files = writer.files.write();
                if let Some(project) = files.as_mut() {
                    let result = project
                        .declarations
                        .as_mut()
                        .ok_or("Declarations were closed.".to_owned())
                        .and_then(|s| s.finish_generation(result));
                    let result = result.and_then(|()| {
                        if let Ok(model) = writer.buffers.model.write().as_mut() {
                            project.refresh(model).map_err(|e| e.to_string())?;
                        }
                        Ok(())
                    });
                    writer
                        .message
                        .report(result, text(MsgId::WriterProducerFinished));
                }
            }
        }
        rect()
    }
}
