use super::*;
use recite_compiler::authoring::WorkControl;
use recite_writer_session::localisation::{catalogue::RefreshPreview, extraction};
use slot::Owner;

impl Plan {
    pub(super) fn prepare(
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
            Target::Passage(anchor) => {
                let selected = model.selected_block().map_err(|error| error.to_string())?;
                let passage = passages
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .find(|passage| {
                        anchor.as_ref().map_or_else(
                            || selected.as_ref() == Some(&passage.section),
                            |anchor| &passage.id == anchor,
                        )
                    })
                    .ok_or("Choose a source passage before opening its localisation context.")?;
                let entry = match owner {
                    Some(owner) => {
                        let id = owner
                            .catalogue
                            .document
                            .entries()
                            .iter()
                            .find(|entry| {
                                entry.context().is_some_and(|context| {
                                    context.split('&').next() == Some(passage.id.as_str())
                                })
                            })
                            .map(|entry| entry.id());
                        presentation::describe(owner, &self.query, id, passages.clone())?.entry
                    }
                    None => None,
                };
                Ok(Projection::Passage {
                    context: PassageContext {
                        pages: text::reading_pages(&passage.text),
                        page: 0,
                    },
                    entry,
                })
            }
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
