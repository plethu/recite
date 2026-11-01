use super::*;
use crate::tests::support;
use recite_writer_model::View;
use tokio::sync::mpsc as async_mpsc;

#[test]
fn cold_link_failure_installs_recovered_source_and_reports_failure_to_the_forwarded_launch() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.change(|model| {
        model.select(View::Source)?;
        model.stage_repaired_source(":: recovered\n? unfinished".to_owned().into())
    });
    let recovered = writer.session.model().recovery();
    writer.session.project_mut().unwrap().checkpoint().unwrap();
    drop(writer);
    let mut app = App::empty();
    let (sender, mut jobs) = async_mpsc::unbounded_channel();
    app.sender = Some(sender);
    let (reply, waiting) = mpsc::channel();
    app.start_open(
        LaunchOptions {
            intent: Startup::Writer {
                project: Some(directory.path().to_owned()),
                route: Some("recite://writer/write?scene=scene.recite&view=source".into()),
            },
            editor: None,
            document: None,
        },
        Some(reply),
        None,
    )
    .unwrap();
    let Job::Open(request) = jobs.try_recv().unwrap() else {
        panic!("project request")
    };
    let result = load(&request);
    let (retire, retired) = mpsc::sync_channel(1);
    app.accept_open(Opened {
        request,
        result,
        retire,
    });
    assert!(retired.try_recv().unwrap().is_none());
    assert!(waiting.try_recv().unwrap().is_err());
    let writer = app.writer.as_ref().unwrap();
    assert_eq!(writer.session.model().recovery(), recovered);
    assert!(writer.source_repair_pending());
    assert!(app.status.contains("Repair and import"));
}
