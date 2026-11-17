//! Trial publication stays with the revision-checked workbench.
use super::{Workbench, WorkbenchError};
use crate::{
    PreparedPreview, Preview, PreviewPage, PreviewProvenance, PreviewRequest, PreviewSetup,
    PreviewTurn,
};

/// The most turns a transcript keeps; the oldest give way first.
const TRANSCRIPT: usize = 200;

impl Workbench {
    pub fn preview_provenance(&self) -> Option<&PreviewProvenance> {
        self.preview.as_ref().and_then(Preview::provenance)
    }
    pub fn preview_request(&self) -> Result<PreviewRequest, WorkbenchError> {
        self.require_applied()?;
        Ok(PreviewRequest::capture(
            &self.document,
            self.selected_block()?,
        ))
    }
    /// Reject stale preparation without changing the running trial. The caller retains
    /// rejected/replaced sessions so their destruction can happen off the UI thread.
    pub fn accept_preview(
        &mut self,
        prepared: Box<PreparedPreview>,
    ) -> Result<Option<Preview>, Box<PreparedPreview>> {
        if !prepared.request.is_current(self) {
            return Err(prepared);
        }
        self.page = Some(prepared.page);
        self.transcript.clear();
        Ok(self.preview.replace(prepared.preview))
    }
    pub fn start_preview(&mut self) -> Result<(), WorkbenchError> {
        self.start_preview_with(PreviewSetup::default())
    }
    pub fn start_preview_with(&mut self, setup: PreviewSetup) -> Result<(), WorkbenchError> {
        let prepared = self.preview_request()?.prepare(setup)?;
        // No state can change between capture and publication in this synchronous API.
        self.preview = Some(prepared.preview);
        self.page = Some(prepared.page);
        self.transcript.clear();
        Ok(())
    }
    pub fn preview_page(&self) -> Option<&PreviewPage> {
        self.page.as_ref()
    }
    /// The running preview's earlier pages, oldest first, each with the reply chosen after it.
    pub fn preview_transcript(&self) -> &[PreviewTurn] {
        &self.transcript
    }
    pub fn advance_preview(&mut self, choice_index: Option<usize>) -> Result<(), WorkbenchError> {
        let choice = choice_index
            .map(|index| {
                self.page
                    .as_ref()
                    .and_then(|p| p.choices.get(index))
                    .map(|c| (c.id.clone(), c.text.clone()))
                    .ok_or(WorkbenchError::NoPreview)
            })
            .transpose()?;
        let (choice, reply) = choice.unzip();
        let preview = self.preview.as_mut().ok_or(WorkbenchError::NoPreview)?;
        let page = preview.advance(choice)?;
        self.turn(page, reply);
        Ok(())
    }
    pub fn answer_preview(
        &mut self,
        value: recite_runtime::ConditionValue,
    ) -> Result<(), WorkbenchError> {
        let request = self
            .page
            .as_ref()
            .and_then(|p| p.condition.clone())
            .ok_or(WorkbenchError::NoPreview)?;
        let preview = self.preview.as_mut().ok_or(WorkbenchError::NoPreview)?;
        let page = preview.answer(&request, value)?;
        self.turn(page, None);
        Ok(())
    }
    pub fn acknowledge_preview(
        &mut self,
        ack: recite_runtime::EffectAck,
    ) -> Result<(), WorkbenchError> {
        let id = self
            .page
            .as_ref()
            .and_then(|p| p.waiting_effect.clone())
            .ok_or(WorkbenchError::NoPreview)?;
        let preview = self.preview.as_mut().ok_or(WorkbenchError::NoPreview)?;
        let page = preview.acknowledge(id, ack)?;
        self.turn(page, None);
        Ok(())
    }
    /// Move to `page`, the current page joining the transcript with the `reply` chosen after it.
    fn turn(&mut self, page: PreviewPage, reply: Option<String>) {
        if let Some(previous) = self.page.replace(page)
            && (!previous.text.is_empty() || reply.is_some())
        {
            if self.transcript.len() == TRANSCRIPT {
                self.transcript.remove(0);
            }
            // A stand-in prompt was a heading, not a line: only the reply chosen under it stays.
            self.transcript.push(PreviewTurn {
                text: if previous.prompt {
                    String::new()
                } else {
                    previous.text
                },
                speaker: previous.speaker,
                reply,
            });
        }
    }
}
