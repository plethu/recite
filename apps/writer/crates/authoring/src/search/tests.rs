use super::*;
use recite_compiler::authoring::CancellationToken;
use recite_core::DocumentKey;
use std::path::{Path, PathBuf};

fn fixture_paths(root: &Path, paths: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            fixture_paths(&path, paths);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "recite")
        {
            paths.push(path);
        }
    }
}
fn assert_whole_file_parity(source: &str) {
    let document = SavedDocument::new(DocumentKey::new("scene.recite").unwrap(), source);
    let lowered = recite_parser::parse("scene.recite", source).lower_source_file();
    let mut expected = RegionIndex::default();
    for block in &lowered.source_file.blocks {
        for root in &block.statements {
            expected.add_root(
                &mut super::region::SearchContext::new(&document, block),
                root,
            );
        }
    }
    for index in [
        SearchIndex::build(std::slice::from_ref(&document)),
        SearchIndex::build_with_control(std::slice::from_ref(&document), &CancellationToken::new())
            .unwrap(),
    ] {
        let actual = &index.documents[0].1;
        let mut hits = Vec::new();
        let mut words: std::collections::BTreeMap<String, Vec<usize>> = Default::default();
        for shard in &actual.shards {
            let region = &shard.index;
            let offset = hits.len();
            hits.extend(region.hits.iter().cloned());
            for (word, ids) in &region.words {
                words
                    .entry(word.clone())
                    .or_default()
                    .extend(ids.iter().map(|id| id + offset));
            }
        }
        assert_eq!(hits, expected.hits, "source: {source:?}");
        assert_eq!(words, expected.words, "source: {source:?}");
    }
}
#[test]
fn regional_search_matches_whole_file_lowering_across_fixtures_and_incomplete_sources() {
    let mut paths = Vec::new();
    fixture_paths(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../fixtures/recite"),
        &mut paths,
    );
    paths.sort();
    assert!(!paths.is_empty());
    for path in paths {
        let source = std::fs::read_to_string(path).unwrap();
        assert_whole_file_parity(&source);
        assert_whole_file_parity(&source.replace("\r\n", "\n").replace('\n', "\r\n"));
        assert_whole_file_parity(&source.replace("\r\n", "\n").replace('\n', "\r"));
        let mut middle = source.len() / 2;
        while !source.is_char_boundary(middle) {
            middle -= 1;
        }
        assert_whole_file_parity(&source[..middle]);
    }
    for source in [
        "::\n> orphan\n  Café 🦊.\n:: good\n> line\n  Current.\n",
        ":: start\n  :: indented\n  > nested\n    Nested.\n:: next\n> line\n  Next.\n",
        ":: start\n! immediate call(\n:: next\n? reply\n  Reply.\n  -> END\n",
    ] {
        assert_whole_file_parity(source);
    }
    // One enormous block cannot benefit from region partitioning, but must
    // retain exactly the same indexing and nested statement order.
    let mut single = String::from(":: giant default\n");
    for n in 0..10_000 {
        single.push_str(&format!("> line_{n}@{n:020x}\n  Café courier {n}.\n"));
    }
    single.push_str("-> END\n");
    assert_eq!(recite_parser::source_regions(&single).len(), 1);
    assert_whole_file_parity(&single);
}

#[test]
fn replacements_reuse_identical_regions_and_retained_results_remain_valid() {
    let original = ":: first default\n> a\n  First courier.\n:: middle\n> b\n  Middle courier.\n:: last\n> c\n  Last courier.\n";
    let key = DocumentKey::new("scene.recite").unwrap();
    for changed in [
        original.replace("Middle", "Changed"),
        original.replace(
            ":: middle",
            ":: inserted\n> x\n  Inserted courier.\n:: middle",
        ),
        original.replace(":: middle\n> b\n  Middle courier.\n", ""),
        original.replace(
            "  Middle courier.",
            "  Middle courier.\n:: split\n> x\n  Split.",
        ),
        original.replace(":: middle", "  :: middle"),
        original.replace(":: middle", ":: first"),
        original.replace(":: middle", "::"),
        original.replace(":: middle", ":: first\n:: first"),
        original.replace("\n", "\r\n"),
        ":: giant default\n> a\n  One courier.\n".into(),
    ] {
        let input = SavedDocument::new(key.clone(), original);
        let mut index = SearchIndex::build(&[input]);
        let retained = index.search_results("courier", 100);
        let next = SavedDocument::new(key.clone(), changed.clone());
        index.replace_document(next.clone());
        let fresh = SearchIndex::build(&[next]);
        for query in [
            "courier",
            "courier first",
            "middle",
            "changed",
            "absent",
            "last",
        ] {
            for limit in [0, 1, 100] {
                assert_eq!(index.search(query, limit), fresh.search(query, limit));
            }
        }
        assert_eq!(retained.total(), 3);
        assert_eq!(retained.get(1).unwrap().text, "Middle courier.");
        assert_whole_file_parity(&changed);
    }
}

