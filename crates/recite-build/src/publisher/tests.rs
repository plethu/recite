#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use recite_compiler::{
        authoring::BuildControl, authoring::BuildCoordinator, authoring::BuildPublisher,
        authoring::BuildRequest,
    };
    use tempfile::TempDir;

    use super::super::super::engine::ProjectBuildEngine;
    use super::super::super::recovery::{
        ProjectBuildRecoveryDetail, ProjectBuildRecoveryIoKind, ProjectBuildRecoveryReason,
    };
    use super::super::super::request::ProjectBuildRequest;
    use super::super::ProjectBuildPublisher;

    struct CancelAfterPrepare {
        publisher: ProjectBuildPublisher,
        output_parent: std::path::PathBuf,
    }

    impl BuildPublisher for CancelAfterPrepare {
        type Prepared = <ProjectBuildPublisher as BuildPublisher>::Prepared;

        fn prepare(
            &mut self,
            request: &BuildRequest,
            candidates: &[recite_compiler::authoring::BuildCandidate],
            control: &BuildControl,
        ) -> Result<Self::Prepared, recite_compiler::authoring::PublishFailure> {
            let prepared = self.publisher.prepare(request, candidates, control)?;
            fs::set_permissions(&self.output_parent, fs::Permissions::from_mode(0o500))
                .expect("deny stage cleanup");
            control.cancel();
            Ok(prepared)
        }

        fn abort(
            &mut self,
            prepared: Option<Self::Prepared>,
            reason: recite_compiler::authoring::PublishAbortReason,
        ) {
            self.publisher.abort(prepared, reason);
            fs::set_permissions(&self.output_parent, fs::Permissions::from_mode(0o700))
                .expect("restore stage directory");
        }

        fn commit(
            &mut self,
            prepared: Self::Prepared,
        ) -> recite_compiler::authoring::PublishOutcome {
            self.publisher.commit(prepared)
        }
    }

    fn write(root: &Path, name: &str, content: &str) {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(path, content).expect("file");
    }

    #[test]
    fn cancelled_prepare_failure_keeps_typed_recovery() {
        let temp = TempDir::new().expect("tempdir");
        write(
            temp.path(),
            "recite.project.toml",
            "format_version = 1\n\n[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
        );
        write(
            temp.path(),
            "dialogue/main.recite",
            ":: start default speaker=hazel\n> intro@11111111111111111111\n  Hello.\n-> END\n",
        );
        let request = ProjectBuildRequest::prepare(temp.path())
            .expect("prepare request")
            .into_request()
            .expect("ready request");
        let output_parent = temp.path().join("compiled");
        let publisher = ProjectBuildPublisher::new(&request).expect("publisher");
        let mut publisher = CancelAfterPrepare {
            publisher,
            output_parent,
        };
        let mut engine = ProjectBuildEngine::new(&request);
        let control = BuildControl::new();
        let mut coordinator = BuildCoordinator::new();
        let result = coordinator
            .run(
                request.build_request().clone(),
                &control,
                &mut engine,
                &mut publisher,
            )
            .expect("cancel transition");
        assert_eq!(
            result.status(),
            recite_compiler::authoring::BuildTerminalStatus::Cancelled
        );
        assert!(!temp.path().join("compiled/dialogue.recitec").exists());
        assert_eq!(publisher.publisher.recovery().len(), 1);
        assert_eq!(
            publisher.publisher.recovery()[0].reason(),
            ProjectBuildRecoveryReason::StageCleanupFailed
        );
        match publisher.publisher.recovery()[0].detail() {
            ProjectBuildRecoveryDetail::Io {
                kind: ProjectBuildRecoveryIoKind::PermissionDenied,
                raw_os_error: Some(13),
                message,
            } => assert!(!message.is_empty()),
            other => panic!("unexpected recovery detail: {other:?}"),
        }
        assert!(publisher.publisher.recovery()[0].marker().is_file());
    }
}
