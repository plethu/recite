use super::*;
use recite_writer_session::navigation::{Destination, Prepare};

#[test]
fn failed_enqueue_retires_checkpoint_without_opening_destination_and_preserves_recovery() {
    let Fixture {
        directory,
        mut writer,
        mut jobs,
        other,
        _runtime: runtime,
    } = fixture().unwrap();
    writer.change(|model| {
        model.set_draft("Recovered after rejected submission.".into());
        Ok(())
    });
    let prepare = Prepare {
        destination: Destination::Switch(Box::new(
            writer
                .session
                .project()
                .unwrap()
                .files()
                .switch_request(writer.session.model(), &other, None)
                .unwrap(),
        )),
        location: None,
        project: Some(directory.path().to_owned()),
        catalogue: None,
        reuse_rules: false,
        initial_view: None,
    };
    let (files, model) = writer.session.parts_mut().unwrap();
    let checkpoint = files.begin_navigation_checkpoint(model).unwrap();
    let request = Arc::new(Request {
        target: Target::Document(other.clone()),
        origin: Origin::capture(&writer),
        cancel: CancellationToken::new(),
        forwarded: None,
    });
    // Model a receiver closing between the initial guard and enqueue.
    jobs.close();
    assert!(
        writer
            .send_navigation(
                request.clone(),
                worker::Work::Navigate {
                    prepare: Box::new(prepare),
                    checkpoint: Some(Box::new(checkpoint)),
                }
            )
            .is_err()
    );
    assert!(request.checkpoint().is_err());
    assert!(!writer.navigation.busy());
    drop(recite_writer_session::recovery::RecoveryStore::open(&other).unwrap());
    drop(writer);
    // Runtime shutdown waits for the rejected capture's blocking retirement.
    drop(runtime);
    let writer = open(&directory).unwrap();
    assert_eq!(
        writer.session.model().draft(),
        "Recovered after rejected submission."
    );
}
