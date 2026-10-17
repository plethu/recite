use super::*;
use crate::recovery::{RecoveryDisk, write_snapshot};
use std::io;

fn snapshot(baseline: &str, source: &str, view: View, draft: &str) -> Recovery {
    Recovery {
        version: 1,
        baseline: baseline.into(),
        draft: serde_json::from_value(serde_json::json!({
            "source": source,
            "view": view,
            "draft": draft,
        }))
        .unwrap(),
    }
}

#[derive(Default)]
struct Writes {
    bytes: Vec<u8>,
    calls: usize,
}
impl Write for Writes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn check(encoding: &mut Encoding, recovery: &Recovery) -> Writes {
    let mut output = Writes::default();
    write_snapshot(&mut output, recovery, encoding).unwrap();
    assert_eq!(output.bytes, serde_json::to_vec(recovery).unwrap());
    assert_eq!(
        serde_json::from_slice::<Recovery>(&output.bytes).unwrap(),
        *recovery
    );
    output
}

#[test]
fn rope_backed_draft_streams_through_the_durable_codec() {
    let mut model = recite_writer_model::Workbench::new(recite_writer_model::FIXTURE).unwrap();
    model.select(View::Source).unwrap();
    let original = format!("{}# \"é💬\"\\\r\n", recite_writer_model::FIXTURE);
    let mut rope = freya::code_editor::Rope::from_str(&original);
    model.set_draft_snapshot(DraftSnapshot::from_rope(rope.clone()));
    let recovery = Recovery {
        version: 1,
        baseline: model.document().source_snapshot(),
        draft: model.recovery(),
    };
    rope.insert(0, "# later editor state\n");
    let mut encoding = Encoding::default();
    let output = check(&mut encoding, &recovery);
    let wire: serde_json::Value = serde_json::from_slice(&output.bytes).unwrap();
    assert_eq!(wire["draft"]["draft"], original);
    assert!(
        recovery
            .draft
            .draft_snapshot()
            .matches_rope(&freya::code_editor::Rope::from_str(&original))
    );

    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("scene.recite");
    let mut disk = RecoveryDisk::<Recovery>::open(&source).unwrap();
    disk.persist(Some(recovery.clone())).unwrap();
    drop(disk);
    assert_eq!(
        RecoveryDisk::open(&source).unwrap().snapshot(),
        Some(&recovery)
    );
}

#[test]
fn immutable_sources_share_encoding_and_warm_writes_are_bounded() {
    let source = "escaped \"é💬\\\n".repeat(100_000);
    let mut encoding = Encoding::default();
    let initial = snapshot(&source, &source, View::Source, "first draft");
    check(&mut encoding, &initial);
    let sources = encoding.sources.as_ref().unwrap();
    assert!(!Arc::ptr_eq(&initial.baseline, &sources.applied.source));
    assert!(Arc::ptr_eq(&sources.baseline.json, &sources.applied.json));
    let escaped = Arc::clone(&sources.baseline.json);
    for index in 0..10 {
        let recovery = Recovery {
            baseline: Arc::clone(&initial.baseline),
            ..snapshot(
                &source,
                &source,
                View::Source,
                &format!("changed draft {index}"),
            )
        };
        let output = check(&mut encoding, &recovery);
        assert!(
            output.calls <= 8,
            "{} writes for {} bytes",
            output.calls,
            output.bytes.len()
        );
        let sources = encoding.sources.as_ref().unwrap();
        assert!(Arc::ptr_eq(&sources.baseline.json, &escaped));
        assert!(Arc::ptr_eq(&sources.applied.json, &escaped));
    }
}

