use super::*;
use crate::localisation::{catalogue::Draft, status::TranslationStatus};
use criterion::measurement::{Measurement, WallTime};

// Frozen filtering behavior supplies a differential oracle, including caption lookup.
fn previous_matches(
    catalogue: &Catalogue,
    passages: &[Passage],
    query: &str,
    attention: bool,
) -> Vec<PoEntryId> {
    catalogue
        .document
        .entries()
        .iter()
        .filter(|entry| {
            if entry.is_header() || entry.is_obsolete() {
                return false;
            }
            let draft = catalogue.draft(entry.id());
            (!attention || TranslationStatus::for_entry(catalogue, entry.id()).needs_attention())
                && (entry.source_text().to_lowercase().contains(query)
                    || entry
                        .context()
                        .is_some_and(|id| id.to_lowercase().contains(query))
                    || Destination::resolve(entry, passages).is_some_and(|d| {
                        d.caption()
                            .to_lowercase()
                            .contains(&query.replace('_', " "))
                    })
                    || draft.as_ref().is_some_and(|d| {
                        d.forms
                            .iter()
                            .any(|text| text.to_lowercase().contains(query))
                    }))
        })
        .map(|entry| entry.id())
        .collect()
}

#[test]
fn filtering_preserves_counts_order_attention_and_current_scene_caption_precedence()
-> Result<(), Box<dyn std::error::Error>> {
    let document =
        recite_writer_model::Document::new(recite_writer_model::workload::source(3, 0, 1))?;
    let passages = document.passage_snapshot()?;
    let mut source = String::from(
        "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
    );
    for passage in passages.iter() {
        source.push_str(&format!("#. file: elsewhere.recite\n#. block: wrong_block\nmsgctxt {:?}\nmsgid {:?}\nmsgstr {:?}\n\n",passage.id,passage.text,passage.text));
    }
    source.push_str("#, fuzzy\nmsgctxt \"33333333333333333333&formal\"\nmsgid \"{count} café\"\nmsgid_plural \"{count} cafés\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n\n#~ msgctxt \"44444444444444444444\"\n#~ msgid \"gone\"\n#~ msgstr \"gone\"\n");
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, &source)?;
    let mut catalogue = Catalogue::open(&path)?;
    let first = catalogue.document.entries()[1].id();
    catalogue.update(
        first,
        Draft {
            forms: vec!["Nouveau café".into()],
            reviewed: true,
        },
    );
    let index = QueueIndex::new(&catalogue, &passages);
    for attention in [false, true] {
        for query in [
            "",
            "café",
            "nouveau",
            "wrong block",
            "elsewhere",
            "beat",
            "formal",
            "absent",
            "gone",
        ] {
            assert_eq!(
                index.matching_entries(&catalogue, query, attention),
                previous_matches(&catalogue, &passages, query, attention),
                "{query}/{attention}"
            );
        }
    }
    assert!(
        index
            .matching_entries(&catalogue, "wrong block", false)
            .is_empty()
    );
    assert!(
        QueueIndex::new(&catalogue, &[])
            .matching_entries(&catalogue, "wrong block", false)
            .contains(&first)
    );
    let revision = catalogue.revision();
    catalogue.discard(first);
    assert!(!std::sync::Arc::ptr_eq(&revision, &catalogue.revision()));
    assert!(
        index
            .matching_entries(&catalogue, "nouveau", false)
            .is_empty()
    );
    catalogue.reload()?;
    assert_eq!(
        QueueIndex::new(&catalogue, &passages)
            .matching_entries(&catalogue, "", false)
            .len(),
        passages.len() + 1
    );
    Ok(())
}

#[test]
#[ignore = "opt-in query profiling with a populated current scene"]
fn catalogue_current_scene_filter_timing() -> Result<(), Box<dyn std::error::Error>> {
    let entries = std::env::var("RECITE_BENCH_ENTRIES")
        .ok()
        .map(|n| n.parse::<usize>())
        .transpose()?
        .unwrap_or(10000);
    let document =
        recite_writer_model::Document::new(recite_writer_model::workload::source(500, 0, 20))?;
    let passages = document.passage_snapshot()?;
    let mut source = String::new();
    for i in 0..entries {
        source.push_str(&format!("#. file: chapter.recite\n#. block: chapter_start\nmsgctxt \"{i:020}\"\nmsgid \"Hello {{name}} café {i}\"\nmsgstr \"Bonjour {{name}} scène {i}\"\n\n"));
    }
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, source)?;
    let catalogue = Catalogue::open(&path)?;
    let index_started = WallTime.start();
    let index = QueueIndex::new(&catalogue, &passages);
    let index_ms = WallTime.end(index_started).as_secs_f64() * 1000.;
    let mut samples = Vec::new();
    for query in ["", "chapter start", "absent", "bonjour"] {
        for round in 0..3 {
            for candidate in if round % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let start = WallTime.start();
                let matches = if candidate {
                    index.matching_entries(&catalogue, query, false)
                } else {
                    previous_matches(&catalogue, &passages, query, false)
                };
                let elapsed = WallTime.end(start).as_secs_f64() * 1000.;
                assert_eq!(
                    matches,
                    previous_matches(&catalogue, &passages, query, false)
                );
                samples.push(serde_json::json!({"query":query,"candidate":candidate,"ms":elapsed,"count":matches.len()}));
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"entries":entries,"current_passages":passages.len(),"index_ms":index_ms,"samples":samples})
    );
    Ok(())
}

