#![expect(
    clippy::expect_used,
    reason = "catalogue discovery integration tests fail fast on filesystem fixture setup; standalone test targets are outside clippy.toml's test allowance"
)]

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;

use recite_config::{ProjectDiscoveryError, discover_project};
use tempfile::TempDir;

fn write(root: &std::path::Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("test parent")).expect("test directory");
    fs::write(path, content).expect("test file");
}

fn project(catalogs: &str) -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    write(
        temp.path(),
        "recite.project.toml",
        &format!(
            "format_version = 1\n\n[discovery]\nexcludes = [\"locales/skip/**\"]\ncatalogs = {catalogs}\n"
        ),
    );
    write(temp.path(), "dialogue.recite", ":: start\n");
    temp
}

fn keys(report: &recite_config::ProjectDiscoveryReport) -> Vec<&str> {
    report
        .catalogs()
        .iter()
        .map(|catalog| catalog.key().as_str())
        .collect()
}

#[test]
fn catalogues_are_opt_in() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", "format_version = 1\n");
    write(temp.path(), "locales/fr.po", "");

    let report = discover_project(temp.path()).expect("project discovery");
    assert!(report.catalogs().is_empty());
    assert!(report.manifest().catalogs().is_empty());
}

#[test]
fn declared_globs_select_sorted_unique_po_files() {
    let temp = project(r#"["locales/**/*.po", "./locales/fr.po", "extra/de.po"]"#);
    write(temp.path(), "locales/fr.po", "");
    write(temp.path(), "locales/nested/es.po", "");
    write(temp.path(), "locales/notes.txt", "");
    write(temp.path(), "locales/template.pot", "");
    write(temp.path(), "locales/skip/it.po", "");
    write(temp.path(), "locales/.hidden/pt.po", "");
    write(temp.path(), "locales/generated/ja.po", "");
    write(temp.path(), "other/ko.po", "");

    let report = discover_project(temp.path()).expect("project discovery");
    assert_eq!(keys(&report), ["locales/fr.po", "locales/nested/es.po"]);
    assert_eq!(
        report.manifest().catalogs(),
        ["locales/**/*.po", "locales/fr.po", "extra/de.po"]
    );
    assert!(report.is_complete());
    assert!(
        report.catalogs()[0]
            .path()
            .ends_with(std::path::Path::new("locales/fr.po"))
    );

    // Re-enumeration sees catalogues added after discovery.
    write(temp.path(), "locales/de.po", "");
    let (catalogs, diagnostics) = report.manifest().discover_catalogs();
    assert!(diagnostics.is_empty());
    assert_eq!(
        catalogs
            .iter()
            .map(|catalog| catalog.key().as_str())
            .collect::<Vec<_>>(),
        ["locales/de.po", "locales/fr.po", "locales/nested/es.po"]
    );
}

#[test]
fn invalid_catalogue_patterns_fail_discovery() {
    for pattern in [r#"["../fr.po"]"#, r#"["/abs/fr.po"]"#, r#"["locales/*"]"#] {
        let temp = project(pattern);
        let error = discover_project(temp.path()).expect_err("invalid catalogue pattern");
        assert!(
            matches!(error, ProjectDiscoveryError::InvalidCatalog { .. }),
            "{pattern}: {error:?}"
        );
        assert_eq!(error.diagnostic().as_str(), "RECITE_CONFIG118");
    }
}

#[cfg(unix)]
#[test]
fn catalogue_symlinks_must_stay_inside_the_project() {
    let outside = TempDir::new().expect("outside tempdir");
    write(outside.path(), "fr.po", "");
    let temp = project(r#"["locales/*.po"]"#);
    fs::create_dir_all(temp.path().join("locales")).expect("locales");
    symlink(
        outside.path().join("fr.po"),
        temp.path().join("locales/fr.po"),
    )
    .expect("symlink");

    let report = discover_project(temp.path()).expect("project discovery");
    assert!(report.catalogs().is_empty());
    assert!(!report.is_complete());
}
