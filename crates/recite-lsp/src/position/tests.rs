use lsp_types::{Position, Range};
use recite_compiler::authoring::{AuthoringKernel, AuthoringRequest, SavedDocument};
use recite_core::{DocumentKey, SourcePosition, SourceSpan};

use super::{document_span_to_range, span_to_range};

#[test]
fn indexed_symbol_ranges_preserve_unicode_and_inclusive_ends()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start\r\n💬café\r\n";
    let mut kernel = AuthoringKernel::new();
    kernel.apply(AuthoringRequest::new(
        kernel.snapshot().generation(),
        [SavedDocument::new(
            DocumentKey::new("source.recite")?,
            source,
        )],
        [],
    ))?;
    let document = &kernel.snapshot().documents()[0];
    let span = SourceSpan::new(
        "source.recite",
        SourcePosition::new(2, 2)?,
        Some(SourcePosition::new(2, 5)?),
    );
    assert_eq!(
        document_span_to_range(document, &span),
        Range::new(Position::new(1, 2), Position::new(1, 6)),
    );
    Ok(())
}

#[test]
fn raw_and_indexed_symbol_ranges_share_line_and_column_clamping()
-> Result<(), Box<dyn std::error::Error>> {
    for source in [
        "",
        "\n",
        "\r",
        ":: start\n",
        ":: start\r\n",
        ":: start\r💬café\rnext",
    ] {
        let mut kernel = AuthoringKernel::new();
        kernel.apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            [SavedDocument::new(
                DocumentKey::new("source.recite")?,
                source,
            )],
            [],
        ))?;
        let document = &kernel.snapshot().documents()[0];
        for line in [1, 2, 3, 100] {
            for column in [1, 2, 5, 100] {
                let start = SourcePosition::new(line, column)?;
                for end in [None, Some(start), Some(SourcePosition::new(100, 100)?)] {
                    let span = SourceSpan::new("source.recite", start, end);
                    assert_eq!(
                        document_span_to_range(document, &span),
                        span_to_range(source, &span),
                        "source {source:?}, span {span:?}",
                    );
                }
            }
        }
    }
    Ok(())
}
