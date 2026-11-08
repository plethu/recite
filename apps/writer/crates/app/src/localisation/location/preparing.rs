use super::*;
use recite_compiler::authoring::WorkControl;
use recite_writer_session::localisation::{catalogue::RefreshPreview, extraction};
use slot::Owner;

impl Plan {
    pub fn prepare(
        &self,
        checkout: &Checkout,
        project: &ProjectSession,
        control: &dyn WorkControl,
    ) -> Result<Ready, String> {
        control.checkpoint().map_err(|error| error.to_string())?;
        let installation = match &self.installation {
            Installation::Keep => Install::Keep,
            Installation::Close => Install::Replace(Slot::Closed),
            Installation::Open(path) => Install::Replace(Slot::Open(Box::new(Owner::new(
                Catalogue::open_recoverable(path)?,
            )))),
        };
        let owner = match &installation {
            Install::Keep => checkout.owner.as_deref(),
            Install::Replace(slot) => slot.owner(),
        };
        let projection = self.project(owner, project, control)?;
        control.checkpoint().map_err(|error| error.to_string())?;
        Ok(Ready {
            installation,
            projection,
            query: self.query.clone(),
        })
    }
    fn project(
        &self,
        owner: Option<&Owner>,
        project: &ProjectSession,
        control: &dyn WorkControl,
    ) -> Result<Projection, String> {
        let model = project.current().model();
        let passages = model.document().cached_passages();
        match &self.target {
            Target::Hidden => Ok(Projection::Hidden),
            Target::Queue => match owner {
                Some(owner) => presentation::describe(owner, &self.query, None, passages)
                    .map(Projection::Queue),
                None => Ok(Projection::Queue(Presentation {
                    queue: Queue::default(),
                    entry: None,
                    message: "Choose a catalogue to translate this project.",
                })),
            },
            Target::Entry(context) => {
                let owner = owner.ok_or("The linked translation needs a catalogue.")?;
                let id = owner
                    .catalogue
                    .document
                    .entries()
                    .iter()
                    .find(|entry| entry.context() == Some(context.as_str()))
                    .map(|entry| entry.id())
                    .ok_or("The linked translation entry is unavailable.")?;
                presentation::describe(owner, &self.query, Some(id), passages)
                    .map(Projection::Entry)
            }
            Target::Passage(selection) => beat::Beat::prepare(owner, model, selection)
                .map(|beat| Projection::Passage(Some(beat))),
            Target::Compare => {
                let compared = owner
                    .ok_or("The linked comparison needs a catalogue.")?
                    .catalogue
                    .compare()?;
                let changes = compared
                    .rows
                    .into_iter()
                    .flat_map(|mut row| {
                        row.caption = row.caption.chars().take(160).collect();
                        text::change_pages(row)
                    })
                    .collect();
                Ok(Projection::Compare(Some(Comparison {
                    inspection: compared.inspection,
                    changes,
                    page: 0,
                })))
            }
            Target::Updates(page) => {
                let catalogue = &owner
                    .ok_or("The linked source review needs a catalogue.")?
                    .catalogue;
                if catalogue.dirty() {
                    return Err(
                        "Save or discard translation drafts before reviewing source updates."
                            .into(),
                    );
                }
                let template = extraction::template(model, Some(project.files()), control)?;
                let mut refresh =
                    refresh::Refresh::new(RefreshPreview::prepare(catalogue, template)?);
                refresh.page = refresh
                    .pages
                    .iter()
                    .position(|value| value.change >= *page)
                    .unwrap_or_else(|| refresh.pages.len().saturating_sub(1));
                Ok(Projection::Updates(Some(refresh)))
            }
        }
    }
}
