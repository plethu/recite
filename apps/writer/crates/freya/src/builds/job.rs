//! The existing project publisher owns output staging, cancellation and recovery.
use recite_cli::watch::{ProjectBuildEngine, ProjectBuildPublisher, ProjectBuildRequest};
use recite_compiler::{BuildControl, BuildCoordinator, BuildTerminalStatus};
use std::{path::PathBuf, sync::mpsc};

pub(super) struct Job {
    pub control: BuildControl,
    result: mpsc::Receiver<Result<String, String>>,
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
            .map_err(|e| e.to_string())?;
        Ok(Self { control, result })
    }
    pub fn poll(&self) -> Option<Result<String, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Build worker stopped unexpectedly.".into()))
            }
        }
    }
}
fn run(root: PathBuf, asset: Option<String>, control: &BuildControl) -> Result<String, String> {
    let preparation = ProjectBuildRequest::prepare(&root).map_err(|e| e.to_string())?;
    let diagnostics = preparation.diagnostics().to_vec();
    let mut request = preparation
        .into_request()
        .ok_or_else(|| crate::project_context::diagnostic_messages(&diagnostics))?;
    if let Some(asset) = asset
        && !request.select_asset(&asset)
    {
        return Err(
            "The selected output is no longer declared by the manifest. Reopen Build scenes."
                .into(),
        );
    }
    if request.targets().is_empty() {
        return Err("Declare a scene and output in recite.project.toml before building.".into());
    }
    let mut engine = ProjectBuildEngine::new(&request);
    let mut publisher = ProjectBuildPublisher::new(&request).map_err(|e| e.to_string())?;
    let result = BuildCoordinator::new()
        .run(
            request.build_request().clone(),
            control,
            &mut engine,
            &mut publisher,
        )
        .map_err(|e| e.to_string())?;
    match result.status() {
        BuildTerminalStatus::Succeeded => {
            let fresh = ProjectBuildRequest::prepare(&root)
                .ok()
                .and_then(|p| p.into_request())
                .is_some_and(|latest| {
                    latest.build_request().fingerprints() == request.build_request().fingerprints()
                });
            if !fresh {
                return Ok("Build finished, but project inputs changed during the build. Build again to include them.".into());
            }
            Ok(format!(
                "Built {}",
                request
                    .targets()
                    .iter()
                    .map(|t| t.asset_id())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
        BuildTerminalStatus::Cancelled => {
            Ok("Build cancelled. Previous outputs remain available.".into())
        }
        _ => Err(format!(
            "Build did not complete.\n{}\n{}\n{}",
            crate::project_context::diagnostic_messages(result.diagnostics()),
            result
                .failure()
                .map(ToString::to_string)
                .unwrap_or_default(),
            publisher
                .recovery()
                .iter()
                .map(|r| format!("Recovery record: {}", r.marker().display()))
                .collect::<Vec<_>>()
                .join("\n")
        )),
    }
}