#[test]
fn one_changed_group_reuses_prefix_and_shifted_suffix_shards() {
    let original = (0..96)
        .map(|n| format!(":: block_{n}\n> line_{n}\n  Courier {n}.\n"))
        .collect::<String>();
    let key = DocumentKey::new("scene.recite").unwrap();
    for middle in [
        ":: inserted\n> x\n  Inserted.\n:: block_40",
        "  :: block_40",
        ":: block_40\n! bad(",
        ":: duplicate",
    ] {
        let mut index = SearchIndex::build(&[SavedDocument::new(key.clone(), original.clone())]);
        let old = index.documents[0].1.clone();
        let next = SavedDocument::new(key.clone(), original.replace(":: block_40", middle));
        index.replace_document(next.clone());
        let shards = &index.documents[0].1.shards;
        assert!(Arc::ptr_eq(&old.shards[0].index, &shards[0].index));
        assert!(Arc::ptr_eq(
            &old.shards[2].index,
            &shards.last().unwrap().index
        ));
        let fresh = SearchIndex::build(&[next]);
        assert_eq!(index.search("courier", 1000), fresh.search("courier", 1000));
    }
}

#[test]
fn repeated_boundary_inserts_keep_shards_packed() {
    let mut source = (0..96)
        .map(|n| format!(":: block_{n}\n> line_{n}\n  Courier {n}.\n"))
        .collect::<String>();
    let key = DocumentKey::new("scene.recite").unwrap();
    let mut index = SearchIndex::build(&[SavedDocument::new(key.clone(), &source)]);
    for n in 0..200 {
        source = source.replace(
            ":: block_40",
            &format!(":: inserted_{n}\n> new_{n}\n  Inserted courier.\n:: block_40"),
        );
        index.replace_document(SavedDocument::new(key.clone(), &source));
        let shards = &index.documents[0].1.shards;
        assert!(shards.iter().all(|shard| (16..=32).contains(&shard.count)));
        assert_eq!(index.len(), 96 + n + 1);
    }
    for n in (0..200).rev() {
        source = source.replace(
            &format!(":: inserted_{n}\n> new_{n}\n  Inserted courier.\n"),
            "",
        );
        index.replace_document(SavedDocument::new(key.clone(), &source));
        assert!(
            index.documents[0]
                .1
                .shards
                .iter()
                .all(|shard| (16..=32).contains(&shard.count))
        );
        assert_eq!(index.len(), 96 + n);
    }
    let fresh = SearchIndex::build(&[SavedDocument::new(key, &source)]);
    assert_eq!(index.search("courier", 1000), fresh.search("courier", 1000));
}

#[test]
fn initial_shards_are_balanced_even_for_large_prime_region_counts() {
    for count in [1, 17, 31, 33, 65, 1025] {
        let source = (0..count)
            .map(|n| format!(":: block_{n}\n> line_{n}\n  Text.\n"))
            .collect::<String>();
        let index = SearchIndex::build(&[SavedDocument::new(
            DocumentKey::new("scene.recite").unwrap(),
            source,
        )]);
        let shards = &index.documents[0].1.shards;
        assert_eq!(shards.iter().map(|s| s.count).sum::<usize>(), count);
        assert!(
            shards
                .iter()
                .all(|s| ((if count < 16 { 1 } else { 16 })..=32).contains(&s.count))
        );
    }
}
