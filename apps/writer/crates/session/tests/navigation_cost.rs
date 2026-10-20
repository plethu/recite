//! Fixed 10,000-passage navigation workload: reuse analysis and bound idle workers.
use criterion::measurement::{Measurement, WallTime};
use recite_writer_model::workload;
use recite_writer_session::ProjectSession;
use std::fs;

fn recovery_workers() -> Option<usize> {
    Some(
        fs::read_dir("/proc/self/task")
            .ok()?
            .filter_map(Result::ok)
            .filter_map(|entry| fs::read_to_string(entry.path().join("comm")).ok())
            .filter(|name| name.trim() == "recite-recovery")
            .count(),
    )
}

#[test]
fn retained_navigation_reuses_analysis_and_parks_idle_recovery_workers() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n",
    )
    .unwrap();
    let inputs = workload::project(10_000, 500).unwrap();
    let paths = inputs
        .iter()
        .map(|input| {
            let path = directory.path().join(input.key().as_str());
            fs::write(&path, input.text()).unwrap();
            path
        })
        .collect::<Vec<_>>();
    let mut project = ProjectSession::open(directory.path(), None).unwrap();
    for path in &paths {
        project.switch(path).unwrap();
    }
    let analyses = paths
        .iter()
        .map(|path| {
            project.switch(path).unwrap();
            (
                path.clone(),
                project.current().model().document().diagnostic_snapshot(),
            )
        })
        .collect::<Vec<_>>();
    let started = WallTime.start();
    for _ in 0..3 {
        for (path, analysis) in &analyses {
            project.switch(path).unwrap();
            assert!(
                analysis == &project.current().model().document().diagnostic_snapshot(),
                "clean revisit rebuilt compiler analysis"
            );
            assert!(!project.current().dirty());
            assert!(
                project
                    .current()
                    .model()
                    .document()
                    .diagnostics()
                    .is_empty()
            );
        }
    }
    let clean_revisit_ms = WallTime.end(started).as_secs_f64() * 1000.;
    for (index, path) in paths.iter().enumerate() {
        project.switch(path).unwrap();
        project
            .current_mut()
            .edit(|model| {
                model.set_draft(format!("Retained draft {index}."));
                Ok(())
            })
            .unwrap();
        if let Some(workers) = recovery_workers() {
            assert!(
                workers <= 1,
                "{workers} recovery workers exceed the one active owner"
            );
        }
    }
    project.checkpoint().unwrap();
    if let Some(workers) = recovery_workers() {
        assert_eq!(
            workers, 0,
            "idle retained owners kept recovery workers alive"
        );
    }

    for (index, path) in paths.iter().enumerate() {
        project.switch(path).unwrap();
        assert_eq!(
            project.current().model().draft(),
            format!("Retained draft {index}.")
        );
        assert_eq!(
            project.current().model().document().source(),
            inputs[index].text()
        );
    }
    assert_eq!(project.sessions().count(), 20);
    println!(
        "10,000 passages; 20 retained owners; 60 warm switches: {clean_revisit_ms:.2} ms; unchanged analysis reused; at most one recovery worker"
    );
}
