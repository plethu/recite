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
        location.page = self.query.page;
        if !active {
            return;
        }
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
                location.page = self
                    .refresh
                    .as_ref()
                    .and_then(|refresh| refresh.pages.get(refresh.page))
                    .map_or(0, |page| page.change);
                LinkedScreen::Updates
            }
            Destination::Create | Destination::Passage => LinkedScreen::Localise,
        };
    }
}