#[test]
fn prepared_index_replaces_saved_words_and_follows_partial_save_and_reordered_reload()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(
        &path,
        "msgctxt \"11111111111111111111\"\nmsgid \"İstanbul\"\nmsgstr \"Ancien\"\n\nmsgctxt \"22222222222222222222\"\nmsgid \"Second\"\nmsgstr \"Old second\"\n",
    )?;
    let mut catalogue = Catalogue::open(&path)?;
    let first = PoEntryId::new(0);
    let second = PoEntryId::new(1);
    let index = QueueIndex::new(&catalogue, &[]);
    assert_eq!(
        index.matching_entries(&catalogue, "i\u{307}stanbul", false),
        [first]
    );
    assert!(index.matching_entries(&catalogue, "", true).is_empty());
    catalogue.update(
        first,
        Draft {
            forms: vec!["Nouveau".into()],
            reviewed: true,
        },
    );
    assert!(
        index
            .matching_entries(&catalogue, "ancien", false)
            .is_empty()
    );
    assert_eq!(index.matching_entries(&catalogue, "nouveau", true), [first]);
    catalogue.update(
        second,
        Draft {
            forms: vec!["Other".into()],
            reviewed: false,
        },
    );
    let saved_identity = catalogue.saved_source();
    catalogue.save(first)?;
    assert!(!std::sync::Arc::ptr_eq(
        &saved_identity,
        &catalogue.saved_source()
    ));
    let index = QueueIndex::new(&catalogue, &[]);
    assert_eq!(
        index.matching_entries(&catalogue, "nouveau", false),
        [first]
    );
    assert!(
        index
            .matching_entries(&catalogue, "old second", false)
            .is_empty()
    );
    catalogue.discard(second);
    assert_eq!(
        index.matching_entries(&catalogue, "old second", false),
        [second]
    );
    std::fs::write(
        &path,
        "msgctxt \"22222222222222222222\"\nmsgid \"Second\"\nmsgstr \"Unique\"\n\nmsgctxt \"11111111111111111111\"\nmsgid \"İstanbul\"\nmsgstr \"Nouveau\"\n",
    )?;
    catalogue.reload()?;
    let index = QueueIndex::new(&catalogue, &[]);
    assert_eq!(
        index.matching_entries(&catalogue, "unique", false),
        [PoEntryId::new(0)]
    );
    assert_eq!(
        index.matching_entries(&catalogue, "nouveau", false),
        [PoEntryId::new(1)]
    );
    Ok(())
}

#[test]
fn duplicate_passage_keys_keep_first_caption_and_scene_replacement_updates_it()
-> Result<(), Box<dyn std::error::Error>> {
    let document =
        recite_writer_model::Document::new(recite_writer_model::workload::source(1, 0, 1))?;
    let passages = document.passage_snapshot()?;
    let mut first = passages[0].clone();
    first.section = "preferred_caption".into();
    let mut duplicate = first.clone();
    duplicate.section = "other_caption".into();
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(
        &path,
        format!(
            "msgctxt {:?}\nmsgid {:?}\nmsgstr {:?}\n",
            first.id, first.text, first.text
        ),
    )?;
    let catalogue = Catalogue::open(&path)?;
    let index = QueueIndex::new(&catalogue, &[first.clone(), duplicate.clone()]);
    assert_eq!(
        index.matching_entries(&catalogue, "preferred caption", false),
        [PoEntryId::new(0)]
    );
    assert!(
        index
            .matching_entries(&catalogue, "other caption", false)
            .is_empty()
    );
    first.section = "replacement_caption".into();
    let index = QueueIndex::new(&catalogue, &[first, duplicate]);
    assert_eq!(
        index.matching_entries(&catalogue, "replacement_caption", false),
        [PoEntryId::new(0)]
    );
    assert!(
        index
            .matching_entries(&catalogue, "preferred caption", false)
            .is_empty()
    );
    Ok(())
}
