use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind};
use recite_compiler::{BuildCoordinator, BuildGeneration, BuildGenerationError};
use recite_core::Diagnostic;

use super::PROJECT_MANIFEST_FILE;
use super::inputs::{is_generated_output_path, is_project_recite_source};
use crate::error::CliError;
use crate::fs::resolve_project_path;
use crate::i18n::{Messages, MsgId};

const DEBOUNCE: Duration = Duration::from_millis(250);

// The watch host owns monotonic timing for build telemetry. Keeping the clock
// at this boundary prevents wall-clock values from entering compiler state and
// leaves build tests free to inject exact readings.
#[allow(
    clippy::disallowed_methods,
    reason = "host watch timing stays outside the deterministic compiler contract"
)]
pub(super) fn monotonic_now() -> Instant {
    Instant::now()
}

// The absolute deadline is tracked so irrelevant events (generated output
// writes) consume the window without resetting it; do not simplify to a
// per-loop recv_timeout(DEBOUNCE).
pub(super) fn drain_debounce(
    receiver: &mpsc::Receiver<notify::Result<Event>>,
    state: &WatchState,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let mut deadline = monotonic_now() + DEBOUNCE;
    loop {
        let now = monotonic_now();
        if now >= deadline {
            return Ok(());
        }

        match receiver.recv_timeout(deadline - now) {
            Ok(Ok(event)) => {
                // Events are wakeups only. Relevant wakeups extend the fixed
                // debounce; generated asset writes are intentionally ignored.
                if state.is_relevant_event(&event) {
                    deadline = monotonic_now() + DEBOUNCE;
                }
            }
            Ok(Err(error)) => {
                writeln!(
                    stderr,
                    "{}",
                    messages.format(MsgId::WatchEventError, [("error", error.to_string())])
                )?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(CliError::Watch {
                    message: "watcher event channel closed".to_owned(),
                });
            }
        }
    }
}

pub(super) fn watch_error(error: notify::Error) -> CliError {
    CliError::Watch {
        message: format!("failed to start watcher: {error}"),
    }
}

#[derive(Debug)]
pub(super) struct WatchState {
    pub(super) project_root: PathBuf,
    pub(super) schema_path: Option<PathBuf>,
    pub(super) manifest: Option<recite_config::ProjectManifest>,
    pub(super) coordinator: BuildCoordinator,
    build_generation: BuildGeneration,
    last_build_generation: Option<BuildGeneration>,
    preparation_inputs: Vec<String>,
    preparation_diagnostics: Vec<Diagnostic>,
}

impl WatchState {
    pub(super) fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            schema_path: None,
            manifest: None,
            coordinator: BuildCoordinator::new(),
            build_generation: BuildGeneration::initial(),
            last_build_generation: None,
            preparation_inputs: vec![PROJECT_MANIFEST_FILE.to_owned()],
            preparation_diagnostics: Vec::new(),
        }
    }

    pub(super) fn next_build_generation(&mut self) -> Result<BuildGeneration, CliError> {
        let generation = self.build_generation;
        self.last_build_generation = Some(generation);
        // The manifest remains a known input even when discovery or
        // preparation fails before it can enumerate the rest of the project.
        // This keeps a recoverable attempt attributable to its trigger.
        self.preparation_inputs = vec![PROJECT_MANIFEST_FILE.to_owned()];
        self.preparation_diagnostics.clear();
        self.build_generation = generation.next().map_err(|error| match error {
            BuildGenerationError::Exhausted { current } => CliError::Watch {
                message: format!("build generation {current} cannot advance"),
            },
            _ => CliError::Watch {
                message: "build generation cannot advance".to_owned(),
            },
        })?;
        Ok(generation)
    }

    pub(super) const fn next_build_generation_number(&self) -> u64 {
        self.build_generation.as_u64()
    }

    pub(super) const fn last_build_generation(&self) -> Option<BuildGeneration> {
        self.last_build_generation
    }

    pub(super) fn set_preparation_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        self.preparation_diagnostics = diagnostics;
    }

    pub(super) fn set_preparation_inputs(&mut self, mut inputs: Vec<String>) {
        inputs.sort();
        inputs.dedup();
        self.preparation_inputs = inputs;
    }

    pub(super) fn preparation_inputs(&self) -> &[String] {
        &self.preparation_inputs
    }

    pub(super) fn preparation_diagnostics(&self) -> &[Diagnostic] {
        &self.preparation_diagnostics
    }

    pub(super) fn update_from_discovery(
        &mut self,
        discovery: &recite_config::ProjectDiscoveryReport,
    ) {
        self.project_root = discovery.manifest().project_root().to_owned();
        self.manifest = Some(discovery.manifest().clone());
        self.schema_path = discovery
            .manifest()
            .source()
            .manifest()
            .project
            .schema
            .as_deref()
            .map(|schema| self.project_root.join(schema));
    }

    pub(super) fn manifest_path(&self) -> PathBuf {
        self.project_root.join(PROJECT_MANIFEST_FILE)
    }

    pub(super) fn is_relevant_event(&self, event: &Event) -> bool {
        if matches!(event.kind, EventKind::Access(_)) {
            return false;
        }

        // Publishing creates output parent directories. Ignore that narrow
        // create wakeup, while still allowing remove/rename events for a
        // configured source directory that happens to contain an output.
        if matches!(event.kind, EventKind::Create(_))
            && !event.paths.is_empty()
            && event.paths.iter().all(|path| {
                let path = if path.is_absolute() {
                    path.to_owned()
                } else {
                    self.project_root.join(path)
                };
                is_generated_output_path(&path) || self.is_generated_output_container(&path)
            })
        {
            return false;
        }

        event.paths.is_empty() || event.paths.iter().any(|path| self.is_relevant_path(path))
    }

    pub(super) fn is_relevant_path(&self, path: &Path) -> bool {
        let path = if path.is_absolute() {
            path.to_owned()
        } else {
            self.project_root.join(path)
        };

        if is_generated_output_path(&path) {
            return false;
        }
        if path == self.manifest_path() {
            return true;
        }
        if self
            .schema_path
            .as_ref()
            .is_some_and(|schema| schema == &path)
        {
            return true;
        }
        self.manifest.as_ref().map_or_else(
            || is_project_recite_source(&self.project_root, &path),
            |manifest| manifest.allows_event_path(&path),
        )
    }

    fn is_generated_output_container(&self, path: &Path) -> bool {
        let Some(manifest) = self.manifest.as_ref() else {
            return false;
        };
        if path == self.project_root {
            return false;
        }
        manifest
            .source()
            .manifest()
            .scenes
            .iter()
            .map(|scene| resolve_project_path(&self.project_root, &scene.asset))
            .any(|output| output.starts_with(path))
    }
}
