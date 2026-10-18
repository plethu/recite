use super::*;
use std::time::Duration;

fn project() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    std::fs::write(
        dir.path().join("scene.recite"),
        ":: start default\n-> END\n",
    )?;
    Ok(dir)
}

fn open(path: PathBuf, forwarded: Option<Arc<AtomicBool>>) -> std::io::Result<Load> {
    Load::open(
        OpenProject {
            path,
            view: recite_config::WriterView::Script,
            route: None,
        },
        forwarded,
    )
}

fn result(load: &Load) -> LoadResult {
    for _ in 0..10_000 {
        if let Some(result) = load.try_result() {
            return result;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("load did not finish");
}

fn finished(load: &Load) {
    for _ in 0..10_000 {
        if load
            .worker
            .as_ref()
            .is_none_or(|worker| worker.is_finished())
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("load worker did not finish");
}

#[test]
fn cancelled_result_is_rejected_and_releases_project_leases()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = project()?;
    let load = open(dir.path().to_owned(), None)?;
    load.cancel();
    assert!(matches!(result(&load), Err(FileError::Interrupted(_))));
    drop(load);
    let mut opened = ProjectFiles::open(dir.path())?;
    opened.workbench()?;
    assert_eq!(
        std::fs::read_to_string(dir.path().join("scene.recite"))?,
        ":: start default\n-> END\n"
    );
    Ok(())
}

#[test]
fn forwarded_cancellation_stops_before_opening_files() -> Result<(), Box<dyn std::error::Error>> {
    let dir = project()?;
    let stopped = Arc::new(AtomicBool::new(true));
    let load = open(dir.path().to_owned(), Some(stopped))?;
    assert!(matches!(result(&load), Err(FileError::Interrupted(_))));
    drop(load);
    assert!(!dir.path().join(".recite-manifest-draft.lock").exists());
    assert!(
        !dir.path()
            .join("scene.recite.recite-editor-recovery.lock")
            .exists()
    );
    Ok(())
}

#[test]
fn dropping_a_load_joins_and_releases_its_resources() -> Result<(), Box<dyn std::error::Error>> {
    let dir = project()?;
    drop(open(dir.path().to_owned(), None)?);
    let mut opened = ProjectFiles::open(dir.path())?;
    opened.workbench()?;
    Ok(())
}

#[test]
fn cancelling_a_completed_load_discards_the_queued_project()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = project()?;
    let load = open(dir.path().to_owned(), None)?;
    finished(&load);
    // The successfully queued project still owns its recovery lease.
    assert!(ProjectFiles::open(dir.path()).is_err());
    load.cancel();
    assert!(matches!(result(&load), Err(FileError::Interrupted(_))));
    let mut reopened = ProjectFiles::open(dir.path())?;
    reopened.workbench()?;
    Ok(())
}

#[test]
fn forwarded_cancellation_also_discards_a_completed_load() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = project()?;
    let stopped = Arc::new(AtomicBool::new(false));
    let load = open(dir.path().to_owned(), Some(stopped.clone()))?;
    finished(&load);
    assert!(ProjectFiles::open(dir.path()).is_err());
    stopped.store(true, Ordering::Release);
    assert!(matches!(result(&load), Err(FileError::Interrupted(_))));
    let mut reopened = ProjectFiles::open(dir.path())?;
    reopened.workbench()?;
    Ok(())
}

#[test]
fn source_route_hint_applies_only_to_its_document() -> Result<(), Box<dyn std::error::Error>> {
    for (scene, expected) in [("scene.recite", true), ("other.recite", false)] {
        let dir = project()?;
        let load = Load::open(
            OpenProject {
                path: dir.path().to_owned(),
                view: recite_config::WriterView::Script,
                route: Some(format!("recite://writer/write?scene={scene}&view=source")),
            },
            None,
        )?;
        let opened = result(&load)?;
        assert_eq!(
            opened.workbench.view() == &recite_writer_model::View::Source,
            expected
        );
        assert_eq!(
            opened.view == Some(recite_config::WriterView::Source),
            expected
        );
        assert!(!opened.workbench.has_draft());
    }
    Ok(())
}

#[test]
fn recovered_fields_take_priority_over_a_source_route_hint()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_writer_model::View;
    let text = ":: start default\n> line@11111111111111111111\n  Original.\n-> END\n";
    for view in [View::Source, View::Passage("11111111111111111111".into())] {
        let dir = project()?;
        std::fs::write(dir.path().join("scene.recite"), text)?;
        let mut files = ProjectFiles::open(dir.path())?;
        let mut model = files.workbench()?;
        model.select(view.clone())?;
        model.set_draft("Unapplied 🦊".into());
        files.checkpoint(&model)?;
        drop(files);
        let load = Load::open(
            OpenProject {
                path: dir.path().to_owned(),
                view: recite_config::WriterView::Script,
                route: Some("recite://writer/write?scene=scene.recite&view=source".into()),
            },
            None,
        )?;
        let mut opened = result(&load)?;
        assert_eq!(opened.workbench.view(), &view);
        assert_eq!(opened.workbench.draft(), "Unapplied 🦊");
        assert_eq!(opened.workbench.document().source(), text);
        assert!(opened.workbench.has_draft());
        assert!(opened.workbench.select(View::Source).is_err());
    }
    Ok(())
}

#[test]
fn preferred_source_can_skip_projections_without_overriding_later_preferences()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = project()?;
    let load = Load::open(
        OpenProject {
            path: dir.path().to_owned(),
            view: recite_config::WriterView::Source,
            route: None,
        },
        None,
    )?;
    let opened = result(&load)?;
    assert_eq!(opened.workbench.view(), &recite_writer_model::View::Source);
    assert!(opened.view.is_none());
    Ok(())
}
