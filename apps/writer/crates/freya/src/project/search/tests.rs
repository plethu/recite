use std::{fs, sync::Arc};

#[test]
fn unchanged_and_schema_only_refresh_reuse_saved_search_with_dirty_overlays()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = super::super::tests::project()?;
    let mut workbench = files.workbench()?;
    let before = files.search_index();
    let old_results = before.search_results("tell", 10);
    let old_hits = (0..old_results.len())
        .map(|index| old_results.get(index).unwrap().clone())
        .collect::<Vec<_>>();
    assert!(!old_hits.is_empty());
    workbench.select(recite_writer_model::View::Source)?;
    workbench.set_draft(format!("{}\n# unsaved zeppelin", workbench.draft()));
    files.refresh(&mut workbench)?;
    assert!(Arc::ptr_eq(&before, &files.search_index()));
    fs::write(dir.path().join("schema.json"), "{\"schema_version\":1}")?;
    fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\nschema = \"schema.json\"\n",
    )?;
    files.refresh(&mut workbench)?;
    assert!(Arc::ptr_eq(&before, &files.search_index()));
    assert!(workbench.draft().contains("unsaved zeppelin"));
    assert_eq!(files.search_index().search("zeppelin", 10).0, 0);
    fs::write(
        &files.current,
        files.saved.replace("Would you tell me", "A saved zeppelin"),
    )?;
    files.refresh(&mut workbench)?;
    assert!(!Arc::ptr_eq(&before, &files.search_index()));
    assert_eq!(files.search_index().search("zeppelin", 10).0, 1);
    assert_eq!(before.search("zeppelin", 10).0, 0);
    assert_eq!(
        (0..old_results.len())
            .map(|index| old_results.get(index).unwrap().clone())
            .collect::<Vec<_>>(),
        old_hits
    );
    Ok(())
}

#[test]
fn search_input_identity_includes_document_order_and_membership()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, files) = super::super::tests::project()?;
    let mut documents = files.context.documents.clone();
    let extra = recite_compiler::authoring::SavedDocument::new(
        recite_core::DocumentKey::new("extra.recite")?,
        ":: extra\n> extra@44444444444444444444\n  Extra passage. Tell me.\n-> END\n",
    );
    documents.push(extra);
    let expanded = files.refreshed_search(&documents);
    assert!(!Arc::ptr_eq(&files.search, &expanded));
    assert_eq!(expanded.search("extra", 10).0, 1);
    documents.reverse();
    let reordered = files.refreshed_search(&documents);
    assert_eq!(
        reordered
            .search_results("tell", 10)
            .get(0)
            .ok_or("hit")?
            .document
            .as_ref(),
        "extra.recite"
    );
    documents.clear();
    assert!(files.refreshed_search(&documents).is_empty());
    Ok(())
}

#[test]
fn sparse_refresh_retains_unchanged_shards_and_saved_result_snapshots()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = super::super::tests::project()?;
    let documents = recite_writer_model::workload::project(10_000, 500)?;
    files.context.documents = documents.clone();
    files.search = Arc::new(recite_writer_model::SearchIndex::build(&documents));
    let before = files.search.search_results("courier", 10_000);
    let mut changed = documents.clone();
    for index in [4, 18] {
        changed[index] = recite_compiler::authoring::SavedDocument::new(
            documents[index].key().clone(),
            documents[index].text().replace("courier", "zeppelin"),
        );
    }
    let refreshed = files.refreshed_search(&changed);
    let after = refreshed.search_results("mara", 10_000);
    let expected = recite_writer_model::SearchIndex::build(&changed);
    assert_eq!(
        refreshed.search("mara", 10_000),
        expected.search("mara", 10_000)
    );
    assert_eq!(
        refreshed.search("zeppelin", 10_000),
        expected.search("zeppelin", 10_000)
    );
    assert_eq!(refreshed.search("courier", 10_000).0, 9_000);
    assert_eq!(before.total(), 10_000);
    for index in 0..10_000 {
        let old = before.get(index).ok_or("old hit")?;
        let next = after.get(index).ok_or("new hit")?;
        if [4, 18].contains(&(index / 500)) {
            assert!(old.text.contains("courier"));
            assert!(next.text.contains("zeppelin"));
        } else {
            assert!(std::ptr::eq(old, next), "unchanged saved shard was rebuilt");
        }
    }
    Ok(())
}
