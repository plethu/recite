//! Reload's UI capture retains shared text/context and scales with open owners, not prose bytes.
use super::*;
use criterion::measurement::{Measurement, WallTime};
use recite_writer_model::workload;

#[test]
fn ten_thousand_passages_capture_only_twenty_shared_document_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    let inputs = workload::project(10_000, 500)?;
    let paths = inputs
        .iter()
        .map(|input| {
            let path = directory.path().join(input.key().as_str());
            fs::write(&path, input.text()).map(|()| path)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut project = ProjectSession::open(directory.path(), None)?;
    for path in &paths {
        project.switch(path)?;
    }
    project.checkpoint()?;
    let files = project.files();
    let model = project.current().model();
    let started = WallTime.start();
    for _ in 0..64 {
        let request = files.reload_request(model)?;
        assert!(Arc::ptr_eq(&request.origin.context, &files.context));
        assert!(Arc::ptr_eq(&request.origin.baseline, &files.saved));
        assert!(Arc::ptr_eq(&request.recovery.baseline, &files.saved));
        assert_eq!(request.origin.retained.len(), 19);
        assert_eq!(request.overlays.len(), 19);
        for overlay in &request.overlays {
            let retained = files
                .retained
                .values()
                .find(|session| session.model.document().key() == overlay.key())
                .ok_or("retained source")?;
            assert!(std::ptr::eq(
                overlay.text(),
                retained.model.document().source()
            ));
        }
    }
    let milliseconds = WallTime.end(started).as_secs_f64() * 1000.;
    let mut prepared = prepare(&project)?;
    project.accept_reload(&mut prepared)?;
    prepared.finish_cleanup()?;
    assert_eq!(project.documents().len(), 20);
    assert_eq!(
        project
            .current()
            .model()
            .document()
            .passage_snapshot()?
            .len(),
        500
    );
    assert_eq!(project.sessions().count(), 20);
    println!(
        "10,000 passages, 20 open owners, 64 Reload captures: {milliseconds:.2} ms; all source/context allocations shared"
    );
    Ok(())
}
