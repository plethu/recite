use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn mounted_catalogue_notice_compares_without_applying_drafts_and_retires_with_its_lifetime() {
    let directory = support::project(SOURCE).unwrap();
    let po = directory.path().join("fr.po");
    let original = "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n";
    fs::write(&po, original).unwrap();
    let writer = support::open(&directory).unwrap();
    let mut mounted = Mounted::new(writer, surface);
    let mut tools = ToolDriver::new(&mut mounted.state);
    mounted.state.open_localisation();
    mounted.settle();
    mounted.type_text("Catalogue path", po.to_str().unwrap());
    mounted.activate("Open catalogue");
    tools.finish(&mut mounted.state);
    mounted.settle();
    mounted.activate_prefix("Open translation · ");
    tools.finish(&mut mounted.state);
    mounted.settle();
    mounted.replace_text("Translation form 1", "Pending translation.");
    let key = mounted
        .state
        .localisation
        .preview_catalogue()
        .unwrap()
        .changed_entries()
        .next()
        .unwrap();
    mounted.state.change(|model| {
        model.set_draft("Pending manuscript.".into());
        Ok(())
    });
    let mut observations = connect(&mut mounted.state);
    let source = directory.path().join("scene.recite");
    fs::write(&source, SOURCE.replace("Hello.", "External source.")).unwrap();
    let external = original.replace("Bonjour.", "External translation.");
    fs::write(&po, &external).unwrap();
    changed(&mut mounted.state, [source.clone(), po.clone()]);
    finish(&mut mounted.state, &mut observations);
    assert_eq!(mounted.state.notices.found.len(), 2);
    mounted.settle();
    mounted.activate("Next changed file");
    assert!(matches!(
        mounted.state.notices.selected(),
        Some((Target::Catalogue(_), _))
    ));
    mounted.activate("Compare changed file");
    tools.finish(&mut mounted.state);
    mounted.settle();
    mounted.find("Keep translation drafts");
    assert_eq!(
        mounted
            .state
            .localisation
            .preview_catalogue()
            .unwrap()
            .draft(key)
            .unwrap()
            .forms[0],
        "Pending translation."
    );
    assert_eq!(mounted.state.session.model().draft(), "Pending manuscript.");
    assert_eq!(mounted.state.session.model().document().source(), SOURCE);
    assert_eq!(fs::read_to_string(&po).unwrap(), external);
    mounted.activate("Keep translation drafts");
    tools.finish(&mut mounted.state);
    mounted.settle();
    assert_eq!(
        fs::read_to_string(&po).unwrap(),
        external,
        "comparison resolution does not save"
    );
    mounted.activate("Dismiss notice");
    assert_eq!(mounted.state.notices.found.len(), 1);
    // A save/reopen changes catalogue authority even at the same path.
    changed(&mut mounted.state, [po.clone()]);
    let old = observations.try_recv().unwrap().observe();
    mounted.state.save_translation_drafts();
    tools.finish(&mut mounted.state);
    mounted.settle();
    mounted.activate("Catalogue files");
    mounted.activate("Reload catalogue");
    tools.finish(&mut mounted.state);
    mounted.state.accept_external_notice(Output {
        window: mounted.state.window,
        event: Event::Observed(old),
    });
    mounted.state.resume_external_notices();
    assert!(
        !mounted
            .state
            .notices
            .found
            .keys()
            .any(|target| matches!(target, Target::Catalogue(_)))
    );
    mounted.settle();
    mounted.activate("Dismiss notice");
    assert!(mounted.state.notices.found.is_empty());
    assert_eq!(mounted.state.session.model().draft(), "Pending manuscript.");
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("External source.")
    );
}
