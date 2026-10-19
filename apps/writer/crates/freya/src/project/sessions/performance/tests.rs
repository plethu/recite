use super::super::*;
use criterion::measurement::{Measurement, WallTime};
use recite_writer_model::View;

#[test]
#[ignore = "opt-in giant-scene Source navigation timing; excludes widgets and painting"]
fn giant_source_scene_navigation_timing() -> Result<(), Box<dyn std::error::Error>> {
    let passages = std::env::var("RECITE_BENCH_PASSAGES")
        .ok()
        .map(|v| v.parse())
        .transpose()?
        .unwrap_or(100_000);
    let inputs = recite_writer_model::workload::project(passages, passages / 2)?;
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    for input in &inputs {
        let path = dir.path().join(input.key().as_str());
        std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
        std::fs::write(path, input.text())?;
    }
    let mut files = ProjectFiles::open(dir.path())?;
    let mut current = files.workbench()?;
    current.select(View::Source)?;
    let paths = files.paths.clone();
    let mut rows = Vec::new();
    for visit in 0..8 {
        let path = &paths[(visit + 1) % paths.len()];
        let started = WallTime.start();
        files.switch(&mut current, path, Some(View::Source), |_| Ok(()))?;
        rows.push(serde_json::json!({"operation":if visit==0 {"cold_source_switch"}else{"retained_source_switch"},"ms":WallTime.end(started).as_secs_f64()*1000.}));
        assert_eq!(current.view(), &View::Source);
        let expected = inputs
            .iter()
            .find(|i| i.key() == current.document().key())
            .ok_or("input")?;
        assert_eq!(current.document().source(), expected.text());
        assert!(!current.has_draft());
    }
    let outgoing = files.current.clone();
    let mut draft = freya::code_editor::Rope::from_str(current.document().source());
    draft.insert(draft.len_chars(), "\n# Unapplied giant draft\n");
    current.set_draft_snapshot(recite_writer_model::DraftSnapshot::from_rope(draft.clone()));
    let other = paths
        .iter()
        .find(|p| **p != outgoing)
        .ok_or("other scene")?;
    let started = WallTime.start();
    files.switch(&mut current, other, Some(View::Source), |_| Ok(()))?;
    rows.push(serde_json::json!({"operation":"dirty_source_switch","ms":WallTime.end(started).as_secs_f64()*1000.}));
    files.switch(&mut current, &outgoing, Some(View::Source), |_| Ok(()))?;
    assert!(current.has_draft());
    assert!(current.draft_snapshot().matches_rope(&draft));
    println!(
        "{}",
        serde_json::json!({"passages":passages,"documents":inputs.len(),"words":inputs.iter().map(|i|i.text().split_whitespace().count()).sum::<usize>(),"rows":rows,"native":false})
    );
    Ok(())
}
