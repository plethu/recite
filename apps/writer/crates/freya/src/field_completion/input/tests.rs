use freya::code_editor::Rope;
use recite_core::SourcePosition;
use recite_writer_model::{Document, EditError};

#[test]
fn utf16_completion_ranges_match_exact_unicode_prefixes_in_multichunk_ropes()
-> Result<(), Box<dyn std::error::Error>> {
    for newline in ["\n", "\r\n"] {
        for paragraphs in [1, 300] {
            let source = format!(
                ":: destination default\n> line@11111111111111111111\n{}-> dest",
                "  Café 🐧 שלום.\n".repeat(paragraphs)
            )
            .replace('\n', newline);
            let doc = Document::new(&source)?;
            let completions = doc.complete_draft(
                &source,
                SourcePosition::new((paragraphs + 3).try_into()?, 8)?,
            )?;
            let rope = Rope::from_str(&source);
            if paragraphs > 1 {
                assert!(rope.chunks().count() > 1);
            }
            let index = completions
                .candidates()
                .iter()
                .position(|c| c.name() == "destination")
                .ok_or("destination")?;
            let (bytes, name) = completions.replacement(&source, index)?;
            let (utf16, replacement) = super::utf16_replacement(&rope, &completions, index)?;
            assert_eq!(
                utf16,
                source[..bytes.start].encode_utf16().count()
                    ..source[..bytes.end].encode_utf16().count()
            );
            assert_eq!(replacement, name);
            let stale = Rope::from_str(&source.replacen("Café", "Cafè", 1));
            assert!(matches!(
                super::utf16_replacement(&stale, &completions, usize::MAX),
                Err(EditError::Stale)
            ));
        }
    }
    Ok(())
}
