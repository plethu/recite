use super::*;

#[test]
fn watch_reads_are_background_hashes_and_report_missing_or_nonregular_files()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    // Watching detects external changes even when the new PO is malformed.
    let source = "not a valid PO catalogue\n";
    std::fs::write(&path, source)?;
    let baseline = PoDocumentFingerprint::from_source("previous");
    let mut worker = Worker::new();
    let results = worker.results();
    worker.submit(Request {
        path: path.clone(),
        baseline: baseline.clone(),
    })?;
    let output = results.recv_blocking()?;
    assert_eq!(output.request.path, path);
    assert_eq!(output.request.baseline, baseline);
    assert_eq!(
        output.fingerprint?,
        PoDocumentFingerprint::from_source(source)
    );
    std::fs::remove_file(&path)?;
    worker.submit(Request {
        path: path.clone(),
        baseline: baseline.clone(),
    })?;
    assert!(results.recv_blocking()?.fingerprint.is_err());
    std::fs::create_dir(&path)?;
    worker.submit(Request { path, baseline })?;
    assert!(results.recv_blocking()?.fingerprint.is_err());
    drop(worker);
    assert!(results.recv_blocking().is_err());
    Ok(())
}
