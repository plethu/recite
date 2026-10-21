use super::*;

#[test]
fn staged_values_and_language_do_not_mutate_a_running_trial_until_restart() {
    let source = ":: start default\n> line@11111111111111111111 bind=(name:string=$person)\n  Hello {name}.\n-> END\n";
    let directory = project(source).unwrap();
    let mut writer = open(&directory).unwrap();
    writer
        .trial
        .settings
        .values
        .insert("person".into(), "Ada".into());
    let request = begin(&mut writer).unwrap();
    writer
        .trial
        .settings
        .values
        .insert("person".into(), "Grace".into());
    writer.trial.settings.locale = "fr-CA".into();
    assert!(finish(&mut writer, request).unwrap().is_none());
    assert_eq!(
        writer.session.model().preview_page().unwrap().text,
        "Hello Ada."
    );
    assert!(
        !writer
            .session
            .model()
            .preview_provenance()
            .unwrap()
            .caption
            .contains("fr-CA")
    );
    let restarted = begin(&mut writer).unwrap();
    assert!(matches!(
        finish(&mut writer, restarted).unwrap(),
        Some(Retired::Preview(_))
    ));
    assert_eq!(
        writer.session.model().preview_page().unwrap().text,
        "Hello Grace."
    );
    assert!(
        writer
            .session
            .model()
            .preview_provenance()
            .unwrap()
            .caption
            .contains("fr-CA")
    );
    assert!(!writer.session.dirty());
}

#[test]
fn obsolete_failure_retires_its_capture_and_reports_restart_without_erasing_the_run() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    let request = begin(&mut writer).unwrap();
    let captured = Arc::downgrade(&request);
    writer.trial.cancel();
    let (receipt, returned) = mpsc::sync_channel(1);
    writer.accept_trial(Output {
        request,
        result: Err("Obsolete error".into()),
        receipt,
    });
    let (request, retired) = returned.recv().unwrap();
    assert!(
        captured.upgrade().is_some(),
        "receipt owns capture until worker retires it"
    );
    drop(request);
    assert!(captured.upgrade().is_none());
    assert!(retired.is_none());
    assert!(!writer.trial.busy());
    assert!(writer.trial.status.contains("Restart when ready"));
    assert_eq!(
        writer.session.model().preview_page().unwrap().text,
        "Hello."
    );
}
