use super::*;
use crate::localisation::catalogue::Draft;
use std::fs;

fn fixture() -> (tempfile::TempDir, Catalogue) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("fr.po");
    fs::write(
        &path,
        "# translator note\nmsgctxt \"ffee6p8\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n",
    )
    .unwrap();
    let catalogue = Catalogue::open_recoverable(&path).unwrap();
    (root, catalogue)
}
fn save(catalogue: &mut Catalogue) {
    let mut request = catalogue.begin_save(None).unwrap();
    let receipt = request.execute().unwrap();
    if let Some(ticket) = catalogue.accept_save(&mut request, receipt).unwrap() {
        ticket.wait().unwrap();
    }
    catalogue.finish_save();
}
fn poison(queue: &CatalogueQueue) {
    let queue = queue.clone();
    assert!(
        std::thread::spawn(move || {
            let mut index = queue.0.lock().unwrap();
            index.replace_forms(&mut [(PoEntryId::new(0), vec!["partially patched".into()])]);
            panic!("interrupted derived-cache update");
        })
        .join()
        .is_err()
    );
}
#[test]
fn the_sole_catalogue_owner_can_move_to_a_worker_and_patch_a_retained_queue() {
    fn assert_send<T: Send>() {}
    assert_send::<Catalogue>();
    let (_root, mut catalogue) = fixture();
    let queue = catalogue.queue_index(None);
    let catalogue = std::thread::spawn(move || {
        catalogue.update(
            PoEntryId::new(0),
            Draft {
                forms: vec!["Salut".into()],
                reviewed: true,
            },
        );
        save(&mut catalogue);
        catalogue
    })
    .join()
    .unwrap();
    assert!(queue == catalogue.queue_index(None));
    assert_eq!(
        queue.matching_entries(&catalogue, "salut", false).unwrap(),
        [PoEntryId::new(0)]
    );
    assert!(
        queue
            .matching_entries(&catalogue, "bonjour", false)
            .unwrap()
            .is_empty()
    );
    assert!(!catalogue.dirty());
}
#[test]
fn poisoned_derived_forms_are_reported_and_rebuilt_from_the_canonical_document() {
    let (_root, mut catalogue) = fixture();
    let queue = catalogue.queue_index(None);
    poison(&queue);
    assert!(
        queue
            .matching_entries(&catalogue, "partially patched", false)
            .is_err()
    );
    let rebuilt = catalogue.queue_index(None);
    assert!(queue != rebuilt);
    assert_eq!(
        rebuilt
            .matching_entries(&catalogue, "bonjour", false)
            .unwrap(),
        [PoEntryId::new(0)]
    );
    poison(&rebuilt);
    catalogue.update(
        PoEntryId::new(0),
        Draft {
            forms: vec!["Salut".into()],
            reviewed: false,
        },
    );
    save(&mut catalogue);
    let saved = catalogue.queue_index(None);
    assert!(saved != rebuilt);
    assert_eq!(
        saved.matching_entries(&catalogue, "salut", false).unwrap(),
        [PoEntryId::new(0)]
    );
    assert!(
        saved
            .matching_entries(&catalogue, "partially patched", false)
            .unwrap()
            .is_empty()
    );
}
