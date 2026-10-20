//! Native presentation selects a concrete product owner; examples never pretend to own files.
use recite_writer_model::{
    ConditionValue, EffectAck, PreparedPreview, Preview, WRITER_EXAMPLES, Workbench, WorkbenchError,
};
use recite_writer_session::{
    FileError, ProjectSession, examples::ExampleSessions, project::ProjectFiles,
};
pub(super) enum Session {
    Project(Box<ProjectSession>),
    Examples {
        sessions: ExampleSessions,
        model: Box<Workbench>,
    },
}
impl Session {
    pub fn examples() -> Result<Self, FileError> {
        Ok(Self::Examples {
            sessions: ExampleSessions::default(),
            model: Box::new(WRITER_EXAMPLES[0].open()?),
        })
    }
    pub fn model(&self) -> &Workbench {
        match self {
            Self::Project(project) => project.current().model(),
            Self::Examples { model, .. } => model,
        }
    }
    pub fn edit(
        &mut self,
        action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
    ) -> Result<(), FileError> {
        match self {
            Self::Project(project) => project.current_mut().edit(action),
            Self::Examples { model, .. } => action(model).map_err(FileError::from),
        }
    }
    pub fn project(&self) -> Option<&ProjectSession> {
        match self {
            Self::Project(project) => Some(project),
            Self::Examples { .. } => None,
        }
    }
    pub fn project_mut(&mut self) -> Option<&mut ProjectSession> {
        match self {
            Self::Project(project) => Some(project),
            Self::Examples { .. } => None,
        }
    }
    pub fn parts_mut(&mut self) -> Option<(&mut ProjectFiles, &mut Workbench)> {
        self.project_mut().map(ProjectSession::parts_mut)
    }
    pub fn current_dirty(&self) -> bool {
        match self {
            Self::Project(project) => project.current().dirty(),
            Self::Examples { model, .. } => ExampleSessions::is_modified(model),
        }
    }
    pub fn dirty(&self) -> bool {
        match self {
            Self::Project(project) => project.dirty(),
            Self::Examples { sessions, model } => sessions.dirty(model),
        }
    }
    pub fn select_example(&mut self, index: usize) -> Result<(), FileError> {
        match self {
            Self::Examples { sessions, model } => sessions
                .switch(model, index, |_| Ok(()))
                .map_err(FileError::from),
            Self::Project(_) => Err(FileError::Selection),
        }
    }
    pub fn accept_preview(
        &mut self,
        preview: Box<PreparedPreview>,
    ) -> Result<Option<Preview>, Box<PreparedPreview>> {
        match self {
            Self::Project(project) => project.current_mut().accept_preview(preview),
            Self::Examples { model, .. } => model.accept_preview(preview),
        }
    }
    pub fn advance_preview(&mut self, choice: Option<usize>) -> Result<(), WorkbenchError> {
        match self {
            Self::Project(project) => project.current_mut().advance_preview(choice),
            Self::Examples { model, .. } => model.advance_preview(choice),
        }
    }
    pub fn answer_preview(&mut self, value: ConditionValue) -> Result<(), WorkbenchError> {
        match self {
            Self::Project(project) => project.current_mut().answer_preview(value),
            Self::Examples { model, .. } => model.answer_preview(value),
        }
    }
    pub fn acknowledge_preview(&mut self, ack: EffectAck) -> Result<(), WorkbenchError> {
        match self {
            Self::Project(project) => project.current_mut().acknowledge_preview(ack),
            Self::Examples { model, .. } => model.acknowledge_preview(ack),
        }
    }
}
