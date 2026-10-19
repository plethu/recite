use super::*;
use recite_core::SourcePosition;

fn span(file: &str, line: u32, column: u32) -> SourceSpan {
    SourceSpan::point(file, SourcePosition::new(line, column).unwrap())
}

#[test]
fn large_report_reads_each_source_once_and_retains_only_requested_lines() {
    let source = (1..=20_000)
        .map(|line| format!("line {line}\n"))
        .collect::<String>();
    let spans = (0..500)
        .map(|index| span("large.recite", 19_999 + index % 2, index + 1))
        .collect::<Vec<_>>();
    let mut excerpts = Excerpts::new(spans.iter());
    let mut reads = 0;
    for span in &spans {
        let line = excerpts.load(span, |_| {
            reads += 1;
            Some(source.clone())
        });
        assert_eq!(line, Some(format!("line {}", span.start.line()).as_str()));
    }
    assert_eq!(reads, 1);
    let File::Loaded(lines) = &excerpts.files["large.recite"] else {
        panic!("not loaded")
    };
    assert_eq!(lines.len(), 2);
}

#[test]
fn interleaved_files_keep_line_endings_unicode_and_missing_reads_report_local() {
    let spans = [
        span("a", 1, 1),
        span("b", 2, 1),
        span("a", 3, 5),
        span("missing", 1, 1),
        span("a", 4, 1),
        span("missing", 8, 1),
        span("a", 2, 1),
        span("a", 5, 1),
    ];
    let expected = [
        Some("hello"),
        Some("second"),
        Some("\t世界"),
        None,
        Some(""),
        None,
        Some("middle"),
        None,
    ];
    let mut excerpts = Excerpts::new(spans.iter());
    let mut reads = BTreeMap::new();
    for (span, expected) in spans.iter().zip(expected) {
        assert_eq!(
            excerpts.load(span, |file| {
                *reads.entry(file.to_owned()).or_insert(0) += 1;
                match file {
                    "a" => Some("hello\r\nmiddle\r\t世界\n".into()),
                    "b" => Some("first\nsecond".into()),
                    _ => None,
                }
            }),
            expected
        );
    }
    assert_eq!(
        reads,
        BTreeMap::from([("a".into(), 1), ("b".into(), 1), ("missing".into(), 1)])
    );
}

#[test]
fn preparing_requests_does_not_read_files_before_their_diagnostic_is_rendered() {
    let first = span("first", 1, 1);
    let later = span("later", 1, 1);
    let mut excerpts = Excerpts::new([&first, &later].into_iter());
    assert_eq!(
        excerpts.load(&first, |_| Some("first".into())),
        Some("first")
    );
    assert!(matches!(excerpts.files["later"], File::Requested(_)));
}
