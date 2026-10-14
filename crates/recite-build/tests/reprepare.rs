use recite_build::ProjectBuildRequest;
use recite_compiler::authoring::{BuildGeneration, SnapshotGeneration};

mod support;

fn assert_current_matches(request: &ProjectBuildRequest) {
    assert_eq!(
        request.reprepare(),
        ProjectBuildRequest::prepare_with_generations(
            request.project_root(),
            request.build_request().generation(),
            request.build_request().snapshot_generation(),
        ),
    );
}

#[test]
fn unchanged_inputs_share_capture_and_restore_all_targets() -> Result<(), Box<dyn std::error::Error>>
{
    for schema in [false, true] {
        let (directory, _) = support::project(schema, 3)?;
        let original = ProjectBuildRequest::prepare_with_generations(
            directory.path(),
            BuildGeneration::new(19),
            SnapshotGeneration::new(23),
        )?
        .into_request()
        .ok_or("request")?;
        assert!(original.diagnostics().is_empty());
        let mut selected = original.clone();
        assert!(selected.select_asset("compiled/0.recitec"));
        let current = selected
            .reprepare()?
            .into_request()
            .ok_or("current request")?;
        assert_eq!(current, original);
        assert!(std::ptr::eq(
            current.build_request().inputs().as_ptr(),
            original.build_request().inputs().as_ptr()
        ));
        assert_current_matches(&selected);
    }
    Ok(())
}

#[test]
fn source_and_discovery_changes_match_full_preparation() -> Result<(), Box<dyn std::error::Error>> {
    for schema in [false, true] {
        let (directory, request) = support::project(schema, 3)?;
        let other = directory.path().join("dialogue/other.recite");
        let original = std::fs::read_to_string(&other)?;
        for text in [
            original.replace("Au revoir.", "À bientôt, 世界!"),
            original.replace('\n', "\r\n"),
            ":: other\n:if broken(\n  bad\n".into(),
            original.clone(),
        ] {
            std::fs::write(&other, text)?;
            assert_current_matches(&request);
        }
        let added = directory.path().join("dialogue/new.recite");
        std::fs::write(
            &added,
            ":: new\n> new@33333333333333333333\n  New.\n-> END\n",
        )?;
        assert_current_matches(&request);
        std::fs::remove_file(&added)?;
        std::fs::remove_file(&other)?;
        assert_current_matches(&request);
        std::fs::write(&other, [0xff])?;
        assert_current_matches(&request);
        std::fs::write(&other, &original)?;
        assert_current_matches(&request);
    }
    Ok(())
}

#[test]
fn manifest_schema_and_warning_changes_match_full_preparation()
-> Result<(), Box<dyn std::error::Error>> {
    for schema in [false, true] {
        let (directory, request) = support::project(schema, 3)?;
        let path = directory.path().join("recite.project.toml");
        let original = std::fs::read_to_string(&path)?;
        for manifest in [
            format!("# same semantics, different captured text\n{original}"),
            original.replace("compiled/0.recitec", "compiled/replaced.recitec"),
            original.replace(
                "source_roots = [\"dialogue\"]",
                "source_roots = [\"dialogue\", \"dialogue/.\"]",
            ),
            original.replace(
                "source_roots = [\"dialogue\"]",
                "source_roots = [\"missing\"]",
            ),
            "invalid [".into(),
            original.clone(),
        ] {
            std::fs::write(&path, manifest)?;
            assert_current_matches(&request);
            if let Ok(prepared) = ProjectBuildRequest::prepare(directory.path())
                && let Some(warnings) = prepared.into_request()
            {
                assert_current_matches(&warnings);
            }
        }
        if schema {
            let path = directory.path().join("schema.json");
            let original = std::fs::read_to_string(&path)?;
            for text in [
                format!("\n{original}\n"),
                original.replace("Hazel", "Changed"),
                "{\"schema_version\":\"one\"}".into(),
                original.clone(),
            ] {
                std::fs::write(&path, text)?;
                assert_current_matches(&request);
            }
            std::fs::remove_file(&path)?;
            assert_current_matches(&request);
            std::fs::create_dir(&path)?;
            assert_current_matches(&request);
        }
    }
    Ok(())
}

#[test]
fn overlapping_roots_revalidate_and_preserve_warning_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let (directory, _) = support::project(false, 3)?;
    let path = directory.path().join("recite.project.toml");
    let original = std::fs::read_to_string(&path)?;
    let overlapping = original.replace(
        "source_roots = [\"dialogue\"]",
        "source_roots = [\".\", \"dialogue\"]",
    );
    std::fs::write(&path, overlapping)?;
    let warning_request = ProjectBuildRequest::prepare(directory.path())?
        .into_request()
        .ok_or("warning request")?;
    assert!(!warning_request.diagnostics().is_empty());
    assert_current_matches(&warning_request);
    std::fs::write(&path, original)?;
    assert_current_matches(&warning_request);
    assert!(warning_request.reprepare()?.diagnostics().is_empty());
    Ok(())
}
