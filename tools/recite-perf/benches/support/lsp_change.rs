//! A fixed small edit surrounded by unrelated manifestless source files.
use recite_lsp::bench_support::{LspBenchmarkConfig, LspBenchmarkDriver, LspDocumentProbe};

pub(super) struct Input {
    pub driver: LspBenchmarkDriver,
    pub document: LspDocumentProbe,
    _directory: tempfile::TempDir,
}

pub(super) fn prepare(files: usize) -> Input {
    let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("workspace: {error}"));
    let path = directory.path().join("main.recite");
    std::fs::write(&path, ":: start default\n-> finish\n:: finish\n-> END\n")
        .unwrap_or_else(|error| panic!("edited source: {error}"));
    for index in 1..files {
        std::fs::write(
            directory.path().join(format!("shard_{index}.recite")),
            format!(":: shard_{index}\n-> END\n"),
        )
        .unwrap_or_else(|error| panic!("unrelated source: {error}"));
    }
    let document = LspDocumentProbe {
        uri: format!("file://{}", path.display())
            .parse()
            .unwrap_or_else(|error| panic!("URI: {error}")),
        path,
        project_relative_path: "main.recite".into(),
    };
    let mut driver =
        LspBenchmarkDriver::new(&LspBenchmarkConfig::new(vec![directory.path().to_owned()]));
    assert_eq!(driver.memory_report().source_files, files);
    assert_eq!(driver.open_file(&document), 0);
    Input {
        driver,
        document,
        _directory: directory,
    }
}

pub(super) fn validate((input, count): (Input, usize)) {
    assert_eq!(count, 0);
    let publication = input
        .driver
        .diagnostics_publication(&input.document)
        .unwrap_or_else(|| panic!("edited publication"));
    assert_eq!(publication.uri, input.document.uri);
    assert_eq!(publication.version, Some(2));
    assert!(publication.diagnostics.is_empty());
    let probes = input.driver.probes();
    assert!(input.driver.definition(&probes.definition).is_some());
}
