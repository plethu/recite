#![expect(
    clippy::expect_used,
    reason = "project discovery integration tests fail fast on filesystem fixture setup and report assertions; standalone test targets are outside clippy.toml's test allowance"
)]

use std::fs;

use recite_config::{Coverage, DiscoveryDiagnostic, ProjectDiscoveryError, discover_project};
use tempfile::TempDir;

fn manifest(source_roots: &str, excludes: &str) -> String {
    format!(
        "format_version = 1\n\n[discovery]\nsource_roots = {source_roots}\nexcludes = {excludes}\n"
    )
}

fn write(root: &std::path::Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("test parent")).expect("test directory");
    fs::write(path, content).expect("test file");
}

#[test]
fn version_and_default_discovery_are_explicit() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", "format_version = 1\n");
    write(temp.path(), "dialogue.recite", ":: start\n");

    let report = discover_project(temp.path()).expect("project discovery");
    assert_eq!(report.manifest().roots().len(), 1);
    assert_eq!(report.manifest().roots()[0].relative(), ".");
    assert_eq!(report.documents().len(), 1);
    assert_eq!(report.documents()[0].key().as_str(), "dialogue.recite");
    assert_eq!(report.coverage(), Coverage::Complete);
}

#[test]
fn missing_and_future_versions_are_typed_and_nearest_malformed_stops_search() {
    let temp = TempDir::new().expect("tempdir");
    let nested = temp.path().join("nested");
    fs::create_dir_all(&nested).expect("nested");
    write(temp.path(), "recite.project.toml", "project = {}\n");

    let missing = nested.join("missing.recite");
    assert!(matches!(
        discover_project(&missing),
        Err(ProjectDiscoveryError::MissingFormatVersion { .. })
    ));

    write(
        temp.path(),
        "nested/recite.project.toml",
        "format_version = 2\n",
    );
    assert!(matches!(
        discover_project(&missing),
        Err(ProjectDiscoveryError::UnsupportedFormatVersion { found: 2, .. })
    ));
    write(
        temp.path(),
        "nested/recite.project.toml",
        "format_version = [\n",
    );
    assert!(matches!(
        discover_project(&missing),
        Err(ProjectDiscoveryError::Malformed { .. })
    ));
}

#[test]
fn explicit_file_and_missing_path_find_the_nearest_manifest() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", "format_version = 1\n");
    write(temp.path(), "src/dialogue.recite", ":: source\n");

    let report =
        discover_project(temp.path().join("src/dialogue.recite")).expect("explicit source path");
    assert_eq!(report.manifest().project_root(), temp.path());
    let report =
        discover_project(temp.path().join("src/new/dialogue.recite")).expect("missing source path");
    assert_eq!(report.manifest().project_root(), temp.path());
}

#[test]
fn roots_preserve_order_warn_on_overlap_and_reject_duplicates() {
    let temp = TempDir::new().expect("tempdir");
    write(
        temp.path(),
        "recite.project.toml",
        &manifest(r#"["src", "other", "src/nested"]"#, "[]"),
    );
    write(temp.path(), "src/z.recite", ":: z\n");
    write(temp.path(), "src/nested/a.recite", ":: a\n");
    fs::create_dir_all(temp.path().join("other")).expect("unrelated root");
    let report = discover_project(temp.path()).expect("overlap is a warning");
    assert_eq!(
        report
            .documents()
            .iter()
            .map(|doc| doc.key().as_str())
            .collect::<Vec<_>>(),
        ["src/nested/a.recite", "src/z.recite"]
    );
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|diagnostic| matches!(diagnostic, DiscoveryDiagnostic::OverlappingRoot { .. }))
    );
    assert!(report.diagnostics().iter().any(|diagnostic| matches!(
        diagnostic,
        DiscoveryDiagnostic::OverlappingRoot { owner, .. }
            if owner.ends_with("src")
    )));
    assert_eq!(report.documents()[0].root_index(), 0);
    assert_eq!(report.coverage(), Coverage::Complete);

    write(
        temp.path(),
        "recite.project.toml",
        &manifest(r#"["src", "./src"]"#, "[]"),
    );
    assert!(matches!(
        discover_project(temp.path()),
        Err(ProjectDiscoveryError::DuplicateRoot { .. })
    ));
}

