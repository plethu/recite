//! One root coordinator owns accepted editor work and ordered durability.
mod catalogue;
pub(crate) mod document;
mod manuscript;
pub(crate) mod worker;
pub(crate) use crate::project::save::ordered::Scope as SaveScope;
use crate::{editing::Writer, project::save::ordered::Scope};
use freya::prelude::*;
use recite_writer_model::{EditOperation, EditRequest, ReplyRulesRequest};
use worker::Worker;

enum Intent {
    Save {
        scope: Scope,
        source: Option<EditRequest>,
    },
    Edit {
        request: EditRequest,
        applying: bool,
    },
    Rules(ReplyRulesRequest),
    Catalogue(Box<crate::localisation::catalogue::CatalogueSave>),
    Navigation(Box<crate::navigation::request::Navigation>),
}
struct Pending {
    intent: Intent,
    worker: Worker,
    completed: EventHandler<Result<(), String>>,
}
#[derive(Clone, Copy)]
pub(crate) struct EditorWork {
    requests: State<Option<async_channel::Sender<Pending>>>,
    active: State<Option<Kind>>,
    pub(crate) focus: State<Option<crate::prose::focus::Request>>,
}
#[derive(Clone, Copy)]
enum Kind {
    Save,
    RulesSave,
    Source,
    History,
    Passage,
    RulesEdit,
    Rules,
    Catalogue,
    Navigation,
}
impl EditorWork {
    pub(crate) fn new() -> Self {
        Self {
            requests: use_state(|| None),
            active: use_state(|| None),
            focus: use_state(|| None),
        }
    }
    pub(crate) fn busy(self) -> bool {
        self.active.read().is_some()
    }
    pub(crate) fn caption(self) -> Option<String> {
        (*self.active.read()).map(|kind| match kind {
            Kind::Navigation => "Preparing navigation… You can keep editing.".into(),
            Kind::Save => crate::messages::text(crate::messages::MsgId::WriterSaving),
            Kind::RulesSave => "Saving reply rules…".into(),
            Kind::Source => "Preparing source edit… You can keep editing.".into(),
            Kind::Passage => "Preparing passage edit… You can keep editing.".into(),
            Kind::History => "Updating document history…".into(),
            Kind::RulesEdit => "Updating reply rules…".into(),
            Kind::Rules => "Preparing reply rules…".into(),
            Kind::Catalogue => "Saving translations… You can keep editing.".into(),
        })
    }
    pub(crate) fn require_idle(self) -> Result<(), String> {
        if self.active.peek().is_some() {
            Err("Editor work is still in progress.".into())
        } else {
            Ok(())
        }
    }
}
pub(crate) fn use_work(mut writer: Writer) {
    let waiting = use_hook(move || {
        let (requests, waiting) = async_channel::bounded(1);
        writer.work.requests.set(Some(requests));
        waiting
    });
    use_future(move || {
        let waiting = waiting.clone();
        async move {
            while let Ok(pending) = waiting.recv().await {
                finish(writer, pending).await;
            }
        }
    });
    use_drop(move || {
        writer.work.focus.set(None);
        writer.work.requests.set(None);
    });
}
fn enqueue(
    mut writer: Writer,
    intent: Intent,
    completed: EventHandler<Result<(), String>>,
) -> Result<(), String> {
    writer.work.require_idle()?;
    let worker = Worker::new().map_err(|e| e.to_string())?;
    let kind = match &intent {
        Intent::Save { .. } if *writer.pane.peek() == crate::editing::Pane::Rules => {
            Kind::RulesSave
        }
        Intent::Save { .. } => Kind::Save,
        Intent::Edit { .. } if *writer.pane.peek() == crate::editing::Pane::Rules => {
            Kind::RulesEdit
        }
        Intent::Edit { .. }
            if writer
                .buffers
                .model
                .peek()
                .as_ref()
                .is_ok_and(|model| model.view() == &recite_writer_model::View::Source) =>
        {
            Kind::Source
        }
        Intent::Edit { applying: true, .. } => Kind::Passage,
        Intent::Edit { .. } => Kind::History,
        Intent::Rules(_) => Kind::Rules,
        Intent::Catalogue(_) => Kind::Catalogue,
        Intent::Navigation(_) => Kind::Navigation,
    };
    let pending = Pending {
        intent,
        worker,
        completed,
    };
    let queued = writer
        .work
        .requests
        .peek()
        .as_ref()
        .is_some_and(|sender| sender.try_send(pending).is_ok());
    if !queued {
        return Err(
            "The editor coordinator is unavailable. Keep the editor open and retry.".into(),
        );
    }
    writer.work.active.set(Some(kind));
    if let Some(project) = writer.files.write().as_mut() {
        project.set_saving(true);
    }
    writer.message.clear();
    Ok(())
}
pub(crate) fn start(mut writer: Writer, scope: Scope, completed: EventHandler<Result<(), String>>) {
    let result = (|| {
        writer.work.require_idle()?;
        let applies = {
            let files = writer.files.peek();
            let project = files.as_ref().ok_or("Open a project before saving.")?;
            project.require_idle_save().map_err(|e| e.to_string())?;
            scope.applies_current(&project.current)
        };
        writer.buffers.harvest();
        let source = if applies {
            document::capture(writer, EditOperation::Apply)?
        } else {
            None
        };
        if applies && source.is_none() {
            writer
                .buffers
                .model
                .write()
                .as_mut()
                .map_err(|e| e.to_string())?
                .apply()
                .map_err(|e| e.to_string())?;
        }
        enqueue(writer, Intent::Save { scope, source }, completed.clone())
    })();
    if let Err(error) = result {
        completed.call(Err(error));
    }
}
pub(crate) fn edit(mut writer: Writer, operation: EditOperation) -> Result<(), String> {
    let completed = EventHandler::new(move |result: Result<(), String>| {
        writer.message.report(result, String::new())
    });
    edit_then(writer, operation, completed)
}
pub(crate) fn edit_then(
    writer: Writer,
    operation: EditOperation,
    completed: EventHandler<Result<(), String>>,
) -> Result<(), String> {
    writer.work.require_idle()?;
    writer.buffers.harvest();
    if let Some(request) = document::capture(writer, operation)? {
        enqueue(
            writer,
            Intent::Edit {
                request,
                applying: operation == EditOperation::Apply,
            },
            completed,
        )
    } else {
        let result = if operation == EditOperation::Apply {
            writer.try_perform(|m| m.apply())
        } else {
            Ok(())
        };
        completed.call(result);
        Ok(())
    }
}
pub(crate) fn rules(mut writer: Writer, passage: &str) -> Result<(), String> {
    writer.work.require_idle()?;
    let request = writer
        .buffers
        .model
        .peek()
        .as_ref()
        .map_err(|e| e.to_string())?
        .document()
        .reply_rules_request(passage);
    let completed = EventHandler::new(move |result: Result<(), String>| {
        if let Err(e) = result {
            writer.message.error(e);
        }
    });
    enqueue(writer, Intent::Rules(request), completed)
}
pub(crate) fn translations(
    mut writer: Writer,
    ids: Option<&[recite_core::po::PoEntryId]>,
    completed: EventHandler<Result<(), String>>,
) {
    if let Err(error) = writer.work.require_idle() {
        completed.call(Err(error));
        return;
    }
    let request = writer
        .localisation
        .write()
        .catalogue
        .as_mut()
        .ok_or_else(|| "The catalogue is no longer open.".to_owned())
        .and_then(|c| c.begin_save(ids));
    let result = match request {
        Ok(request) => enqueue(
            writer,
            Intent::Catalogue(Box::new(request)),
            completed.clone(),
        ),
        Err(error) => {
            completed.call(Err(error));
            return;
        }
    };
    if let Err(error) = result {
        if let Some(catalogue) = writer.localisation.write().catalogue.as_mut() {
            catalogue.finish_save();
        }
        completed.call(Err(error));
    }
}
async fn finish(mut writer: Writer, pending: Pending) {
    let Pending {
        intent,
        worker,
        completed,
    } = pending;
    let translation = matches!(intent, Intent::Catalogue(_));
    let result = match intent {
        Intent::Edit { request, .. } => document::accept(writer, request, &worker)
            .await
            .map_err(|e| e.to_string()),
        Intent::Rules(request) => document::rules(writer, request, &worker).await,
        Intent::Catalogue(request) => catalogue::save(writer, request, &worker).await,
        Intent::Navigation(request) => {
            crate::navigation::request::execute(writer, request, &worker).await
        }
        Intent::Save {
            scope,
            source: request,
        } => manuscript::save(writer, scope, request, &worker).await,
    };
    writer.buffers.harvest();
    crate::declarations::harvest(writer);
    writer.work.active.set(None);
    if let Some(project) = writer.files.write().as_mut() {
        project.set_saving(false);
    }
    if translation && let Some(catalogue) = writer.localisation.write().catalogue.as_mut() {
        catalogue.finish_save();
    }
    let successful = result.is_ok();
    completed.call(result);
    crate::prose::focus::complete(writer, successful);
}

