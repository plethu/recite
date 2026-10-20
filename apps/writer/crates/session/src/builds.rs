//! Manifest-selected build jobs, independent of native screen lifetimes.
pub mod job;
#[derive(Default)]
pub struct Builds {
    pub targets: Vec<(String, Vec<String>)>,
    pub selected: Option<String>,
    pub job: Option<job::Job>,
    pub result: Option<job::Outcome>,
}
impl Builds {
    pub fn cancel(&self) {
        if let Some(job) = &self.job {
            job.control.cancel();
        }
    }
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}
impl job::Outcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed { result, .. } if matches!(
            result.status(),
            recite_compiler::authoring::BuildTerminalStatus::Succeeded | recite_compiler::authoring::BuildTerminalStatus::Cancelled
        ))
    }

    pub fn message(&self) -> String {
        use recite_compiler::authoring::BuildTerminalStatus;
        match self {
            Self::Completed { result, recovery, assets, inputs_changed } => match result.status() {
                BuildTerminalStatus::Succeeded if *inputs_changed => "Build finished, but project inputs changed during the build. Build again to include them.".into(),
                BuildTerminalStatus::Succeeded => format!("Built {}", assets.join(", ")),
                BuildTerminalStatus::Cancelled => "Build cancelled. Previous outputs remain available.".into(),
                _ => format!(
                    "Build did not complete.\n{}\n{}\n{}",
                    crate::diagnostic_messages(result.diagnostics()),
                    result.failure().map(ToString::to_string).unwrap_or_default(),
                    recovery.iter().map(|entry| format!("Recovery record: {}", entry.marker().display())).collect::<Vec<_>>().join("\n")
                ),
            },
            Self::Failed(failure) => match failure {
                job::Failure::Preparation(error) => error.to_string(),
                job::Failure::Diagnostics(diagnostics) => crate::diagnostic_messages(diagnostics),
                job::Failure::AssetRetired => "The selected output is no longer declared by the manifest. Reopen Build scenes.".into(),
                job::Failure::NoTargets => "Declare a scene and output in recite.project.toml before building.".into(),
                job::Failure::Publisher(error) => error.to_string(),
                job::Failure::Coordinator { error, recovery } => format!(
                    "{}\n{}",
                    error,
                    recovery.iter().map(|entry| format!("Recovery record: {}", entry.marker().display())).collect::<Vec<_>>().join("\n")
                ),
                job::Failure::WorkerStopped => "Build worker stopped unexpectedly.".into(),
            },
        }
    }
}
