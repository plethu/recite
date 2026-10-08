use super::{contains, symbol_at, symbol_locations};
use crate::authoring::{AuthoringKernel, AuthoringRequest, SavedDocument, SymbolQueryOptions};
use recite_core::{DocumentKey, SourcePosition};

#[test]
fn cursor_lookup_matches_full_symbol_order_at_every_fixture_position()
-> Result<(), Box<dyn std::error::Error>> {
    let key = DocumentKey::new("main.recite")?;
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/recite/valid/language_pressure.recite"
    ));
    let mut kernel = AuthoringKernel::new();
    kernel.apply(AuthoringRequest::new(
        kernel.snapshot().generation(),
        [SavedDocument::new(key.clone(), source)],
        [],
    ))?;
    let document = kernel.snapshot().document(&key).ok_or("missing document")?;
    let all = symbol_locations(&key, document, SymbolQueryOptions::default());
    assert!(!all.is_empty());
    for (line, text) in source.lines().enumerate() {
        for column in 0..=text.chars().count() {
            let position =
                SourcePosition::new(u32::try_from(line + 1)?, u32::try_from(column + 1)?)?;
            assert_eq!(
                symbol_at(&key, document, position).as_ref(),
                all.iter().find(|symbol| contains(symbol.span(), position)),
                "{position:?}"
            );
        }
    }
    Ok(())
}
