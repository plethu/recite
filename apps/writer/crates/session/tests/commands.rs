use recite_config::WriterCommand;
use recite_writer_model::{Document, View, Workbench};
use recite_writer_session::{
    commands::{targets, vim_alias},
    location::Location,
};

#[test]
fn lookup_uses_accepted_headers_and_rewrites_a_link_to_script_without_losing_project() {
    let mut model = Workbench::from_document(
        Document::new(":: first default\n-> END\n").unwrap(),
        Some(View::Source),
    )
    .unwrap();
    model.set_draft(":: draft_only default\n-> END\n".into());
    let all = targets::project(&model);
    assert!(
        all.iter()
            .any(|target| target.beat.as_deref() == Some("first"))
    );
    assert!(
        !all.iter()
            .any(|target| target.beat.as_deref() == Some("draft_only"))
    );
    let target = all.iter().find(|target| target.beat.is_some()).unwrap();
    let location = target.location(Location {
        project: Some("/project".into()),
        source: true,
        passage: Some("old".into()),
        ..Location::default()
    });
    assert!(!location.source);
    assert_eq!(location.project.as_deref(), Some("/project"));
    assert!(location.passage.is_none());
    assert_eq!(location.beat.as_deref(), Some("first"));
    assert_eq!(location.view, Some(recite_config::WriterView::Script));
    model.apply().unwrap();
    assert!(
        targets::project(&model)
            .iter()
            .any(|target| target.beat.as_deref() == Some("draft_only"))
    );
}

#[test]
fn lookup_rank_and_vim_aliases_are_consistent_for_both_native_frontends() {
    assert_eq!(targets::rank("Sérène Train", "sérène train"), Some(0));
    assert_eq!(targets::rank("Sérène Train", "SÉRÈNE"), Some(1));
    assert_eq!(targets::rank("Sérène Train", "train sérène"), Some(2));
    assert_eq!(targets::rank("Sérène Train", "missing"), None);
    assert_eq!(vim_alias(" :wa "), Some(WriterCommand::SaveAll));
    assert_eq!(vim_alias("w file.recite"), None);
}
