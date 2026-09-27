//! One background build owns the shared build adapter until its typed outcome is delivered.
use recite_build::{
    ProjectBuildEngine, ProjectBuildPreparationError, ProjectBuildPublisher,
    ProjectBuildPublisherError, ProjectBuildRecovery, ProjectBuildRequest,
};
use recite_compiler::authoring::{
    BuildControl, BuildCoordinator, BuildResult, BuildRunError, BuildTerminalStatus,
};
use recite_core::Diagnostic;
use std::{path::PathBuf, sync::mpsc};

pub(super) enum Outcome {
    Completed {
        result: Box<BuildResult>,
        recovery: Vec<ProjectBuildRecovery>,
        assets: Vec<String>,
        inputs_changed: bool,
    },
    Failed(Failure),
}

pub(super) enum Failure {
    Preparation(ProjectBuildPreparationError),
    Diagnostics(Vec<Diagnostic>),
    AssetRetired,
    NoTargets,
    Publisher(ProjectBuildPublisherError),
    Coordinator {
        error: BuildRunError,
        recovery: Vec<ProjectBuildRecovery>,
    },
    WorkerStopped,
}

pub(super) struct Job {
    pub control: BuildControl,
    result: mpsc::Receiver<Outcome>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
impl Job {
    pub fn start(root: PathBuf, asset: Option<String>) -> Result<Self, String> {
        let (send, result) = mpsc::sync_channel(1);
        let control = BuildControl::new();
        let worker = control.clone();
        std::thread::Builder::new()
            .name("recite-build".into())
            .spawn(move || {
                let _ = send.send(run(root, asset, &worker));
            })
            .map_err(|error| error.to_string())?;
        Ok(Self { control, result })
    }
    pub fn poll(&self) -> Option<Outcome> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Outcome::Failed(Failure::WorkerStopped)),
        }
    }
}
fn run(root: PathBuf, asset: Option<String>, control: &BuildControl) -> Outcome {
    let preparation = match ProjectBuildRequest::prepare(&root) {
        Ok(preparation) => preparation,
        Err(error) => return Outcome::Failed(Failure::Preparation(error)),
    };
    let diagnostics = preparation.diagnostics().to_vec();
    let mut request = match preparation.into_request() {
        Some(request) => request,
        None => return Outcome::Failed(Failure::Diagnostics(diagnostics)),
    };
    if let Some(asset) = asset
        && !request.select_asset(&asset)
    {
        return Outcome::Failed(Failure::AssetRetired);
    }
    if request.targets().is_empty() {
        return Outcome::Failed(Failure::NoTargets);
    }
    let assets = request
        .targets()
        .iter()
        .map(|target| target.asset_id().to_owned())
        .collect::<Vec<_>>();
    let mut engine = ProjectBuildEngine::new(&request);
    let mut publisher = match ProjectBuildPublisher::new(&request) {
        Ok(publisher) => publisher,
        Err(error) => return Outcome::Failed(Failure::Publisher(error)),
    };
    let result = match BuildCoordinator::new().run(
        request.build_request().clone(),
        control,
        &mut engine,
        &mut publisher,
    ) {
        Ok(result) => result,
        Err(error) => {
            return Outcome::Failed(Failure::Coordinator {
                error,
                recovery: publisher.recovery().to_vec(),
            });
        }
    };
    let inputs_changed = result.status() == BuildTerminalStatus::Succeeded
        && !ProjectBuildRequest::prepare(&root)
            .ok()
            .and_then(|preparation| preparation.into_request())
            .is_some_and(|latest| {
                latest.build_request().fingerprints() == request.build_request().fingerprints()
            });
    Outcome::Completed {
        result: Box::new(result),
        recovery: publisher.recovery().to_vec(),
        assets,
        inputs_changed,
    }
}
