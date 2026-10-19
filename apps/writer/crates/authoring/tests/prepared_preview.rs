use recite_writer_model::{PreparedPreview, PreviewRequest, PreviewSetup, View, Workbench};

const SOURCE: &str = ":: start default\n> guide@12345678901234567890\n  Hello.\n-> END\n:: other\n> guide@12345678901234567891\n  Other.\n-> END\n";

#[test]
fn prepared_trials_are_send_and_remain_fixed_after_an_edit()
-> Result<(), Box<dyn std::error::Error>> {
    fn send<T: Send>() {}
    send::<PreviewRequest>();
    send::<PreparedPreview>();
    let mut model = Workbench::new(SOURCE)?;
    let request = model.preview_request()?;
    let prepared = std::thread::spawn(move || {
        request
            .prepare(PreviewSetup::default())
            .map_err(|e| e.to_string())
    })
    .join()
    .map_err(|_| "worker panic")??;
    assert!(model.accept_preview(prepared).is_ok());
    assert_eq!(model.preview_page().ok_or("page")?.text, "Hello.");
    model.select(View::Source)?;
    model.set_draft(SOURCE.replace("Hello.", "Changed."));
    model.apply()?;
    assert!(model.preview_stale());
    assert_eq!(model.preview_page().ok_or("page")?.text, "Hello.");
    Ok(())
}

#[test]
fn draft_selection_revision_and_reopened_identity_reject_without_replacing_run()
-> Result<(), Box<dyn std::error::Error>> {
    let mut model = Workbench::new(SOURCE)?;
    model.start_preview()?;
    let request = model.preview_request()?;
    let mut reopened = Workbench::new(SOURCE)?;
    assert!(
        reopened
            .accept_preview(request.prepare(PreviewSetup::default())?)
            .is_err()
    );
    model.select(View::Source)?;
    model.set_draft(SOURCE.replace("Hello.", "Draft."));
    assert!(model.preview_request().is_err());
    assert!(
        model
            .accept_preview(request.prepare(PreviewSetup::default())?)
            .is_err()
    );
    model.discard();
    model.inspect_block("other")?;
    assert!(
        model
            .accept_preview(request.prepare(PreviewSetup::default())?)
            .is_err()
    );
    assert_eq!(model.preview_page().ok_or("old page")?.text, "Hello.");
    model.select(View::Source)?;
    model.set_draft(SOURCE.replace("Hello.", "Accepted."));
    model.apply()?;
    assert!(
        model
            .accept_preview(request.prepare(PreviewSetup::default())?)
            .is_err()
    );
    Ok(())
}

#[test]
fn no_op_save_refresh_keeps_prepared_analysis_current() -> Result<(), Box<dyn std::error::Error>> {
    let mut model = Workbench::new(SOURCE)?;
    let request = model.preview_request()?;
    model.refresh_project(Default::default())?;
    assert!(request.is_current(&model));
    assert!(
        model
            .accept_preview(request.prepare(PreviewSetup::default())?)
            .is_ok()
    );
    Ok(())
}
