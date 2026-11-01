use super::*;
use crate::tests::support;

fn documents(app: &App) -> Vec<String> {
    entries(app, Search::GoTo)
        .unwrap()
        .into_iter()
        .filter_map(|entry| {
            if let Item::Lookup(target) = entry.item {
                Some(target.document)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn project_lookup_keeps_its_documents_while_a_worker_exclusively_owns_the_project() {
    let directory = support::project(":: start default\n> @ffee6p8\n  Hello.\n-> END\n").unwrap();
    std::fs::write(
        directory.path().join("other.recite"),
        ":: other\n> @rthh7ab\n  Other.\n-> END\n",
    )
    .unwrap();
    let writer = support::open(&directory).unwrap();
    let mut app = App::with_writer(writer);
    let before = documents(&app);
    assert!(before.iter().any(|key| key == "other.recite"));
    let mut checkout = app.writer.as_mut().unwrap().session.checkout().unwrap();
    assert!(app.writer.as_ref().unwrap().session.project().is_none());
    assert_eq!(
        documents(&app),
        before,
        "a project never becomes the temporary examples during a checked action"
    );
    app.writer
        .as_mut()
        .unwrap()
        .session
        .restore_checkout(&mut checkout, None)
        .unwrap();
    assert_eq!(documents(&app), before);
}
