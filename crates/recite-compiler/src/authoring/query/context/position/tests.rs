use super::*;
use recite_core::source_lines;

#[test]
fn indexed_lines_and_local_spans_match_prefix_scanning() {
    let key = DocumentKey::new("spans.recite").expect("key");
    for text in [
        "",
        "\n",
        "\r\n\r\n",
        "أهلاً😀\r:: beat default\n  ! effect(α)\r\n-> target\n",
        "x\r\r\n\n? item requires=gate reason=locked",
        "first\n  > line speaker=mara\n\t😀aé\nlast",
    ] {
        let index = SourceLineIndex::new(text);
        let mut offset = 0;
        for (number, (content, terminator)) in source_lines(text).enumerate() {
            let number = u32::try_from(number + 1).expect("line number");
            let line = line_at(&index, number).expect("indexed line");
            assert_eq!(line.text, content);
            let boundaries = content
                .char_indices()
                .map(|(byte, _)| byte)
                .chain([content.len()])
                .collect::<Vec<_>>();
            for &start in &boundaries {
                for &end in boundaries.iter().filter(|end| **end >= start) {
                    let global_start = offset + start;
                    let exclusive_end = offset + end;
                    let inclusive_end = if end == start {
                        global_start
                    } else {
                        text[..exclusive_end]
                            .char_indices()
                            .next_back()
                            .expect("scalar")
                            .0
                    };
                    let expected = SourceSpan::new(
                        key.as_str(),
                        scanned_position(text, global_start),
                        Some(scanned_position(text, inclusive_end)),
                    );
                    assert_eq!(span(&key, line, start, end), expected);
                }
            }
            assert_eq!(byte_at_column(content, u32::MAX), content.len());
            offset += content.len() + terminator.len();
        }
        assert!(line_at(&index, 0).is_none());
        assert!(line_at(&index, u32::MAX).is_none());
    }
}

fn scanned_position(text: &str, offset: usize) -> SourcePosition {
    let mut line = 1;
    let mut column = 1;
    for (byte, character) in text[..offset].char_indices() {
        if character == '\n' || (character == '\r' && text.as_bytes().get(byte + 1) != Some(&b'\n'))
        {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    SourcePosition::new(line, column).expect("position")
}
