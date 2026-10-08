use std::{collections::BTreeSet, fs, path::Path};

#[test]
fn every_promised_distribution_target_has_a_native_archive_runner() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let config: toml::Value =
        toml::from_str(&fs::read_to_string(root.join("dist-workspace.toml")).unwrap()).unwrap();
    let promised: BTreeSet<_> = config["dist"]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|target| target.as_str().unwrap())
        .collect();
    let workflow = fs::read_to_string(root.join(".github/workflows/cli-packages.yml")).unwrap();
    let exercised: BTreeSet<_> = workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("target: "))
        .collect();
    assert!(!promised.is_empty());
    assert_eq!(promised, exercised);
}
