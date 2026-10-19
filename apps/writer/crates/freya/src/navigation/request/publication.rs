//! Publish only after destination and outgoing recovery proofs remain current.
use super::{Origin, Request, Target};
use crate::navigation::worker::{self, Example, PreparedDestination};
use crate::{
    editing::Writer,
    editor_work::worker::{Output, Work, Worker},
};
pub(super) async fn publish(
    mut writer: Writer,
    request: &Request,
    origin: &Origin,
    prepared: &mut worker::Prepared,
    worker: &Worker,
) -> Result<(), String> {
    writer.buffers.harvest();
    request.require_current(origin, writer)?;
    let checkpoint = if matches!(prepared.destination, PreparedDestination::Switch(_)) {
        let checkpoint = {
            let model = writer.buffers.model.peek();
            let model = model.as_ref().map_err(|e| e.to_string())?;
            writer
                .files
                .write()
                .as_mut()
                .ok_or("Project is no longer open.")?
                .begin_navigation_checkpoint(model)
                .map_err(|e| e.to_string())?
        };
        let Output::Navigation(worker::Output::Checkpoint(result)) = worker
            .execute(Work::Navigation(Box::new(worker::Work::Checkpoint(
                Box::new(checkpoint),
            ))))
            .await
            .map_err(|e| e.to_string())?
        else {
            return Err("Invalid navigation checkpoint receipt.".into());
        };
        Some(result?)
    } else {
        None
    };
    writer.buffers.harvest();
    request.require_current(origin, writer)?;
    {
        let model = writer.buffers.model.peek();
        let model = model.as_ref().map_err(|e| e.to_string())?;
        let current = match &prepared.destination {
            PreparedDestination::Current(prepared) => prepared.is_current(model),
            PreparedDestination::Switch(prepared) => writer
                .files
                .peek()
                .as_ref()
                .zip(checkpoint.as_ref())
                .is_some_and(|(files, checkpoint)| prepared.is_current(files, model, checkpoint)),
            PreparedDestination::Example {
                name,
                prepared: Example::Retained(prepared),
            } => writer
                .examples
                .peek()
                .get(name)
                .is_some_and(|model| prepared.is_current(model)),
            PreparedDestination::Example {
                name,
                prepared: Example::Cold(next),
            } => next.is_some() && !writer.examples.peek().contains_key(name),
        };
        if !current {
            return Err(
                "The destination changed while navigation was preparing. Retry navigation.".into(),
            );
        }
    }
    let catalogue = if prepared.catalogue_changed {
        if writer.localisation.peek().dirty() {
            return Err(crate::localisation::close_drafts_message());
        }
        // Existing installation owns catalogue recovery and failure policy.
        let catalogue = prepared
            .catalogue
            .take()
            .map(|path| crate::localisation::catalogue::Catalogue::open_recoverable(&path))
            .transpose()?;
        let mut localisation = writer.localisation.write();
        localisation.prepare_install()?;
        Some(catalogue)
    } else {
        None
    };
    // Cancellation is checked once at the commit boundary, after all fallible I/O.
    request.require_not_cancelled()?;
    if let Some(catalogue) = catalogue {
        writer.localisation.write().install_prepared(catalogue);
    }
    let previous_scene = writer.scene_key();
    let previous_view = writer
        .buffers
        .model
        .peek()
        .as_ref()
        .map_err(|e| e.to_string())?
        .view()
        .clone();
    {
        let mut model = writer.buffers.model.write();
        let model = model.as_mut().map_err(|e| e.to_string())?;
        match &mut prepared.destination {
            PreparedDestination::Current(selection) => model
                .accept_navigation(selection)
                .map_err(|e| e.to_string())?,
            PreparedDestination::Switch(target) => writer
                .files
                .write()
                .as_mut()
                .ok_or("Project is no longer open.")?
                .accept_switch(
                    model,
                    target,
                    checkpoint.as_ref().ok_or("Missing outgoing checkpoint.")?,
                )
                .map_err(|e| e.to_string())?,
            PreparedDestination::Example { name, prepared } => {
                let mut examples = writer.examples.write();
                let next = match prepared {
                    Example::Retained(selection) => {
                        examples
                            .get_mut(name)
                            .ok_or("Example session disappeared.")?
                            .accept_navigation(selection)
                            .map_err(|e| e.to_string())?;
                        examples
                            .remove(name)
                            .ok_or("Example session disappeared.")?
                    }
                    Example::Cold(next) => {
                        *next.take().ok_or("Example session already accepted.")?
                    }
                };
                let old = std::mem::replace(model, next);
                examples.insert(old.document().key().to_string(), old);
            }
        }
    }
    let changed_scene = previous_scene != writer.scene_key();
    if changed_scene {
        writer.switch_rules(previous_scene);
    }
    if prepared.rules.is_some() {
        std::mem::swap(&mut *writer.buffers.rules.write(), &mut prepared.rules);
    }
    if let Some(rules) = writer.buffers.rules.write().as_mut()
        && let Ok(model) = writer.buffers.model.peek().as_ref()
    {
        match &prepared.destination {
            PreparedDestination::Current(prepared) => {
                prepared.refresh_rules(rules, model.document());
            }
            PreparedDestination::Switch(prepared) => {
                prepared.refresh_rules(rules, model);
            }
            PreparedDestination::Example {
                prepared: Example::Retained(prepared),
                ..
            } => {
                prepared.refresh_rules(rules, model.document());
            }
            PreparedDestination::Example { .. } => {}
        }
    }
    writer
        .buffers
        .sync_navigation(&previous_view, changed_scene, writer.dark);
    if let Some(project) = writer.files.write().as_mut() {
        project.set_saving(false);
    }
    let result = match &request.target {
        Target::Location(location) => crate::navigation::apply::present(writer, location),
        Target::Document { view, .. } => {
            writer.resume_prepared_scene(*view);
            Ok(())
        }
    };
    if let Some(project) = writer.files.write().as_mut() {
        project.set_saving(true);
    }
    result
}
