use recite_benchmarks::{BenchmarkFixture, lsp::LspBenchmarkProject, project::BenchmarkProject};

#[test]
fn authored_lsp_probes_resolve_project_relative_targets() -> Result<(), Box<dyn std::error::Error>>
{
    let project = BenchmarkProject::load_fixture(BenchmarkFixture::RealisticV1Pack)?;
    let lsp = LspBenchmarkProject::load(&project)?;
    let mut driver = lsp.driver();
    let probes = lsp.probes();
    assert_eq!(probes.document.project_relative_path, "src/arrival.recite");
    let _ = driver.open_file(&probes.document);
    let response = serde_json::to_value(driver.completion(&probes.completion))?;
    let items = response
        .as_array()
        .ok_or("completion response is an array")?;
    assert!(items.iter().any(|item| item["label"] == "objects"));
    let definition = serde_json::to_value(driver.definition(&probes.definition))?;
    assert_eq!(
        definition["uri"],
        probes
            .definition
            .uri
            .as_str()
            .replace("arrival.recite", "objects.recite"),
    );
    assert_eq!(definition["range"]["start"]["line"], 0);
    assert_eq!(definition["range"]["start"]["character"], 3);
    assert!(driver.rename(&probes.rename, "renamed").is_some());
    Ok(())
}
