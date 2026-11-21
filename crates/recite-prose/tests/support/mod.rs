use std::fmt::Write as _;
use std::path::PathBuf;

use recite_core::project::ProjectProse;
use recite_prose::{ProseFinding, ProseSettings};

pub(crate) fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(crate) fn fixture(relative: &str) -> String {
    let path = workspace().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

/// Settings rooted at the prose fixture directory.
pub(crate) fn settings(configure: impl FnOnce(&mut ProjectProse)) -> ProseSettings {
    let mut prose = ProjectProse::default();
    configure(&mut prose);
    ProseSettings::from_project(&workspace().join("fixtures/recite/valid/prose"), &prose)
        .expect("valid prose settings")
}

pub(crate) fn render(source: &str, findings: &[ProseFinding]) -> String {
    let mut output = String::new();
    for finding in findings {
        let record = finding.diagnostic.record().expect("structured diagnostic");
        let _ = writeln!(
            output,
            "{} {:?} {}:{}-{} {:?} rule={} region={} id={}",
            record.code.as_str(),
            record.severity,
            record.span.start.line(),
            record.span.start.column(),
            record.span.end.map_or_else(String::new, |end| format!(
                "{}:{}",
                end.line(),
                end.column()
            )),
            finding.text,
            finding.rule,
            finding.region.as_str(),
            finding.stable_id.as_deref().unwrap_or("-"),
        );
        for fix in &finding.fixes {
            let _ = writeln!(
                output,
                "  fix {:?} -> {:?}",
                &source[fix.range.clone()],
                fix.replacement
            );
        }
    }
    output
}

pub(crate) fn assert_snapshot(name: &str, actual: &str) {
    let snapshot_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    insta::with_settings!({ snapshot_path => snapshot_path, prepend_module_to_snapshot => false }, {
        insta::assert_snapshot!(name, actual);
    });
}
