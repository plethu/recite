//! A line's speaker, chosen from the project's speakers on a slip anchored on its label.
use super::*;

pub struct SpeakerRequest {
    window: crate::WindowKey,
    origin: NavigationStamp,
    pub(crate) passage: String,
    /// The speaker the line has now.
    pub(crate) current: Option<String>,
    pub(crate) choices: Vec<PickerOption>,
}
impl SpeakerRequest {
    pub fn is_current(&self, writer: &Writer) -> bool {
        self.window == writer.window
            && self.origin.is_current(writer.session.model())
            && writer.phase == Phase::Writing
            && !writer.work.busy()
            && !writer.navigation.busy()
    }
    pub(crate) fn apply(self, writer: &mut Writer, speaker: String) {
        if !self.is_current(writer) || !self.choices.iter().any(|option| option.value == speaker) {
            writer.status = "This line changed. Reopen its speaker choice to continue.".into();
            return;
        }
        writer.navigate(crate::navigation::Target::Structure(
            StructuralCommand::Speaker {
                passage: self.passage,
                speaker,
            },
        ));
    }
}
impl Writer {
    /// Offer the speakers for line `passage`, on a slip anchored on its speaker label.
    pub fn choose_speaker(&mut self, passage: String) {
        if self.phase != Phase::Writing || self.navigation.busy() || self.work.busy() {
            return;
        }
        let speakers = match self.session.model().document().speakers() {
            Ok(speakers) => speakers,
            Err(error) => {
                self.status = error.to_string();
                return;
            }
        };
        let current = self
            .reading
            .passage(&passage)
            .and_then(|line| match &line.kind {
                recite_writer_model::PassageKind::Dialogue { speaker } => speaker.clone(),
                recite_writer_model::PassageKind::Choice { .. } => None,
            });
        self.pending_choice = Some(crate::choices::Request::Speaker(SpeakerRequest {
            window: self.window,
            origin: self.session.model().navigation_stamp(),
            passage,
            current,
            choices: choices::speakers(speakers),
        }));
    }
}