pub(crate) fn save(writer: Writer, all: bool) {
    let mut message = writer.message;
    start(
        writer,
        if all { Scope::All } else { Scope::Current },
        EventHandler::new(move |result: Result<(), String>| {
            let changed = writer.buffers.model.peek().as_ref().is_ok_and(|m| {
                m.has_draft()
                    || writer
                        .files
                        .peek()
                        .as_ref()
                        .is_some_and(|p| p.dirty(m.document().source()))
            });
            let declarations_changed = all
                && writer
                    .files
                    .peek()
                    .as_ref()
                    .and_then(|p| p.declarations.as_ref())
                    .is_some_and(|s| s.dirty());
            if result.is_ok() && (changed || declarations_changed) {
                message.info(crate::messages::text(
                    crate::messages::MsgId::WriterNewerEditsRemain,
                ));
            } else {
                message.report(result, "Saved.".into());
            }
        }),
    );
}

pub(crate) fn navigate(
    writer: Writer,
    request: crate::navigation::request::Request,
    completed: EventHandler<Result<(), String>>,
) -> Result<(), String> {
    writer.work.require_idle()?;
    enqueue(
        writer,
        Intent::Navigation(Box::new(request.begin(writer)?)),
        completed,
    )
}

#[cfg(test)]
mod tests;
