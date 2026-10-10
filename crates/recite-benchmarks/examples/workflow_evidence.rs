//! Semantic evidence is collected separately from timing and allocation probes.
use recite_benchmarks::BenchmarkFixture;
use recite_benchmarks::preview::PreviewProject;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reports = Vec::new();
    for fixture in BenchmarkFixture::selected_from_env()? {
        let evidence = PreviewProject::load(fixture)?.evidence_report()?;
        if !evidence.restore.events_match {
            return Err("restoring the preview changed subsequent events".into());
        }
        reports.push(serde_json::json!({
            "fixture": fixture.as_str(),
            "event_count": evidence.traversal.event_count,
            "output_count": evidence.traversal.output_count,
            "event_hash": evidence.traversal.event_hash,
            "state_hash": evidence.traversal.state_hash,
            "restored_event_count": evidence.restore.restored_event_count,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&reports)?);
    Ok(())
}
