use super::*;
use crate::recovery::Recovery;
use recite_compiler::authoring::Interrupted;
use std::{cell::Cell, collections::BTreeMap, fs};

struct Control {
    calls: Cell<usize>,
    stop_at: usize,
}

impl Control {
    fn new(stop_at: usize) -> Self {
        Self {
            calls: Cell::new(0),
            stop_at,
        }
    }
}

impl WorkControl for Control {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        let call = self.calls.get();
        self.calls.set(call + 1);
        if call >= self.stop_at {
            Err(Interrupted)
        } else {
            Ok(())
        }
    }
}

fn pending_project() -> Result<(tempfile::TempDir, ProjectFiles), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    let mut snapshots = BTreeMap::new();
    for (name, block, anchor) in [("a.recite", "start default", 1), ("b.recite", "other", 2)] {
        let source = format!(":: {block}\n> line@{anchor:020}\n  Saved {name}.\n-> END\n");
        fs::write(dir.path().join(name), &source)?;
        let mut model = Workbench::open(name, &source)?;
        model.set_draft(format!("Recovered {name}."));
        model.apply()?;
        model.set_draft(format!("Unapplied {name}."));
        snapshots.insert(name.into(), Recovery::new(source.into(), model.recovery()));
    }
    let mut files = ProjectFiles::open(dir.path())?;
    let affected = snapshots.keys().cloned().collect();
    files
        .manifest
        .set(files.manifest.text().to_owned(), snapshots, affected)?;
    Ok((dir, files))
}

#[test]
fn interrupted_replay_preserves_both_drafts_at_every_checkpoint()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = pending_project()?;
    let complete = Control::new(usize::MAX);
    files.workbench_with_control(&complete)?;
    assert!(complete.calls.get() > 2);
    for stop_at in 0..complete.calls.get() {
        let (dir, mut files) = pending_project()?;
        let authored = ["a.recite", "b.recite"]
            .map(|name| fs::read_to_string(dir.path().join(name)))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        assert!(
            files
                .workbench_with_control(&Control::new(stop_at))
                .is_err()
        );
        drop(files);
        let mut reopened = ProjectFiles::open(dir.path())?;
        let current = reopened.workbench()?;
        assert!(current.document().source().contains("Recovered a.recite."));
        assert_eq!(current.draft(), "Unapplied a.recite.");
        let other = &reopened
            .retained
            .get(&dir.path().join("b.recite"))
            .ok_or("retained b")?
            .model;
        assert!(other.document().source().contains("Recovered b.recite."));
        assert_eq!(other.draft(), "Unapplied b.recite.");
        for (name, before) in ["a.recite", "b.recite"].iter().zip(&authored) {
            assert_eq!(&fs::read_to_string(dir.path().join(name))?, before);
        }
    }
    Ok(())
}
