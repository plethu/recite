use recite_core::{BlockId, ChoiceId, LineId};

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewPromptIdentity {
    pub(crate) block: BlockId,
    pub(crate) line: Option<LineId>,
    pub(crate) choices: Vec<ChoiceId>,
}

impl PreviewPromptIdentity {
    pub(crate) fn from_parts(block: BlockId, line: Option<LineId>, choices: Vec<ChoiceId>) -> Self {
        Self {
            block,
            line,
            choices,
        }
    }

    #[must_use]
    pub fn block(&self) -> &BlockId {
        &self.block
    }

    #[must_use]
    pub fn line(&self) -> Option<&LineId> {
        self.line.as_ref()
    }

    #[must_use]
    pub fn choices(&self) -> &[ChoiceId] {
        &self.choices
    }
}
