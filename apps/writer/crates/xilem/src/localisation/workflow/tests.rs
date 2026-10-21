use super::*;
use crate::navigation::Target;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use recite_writer_session::location::{Location, Screen as LinkedScreen};
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn mounted_source_context_pages_and_source_links_keep_the_translation_draft() {
    let texts = [
        "Earlier source context. ".repeat(80),
        "Selected source prose. ".repeat(80),
        "Later source context. ".repeat(80),
    ];
    let mut source = ":: start default\n".to_owned();
    let mut po = String::new();
    for (index, text) in texts.iter().enumerate() {
        source.push_str(&format!("> line@{index:020x}\n  {text}\n"));
        po.push_str(&format!("#. file: scene.recite\n#. block: start\nmsgctxt \"{index:020x}\"\nmsgid \"{text}\"\nmsgstr \"Original translation {index}.\"\n\n"));
    }
    source.push_str("-> END\n");
    let directory = support::project(&source).unwrap();
    fs::write(directory.path().join("fr.po"), &po).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut driver = ToolDriver::new(&mut writer);
    writer.open_localisation();
    writer.localisation.path = directory.path().join("fr.po").display().to_string();
    writer.open_catalogue_path();
    driver.finish(&mut writer);
    let id = writer.localisation.slot.document().unwrap().entries()[1].id();
    let key = EntryKey {
        catalogue: writer.localisation.slot.identity().unwrap().clone(),
        id,
    };
    writer.inspect_translation(&key);
    driver.finish(&mut writer);
    let mut mounted = Mounted::new(writer, surface);
    let mut driver = ToolDriver::new(&mut mounted.state);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut navigation) = mpsc::unbounded_channel();
    mounted.state.navigation.sender = Some(sender);
    mounted.state.navigation.runtime = Some(runtime.handle().clone());
    mounted.replace_text(
        "Translation form 1",
        "Retained translation while reading source.",
    );
    mounted.activate("More translation source");
    mounted.activate("Previous translation source part");
    mounted.activate("More source context");
    mounted.activate("Previous source context");
    mounted.activate("Show source passage");
    support::finish_navigation(&mut mounted.state, &mut navigation).unwrap();
    mounted.settle();
    assert_eq!(
        mounted.state.session.model().view(),
        &View::Passage("00000000000000000001".into())
    );
    assert_eq!(
        form(&mounted.state, &key),
        "Retained translation while reading source."
    );
    mounted.activate("Back");
    support::finish_navigation(&mut mounted.state, &mut navigation).unwrap();
    driver.finish(&mut mounted.state);
    mounted.settle();
    let linked = Location {
        screen: LinkedScreen::Localise,
        document: "scene.recite".into(),
        catalogue: Some("fr.po".into()),
        passage: Some("00000000000000000001".into()),
        ..Location::default()
    };
    mounted.state.navigate(Target::Location(linked.clone()));
    support::finish_navigation(&mut mounted.state, &mut navigation).unwrap();
    mounted.settle();
    mounted.activate("More source passage");
    mounted.activate("Previous source part");
    mounted.activate("Translate this passage");
    assert_eq!(
        form(&mounted.state, &key),
        "Retained translation while reading source."
    );
    mounted.state.navigate(Target::Location(linked));
    support::finish_navigation(&mut mounted.state, &mut navigation).unwrap();
    mounted.settle();
    mounted.activate("Open translation queue");
    mounted.type_text("Search translations", "Selected source");
    mounted.activate("Search translations");
    driver.finish(&mut mounted.state);
    mounted.settle();
    mounted.activate_prefix("Open translation · ");
    driver.finish(&mut mounted.state);
    mounted.settle();
    mounted.activate("Preview this passage");
    support::finish_navigation(&mut mounted.state, &mut navigation).unwrap();
    assert!(mounted.state.trial.visible);
    assert_eq!(
        form(&mounted.state, &key),
        "Retained translation while reading source."
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("fr.po")).unwrap(),
        po
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        source
    );
}

#[test]
fn mounted_unsaved_translation_reentry_and_paged_report_keep_close_recoverable() {
    let (directory, writer, _) = fixture();
    let key = key(&writer);
    let mut mounted = Mounted::new(writer, surface);
    let mut driver = ToolDriver::new(&mut mounted.state);
    mounted.replace_text("Translation form 1", "Discard only this translation.");
    mounted.state.change(|model| {
        model.set_draft("Retain source recovery.".into());
        Ok(())
    });
    mounted.activate("Close");
    assert!(mounted.state.phase == Phase::ConfirmClose);
    mounted.activate("Open unsaved translations");
    driver.finish(&mut mounted.state);
    mounted.settle();
    assert!(mounted.state.phase == Phase::Writing);
    assert_eq!(form(&mounted.state, &key), "Discard only this translation.");
    let report = format!(
        "Report start\n{}\nReport end",
        "A detailed diagnostic remains readable without losing the translation. ".repeat(60)
    );
    mounted.state.catalogue_feedback(report);
    mounted.settle();
    assert!(mounted.contains_text("Report start"));
    let parts = mounted.state.localisation.report.len();
    assert!(parts > 1);
    for _ in 1..parts {
        mounted.activate("More catalogue report");
    }
    assert!(mounted.contains_text("Report end"));
    mounted.activate("Previous catalogue report part");
    assert!(!mounted.contains_text("Report end"));
    mounted.activate("Close");
    mounted.activate("Discard translations and close");
    driver.finish(&mut mounted.state);
    assert!(mounted.state.phase == Phase::Closed);
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
    drop(mounted);
    assert!(
        !Catalogue::open_recoverable(&directory.path().join("fr.po"))
            .unwrap()
            .dirty()
    );
    let reopened = support::open(&directory).unwrap();
    assert_eq!(reopened.session.model().draft(), "Retain source recovery.");
}
