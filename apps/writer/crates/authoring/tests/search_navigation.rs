use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::{SearchIndex, passage_excerpt};

#[test]
fn saved_hits_identify_distinct_passages_and_retain_their_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let key = DocumentKey::new("scene.recite")?;
    let source = ":: start default\n> first@11111111111111111111\n  Courier waits.\n> second@22222222222222222222\n  Courier leaves.\n? reply@33333333333333333333\n  Courier?\n  -> END\n";
    let mut index = SearchIndex::build(&[SavedDocument::new(key.clone(), source)]);
    let results = index.search_results("courier", 10);
    let ids = (0..results.len())
        .map(|n| {
            results
                .get(n)
                .and_then(|hit| hit.passage.as_ref())
                .map(|id| id.as_str())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            Some("11111111111111111111"),
            Some("22222222222222222222"),
            Some("33333333333333333333")
        ]
    );
    index.replace_document(SavedDocument::new(
        key,
        source.replace("Courier", "Zeppelin"),
    ));
    assert_eq!(index.search_results("courier", 10).total(), 0);
    assert_eq!(
        results
            .get(1)
            .ok_or("second hit")?
            .passage
            .as_ref()
            .map(|id| id.as_str()),
        Some("22222222222222222222")
    );
    Ok(())
}

#[test]
fn unfrozen_passages_remain_searchable_without_inventing_navigation_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let index = SearchIndex::build(&[SavedDocument::new(
        DocumentKey::new("scene.recite")?,
        ":: start\n> draft@\n  Courier.\n",
    )]);
    let results = index.search_results("courier", 10);
    assert_eq!(results.total(), 1);
    assert!(results.get(0).ok_or("draft hit")?.passage.is_none());
    Ok(())
}

#[test]
fn snippets_show_late_unicode_matches_with_bounded_output() -> Result<(), Box<dyn std::error::Error>>
{
    let text = format!(
        "{}CAFÉ_東京 arrives with the courier. {}",
        "A quiet hallway. ".repeat(500),
        "🦊 ".repeat(500)
    );
    let source = format!(":: start default\n> line@11111111111111111111\n  {text}\n-> END\n");
    let index = SearchIndex::build(&[SavedDocument::new(
        DocumentKey::new("scene.recite")?,
        source,
    )]);
    let results = index.search_results("café_東京", 1);
    let hit = results.get(0).ok_or("Unicode hit")?;
    for limit in [40, 90, 120] {
        let excerpt = hit.excerpt("café_東京", limit);
        assert!(excerpt.contains("CAFÉ_東京"), "{excerpt}");
        assert!(excerpt.starts_with("… "));
        assert!(excerpt.ends_with('…'));
        assert!(excerpt.chars().count() <= limit);
    }
    assert!(hit.excerpt("scene", 90).starts_with("A quiet hallway."));
    assert_eq!(hit.excerpt("café_東京", 0), "");
    assert_eq!(hit.excerpt("café_東京", 2), "A ");
    Ok(())
}

#[test]
fn passage_excerpts_use_substrings_and_keep_original_unicode() {
    let text = format!(
        "{}İstanbul's Courier’s packet arrives. {}",
        "A quiet hallway. ".repeat(500),
        "A quiet ending. ".repeat(50)
    );
    for (query, expected) in [
        ("cour", "Courier’s"),
        ("cour pack", "Courier’s"),
        ("courier’s", "Courier’s"),
        ("İST", "İstanbul's"),
        ("packet", "packet"),
    ] {
        let excerpt = passage_excerpt(&text, query, 90);
        assert!(excerpt.contains(expected), "{query}: {excerpt}");
        assert!(excerpt.starts_with("… "));
        assert!(excerpt.chars().count() <= 90);
    }
    let token = format!("{}İCourier’spacket", "x".repeat(20_000));
    let excerpt = passage_excerpt(&token, "cour", 90);
    assert!(excerpt.contains("İCourier’spacket"));
    assert!(excerpt.chars().count() <= 90);
}
