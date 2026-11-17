#![allow(unused_imports)]
use super::super::*;
use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use recite_core::compiled::decode_compiled_dialogue_messagepack;
use recite_writer_session::builds::job;
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn build_options_on_the_slip_keep_saved_inputs_separate_from_save_all_and_build() {
    let directory = fixture().unwrap();
    let manifest = "format_version=1\n[project]\ncontent_set='trial'\nversion='1'\nschema='schema.json'\n".to_owned() + &(0..6).map(|index| format!("[[scenes]]\nid='scene{index}'\nasset='build/scene{index}.recitec'\nblock='start'\nparticipants=['mara']\n")).collect::<String>();
    fs::write(directory.path().join("recite.project.toml"), manifest).unwrap();
    let original = fs::read_to_string(directory.path().join("scene.recite")).unwrap();
    let mut writer = support::open(&directory).unwrap();
    // The options open on a finished build's result slip; that first build's outputs go.
    writer.build_scenes();
    finish_build(&mut writer);
    fs::remove_dir_all(directory.path().join("build")).unwrap();
    writer.toggle_build_options();
    writer.change(|model| {
        model.set_draft("Pending build manuscript.".into());
        Ok(())
    });
    let mut mounted = Mounted::new(writer, surface);
    // Every output is a radio on the slip; nothing pages.
    mounted.activate("scene5");
    assert_eq!(
        mounted
            .state
            .session
            .project()
            .unwrap()
            .files()
            .builds
            .selected
            .as_deref(),
        Some("build/scene5.recitec")
    );
    mounted.activate("Build saved files");
    finish_build(&mut mounted.state);
    assert!(directory.path().join("build/scene5.recitec").is_file());
    let saved = decode_compiled_dialogue_messagepack(
        &fs::read(directory.path().join("build/scene5.recitec")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved.lines[0].source_text, "Hello.");
    assert!(!directory.path().join("build/scene0.recitec").exists());
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        original
    );
    assert_eq!(
        mounted.state.session.model().draft(),
        "Pending build manuscript."
    );
    mounted.state.toggle_build_options();
    mounted.settle();
    mounted.activate("All outputs");
    let (sender, mut saves) = mpsc::unbounded_channel();
    mounted.state.work.sender = Some(sender);
    mounted.activate("Save all and build");
    support::finish_work(&mut mounted.state, &mut saves).unwrap();
    finish_build(&mut mounted.state);
    assert!(
        mounted
            .state
            .session
            .project()
            .unwrap()
            .files()
            .builds
            .result
            .as_ref()
            .unwrap()
            .is_success()
    );
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Pending build manuscript.")
    );
    for index in 0..6 {
        let bytes = fs::read(directory.path().join(format!("build/scene{index}.recitec"))).unwrap();
        let generated = decode_compiled_dialogue_messagepack(&bytes).unwrap();
        assert_eq!(generated.lines[0].source_text, "Pending build manuscript.");
    }
}

#[test]
fn a_failed_build_bar_never_runs_wider_than_the_window_at_200_percent() {
    let directory = fixture().unwrap();
    // No scenes declared: the build fails with a long message.
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n[project]\ncontent_set='trial'\nversion='1'\nschema='schema.json'\n",
    )
    .unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut config = writer.config.clone();
    config.writer.presentation = config
        .writer
        .presentation
        .with_value(recite_config::WriterPresentationField::UiScale, 200)
        .unwrap();
    writer.configure(config);
    writer.viewport.width = 1200.;
    writer.build_scenes();
    finish_build(&mut writer);
    let result = writer.build_result().unwrap();
    assert!(!result.succeeded);
    let message = format!(
        "{}{}",
        result.headline,
        result
            .detail
            .map(|detail| format!(" · {detail}"))
            .unwrap_or_default()
    );
    let mounted = Mounted::new(writer, surface);
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut bar = None;
    while let Some(node) = pending.pop() {
        // Every control stays inside the 1200px window, the bar's actions included.
        if let Some(bounds) = node.bounding_box() {
            assert!(
                bounds.x0 >= -0.5 && bounds.x1 <= 1200.5,
                "{:?} {:?} runs outside the window: {bounds:?}",
                node.role(),
                node.label()
            );
        }
        if node.label().as_deref() == Some("Build result") {
            bar = Some(node.description().map(|text| text.to_string()));
        }
        pending.extend(node.children());
    }
    // The bar's description carries the whole message the bar abbreviates.
    assert_eq!(bar, Some(Some(message)));
}