#[test]
fn changed_sources_views_and_equal_replacements_keep_wire_identity_and_drop_old_encoding() {
    let mut encoding = Encoding::default();
    let initial = snapshot("old\nsource", "old\nsource", View::Source, "draft");
    check(&mut encoding, &initial);
    let obsolete = Arc::downgrade(&encoding.sources.as_ref().unwrap().baseline.json);
    for view in [
        View::Source,
        View::Passage("first".into()),
        View::Block("root".into()),
    ] {
        check(
            &mut encoding,
            &snapshot("new baseline", "applied 🦊\nsource", view, "draft\"\\\r\n"),
        );
        assert!(obsolete.upgrade().is_none());
        let sources = encoding.sources.as_ref().unwrap();
        assert!(!Arc::ptr_eq(&sources.baseline.json, &sources.applied.json));
        assert_eq!(sources.baseline.source.as_ref(), "new baseline");
        assert_eq!(sources.applied.source.as_ref(), "applied 🦊\nsource");
    }
    check(&mut encoding, &initial);
    assert!(Arc::ptr_eq(
        &encoding.sources.as_ref().unwrap().baseline.json,
        &encoding.sources.as_ref().unwrap().applied.json
    ));
}

#[test]
fn failed_writes_retry_with_matching_cached_keys_and_clear_releases_encoding() {
    struct Failed;
    impl Write for Failed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("write failed"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush failed"))
        }
    }
    let mut encoding = Encoding::default();
    let first = snapshot("before", "before", View::Source, "first draft");
    check(&mut encoding, &first);
    let second = snapshot(
        "after",
        "applied",
        View::Passage("line".into()),
        "second draft",
    );
    assert!(write_snapshot(Failed, &second, &mut encoding).is_err());
    check(&mut encoding, &second);
    check(&mut encoding, &first);

    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("scene.recite");
    let mut disk = RecoveryDisk::<Recovery>::open(&source).unwrap();
    assert!(write_snapshot(Failed, &second, &mut disk.encoding).is_err());
    disk.persist(None).unwrap();
    assert!(disk.encoding.sources.is_none());
    disk.persist(Some(second.clone())).unwrap();
    let escaped = Arc::downgrade(&disk.encoding.sources.as_ref().unwrap().baseline.json);
    disk.persist(None).unwrap();
    assert!(disk.encoding.sources.is_none());
    assert!(escaped.upgrade().is_none());
    assert!(!disk.path.exists());
    disk.persist(Some(second.clone())).unwrap();
    drop(disk);
    assert_eq!(
        RecoveryDisk::open(&source).unwrap().snapshot(),
        Some(&second)
    );
}

#[test]
#[ignore = "serialization timing; use just writer bench-recovery"]
fn recovery_serialization_timing() {
    use criterion::measurement::{Measurement, WallTime};
    use std::hint::black_box;
    let source = std::env::var_os("RECITE_BENCH_SOURCE").map_or_else(
        || "escaped \"é💬\\\n".repeat(100_000),
        |path| std::fs::read_to_string(path).unwrap(),
    );
    let recovery = snapshot(&source, &source, View::Source, &format!("{source}x"));
    let mut encoding = Encoding::default();
    let started = WallTime.start();
    write_snapshot(io::sink(), &recovery, &mut encoding).unwrap();
    let cold_ms = WallTime.end(started).as_secs_f64() * 1000.;
    check(&mut encoding, &recovery);
    let mut normal_ms = Vec::new();
    let mut cached_ms = Vec::new();
    for _ in 0..15 {
        let started = WallTime.start();
        let mut output = io::BufWriter::new(io::sink());
        serde_json::to_writer(&mut output, black_box(&recovery)).unwrap();
        output.flush().unwrap();
        normal_ms.push(WallTime.end(started).as_secs_f64() * 1000.);
        let started = WallTime.start();
        write_snapshot(io::sink(), black_box(&recovery), &mut encoding).unwrap();
        cached_ms.push(WallTime.end(started).as_secs_f64() * 1000.);
    }
    let sources = encoding.sources.as_ref().unwrap();
    let retained = sources.baseline.json.get().len()
        + if Arc::ptr_eq(&sources.baseline.json, &sources.applied.json) {
            0
        } else {
            sources.applied.json.get().len()
        };
    println!(
        "{}",
        serde_json::json!({"scope":"Serialization through BufWriter to sink; exact bytes checked outside timers; excludes filesystem writes", "source_bytes":source.len(),"cold_snapshot_ms":cold_ms,"retained_escaped_bytes":retained,"normal_ms":normal_ms,"cached_ms":cached_ms})
    );
}