#[test]
fn builtins_custom_excludes_and_unicode_are_deterministic() {
    let temp = TempDir::new().expect("tempdir");
    write(
        temp.path(),
        "recite.project.toml",
        &manifest(r#"["."]"#, r#"["custom/**"]"#),
    );
    for path in [
        "z.recite",
        "a.recite",
        "custom/ignored.recite",
        ".hidden/ignored.recite",
        "target/ignored.recite",
        "build/ignored.recite",
        "generated/ignored.recite",
        "café.recite",
    ] {
        write(temp.path(), path, ":: source\n");
    }
    let report = discover_project(temp.path()).expect("project discovery");
    assert_eq!(
        report
            .documents()
            .iter()
            .map(|doc| doc.key().as_str())
            .collect::<Vec<_>>(),
        ["a.recite", "café.recite", "z.recite"]
    );
}

#[test]
fn exclude_dot_segments_are_normalized_before_matching() {
    let temp = TempDir::new().expect("tempdir");
    write(
        temp.path(),
        "recite.project.toml",
        &manifest(r#"["."]"#, r#"["./generated/**", "nested/./ignored/**"]"#),
    );
    write(temp.path(), "kept.recite", ":: kept\n");
    write(temp.path(), "generated/ignored.recite", ":: ignored\n");
    write(temp.path(), "nested/ignored/ignored.recite", ":: ignored\n");
    let report = discover_project(temp.path()).expect("project discovery");
    assert_eq!(
        report
            .manifest()
            .excludes()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["generated/**", "nested/ignored/**"]
    );
    assert_eq!(report.documents().len(), 1);
}

#[test]
fn invalid_utf8_source_is_retained_as_partial_coverage() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", "format_version = 1\n");
    fs::write(temp.path().join("broken.recite"), [0xff, 0xfe]).expect("invalid source");

    let report = discover_project(temp.path()).expect("project discovery");
    assert!(report.documents().is_empty());
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|diagnostic| matches!(diagnostic, DiscoveryDiagnostic::NonUtf8Source { .. }))
    );
    assert_eq!(report.coverage(), Coverage::Partial);
}

#[test]
fn invalid_roots_and_excludes_reject_windows_and_parent_shapes() {
    let temp = TempDir::new().expect("tempdir");
    for (roots, excludes) in [
        (r#"["/absolute"]"#, "[]"),
        (r#"["C:\\source"]"#, "[]"),
        (r#"["../outside"]"#, "[]"),
        (r#"["."]"#, r#"["!generated/**"]"#),
        (r#"["."]"#, r#"["C:\\generated"]"#),
        (r#"["."]"#, r#"["../generated"]"#),
    ] {
        write(
            temp.path(),
            "recite.project.toml",
            &manifest(roots, excludes),
        );
        let error = discover_project(temp.path()).expect_err("invalid path shape");
        assert!(matches!(
            error,
            ProjectDiscoveryError::InvalidSourceRoot { .. }
                | ProjectDiscoveryError::InvalidExclude { .. }
        ));
    }
}

#[cfg(unix)]
#[test]
fn symlink_directories_are_not_recurred_and_outside_files_are_reported() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().expect("tempdir");
    let outside = TempDir::new().expect("outside tempdir");
    write(temp.path(), "recite.project.toml", "format_version = 1\n");
    write(temp.path(), "real/inside.recite", ":: source\n");
    write(outside.path(), "outside.recite", ":: source\n");
    symlink(temp.path().join("real"), temp.path().join("linked-dir")).expect("directory link");
    symlink(
        outside.path().join("outside.recite"),
        temp.path().join("outside.recite"),
    )
    .expect("file link");
    let report = discover_project(temp.path()).expect("project discovery");
    assert_eq!(
        report
            .documents()
            .iter()
            .map(|doc| doc.key().as_str())
            .collect::<Vec<_>>(),
        ["real/inside.recite"]
    );
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|diagnostic| matches!(diagnostic, DiscoveryDiagnostic::FileOutsideProject { .. }))
    );
    assert_eq!(report.coverage(), Coverage::Partial);
}

#[cfg(unix)]
#[test]
fn symlink_files_must_remain_inside_their_configured_source_root() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().expect("tempdir");
    write(
        temp.path(),
        "recite.project.toml",
        &manifest(r#"["src"]"#, "[]"),
    );
    write(temp.path(), "docs/outside.recite", ":: outside\n");
    fs::create_dir_all(temp.path().join("src")).expect("source root");
    symlink(
        temp.path().join("docs/outside.recite"),
        temp.path().join("src/link.recite"),
    )
    .expect("source link");

    let report = discover_project(temp.path()).expect("project discovery");
    assert!(report.documents().is_empty());
    assert!(report.diagnostics().iter().any(|diagnostic| matches!(
        diagnostic,
        DiscoveryDiagnostic::FileOutsideProject { path, target }
            if path.ends_with("src/link.recite") && target.ends_with("docs/outside.recite")
    )));
    assert_eq!(report.coverage(), Coverage::Partial);
}

#[cfg(unix)]
#[test]
fn canonical_symlink_targets_cannot_bypass_excludes() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().expect("tempdir");
    write(
        temp.path(),
        "recite.project.toml",
        &manifest(r#"["src"]"#, r#"["src/vendor/**"]"#),
    );
    write(temp.path(), "src/vendor/target.recite", ":: target\n");
    fs::create_dir_all(temp.path().join("src")).expect("source root");
    symlink(
        temp.path().join("src/vendor/target.recite"),
        temp.path().join("src/link.recite"),
    )
    .expect("excluded target link");

    let report = discover_project(temp.path()).expect("project discovery");
    assert!(report.documents().is_empty());
    assert_eq!(report.coverage(), Coverage::Complete);
}
