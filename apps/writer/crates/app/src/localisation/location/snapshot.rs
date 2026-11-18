use super::*;

impl Localisation {
    pub(crate) fn describe_location(
        &self,
        root: Option<&Path>,
        active: bool,
        location: &mut Location,
    ) {
        location.catalogue = self.slot.path().map(|path| {
            root.and_then(|root| path.strip_prefix(root).ok())
                .unwrap_or(path)
                .to_string_lossy()
                .into_owned()
        });
        location.entry = None;
        location.query = self.query.text.clone();
        location.attention = self.query.filter == Filter::Attention;
        if !active {
            return;
        }
        location.source = false;
        location.view = Some(recite_config::WriterView::Script);
        location.screen = match self.destination {
            Destination::Queue => LinkedScreen::Translations,
            Destination::Entry => {
                location.entry = self.entry.as_ref().and_then(|entry| {
                    self.slot
                        .document()?
                        .entry(entry.key.id)?
                        .context()
                        .map(str::to_owned)
                });
                LinkedScreen::Entry
            }
            Destination::Comparison => LinkedScreen::Compare,
            Destination::Refresh => {
                // A link to the updates names the first row whose diff is open, counting from
                // one; zero opens none.
                location.page = self
                    .refresh
                    .as_ref()
                    .and_then(|refresh| refresh.open.first())
                    .map_or(0, |row| row + 1);
                LinkedScreen::Updates
            }
            Destination::Create => LinkedScreen::Localise,
            Destination::Passage => {
                if let Some(beat) = &self.passage {
                    location.beat = Some(beat.name.clone());
                    location.page = beat.page;
                    location.passage = None;
                }
                LinkedScreen::Localise
            }
        };
    }
}
