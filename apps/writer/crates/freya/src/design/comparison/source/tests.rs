use super::*;
#[test]
fn source_rows_align_insertions_and_preserve_unicode_crlf_and_final_newline() {
    let before = ":: old\r\n> line\r\n  À neuf.\r\n-> old\r\n";
    let after = "# inserted\r\n:: new\r\n> line\r\n  À neuf.\r\n-> new";
    let rows = ComparisonRow::source("scene.recite", before, after);
    assert_eq!(
        rows.iter()
            .filter_map(|r| r.before.as_deref())
            .collect::<String>(),
        before
    );
    assert_eq!(
        rows.iter()
            .filter_map(|r| r.after.as_deref())
            .collect::<String>(),
        after
    );
    assert_eq!(rows.iter().filter(|r| r.before != r.after).count(), 2);
    assert!(rows.iter().any(|r| r.before == r.after && r.before.as_ref().is_some_and(|s| s.contains("À neuf"))));
}
