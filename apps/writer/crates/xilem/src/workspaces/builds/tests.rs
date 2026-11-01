use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use recite_core::compiled::decode_compiled_dialogue_messagepack;
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn mounted_build_selection_pages_keep_saved_inputs_separate_from_save_all_and_build() {
    let directory = fixture().unwrap();
    let manifest = "format_version=1\n[project]\ncontent_set='trial'\nversion='1'\nschema='schema.json'\n".to_owned() + &(0..6).map(|index| format!("[[scenes]]\nid='scene{index}'\nasset='build/scene{index}.recitec'\nblock='start'\nparticipants=['mara']\n")).collect::<String>();
    fs::write(directory.path().join("recite.project.toml"), manifest).unwrap();
    let original = fs::read_to_string(directory.path().join("scene.recite")).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut prepared = opening::Work::capture(&writer, Open::Build)
        .unwrap()
        .execute()
        .unwrap();
    writer.accept_project_workspace(&mut prepared).unwrap();
    writer.change(|model| {
        model.set_draft("Pending build manuscript.".into());
        Ok(())
    });
    let mut mounted = Mounted::new(writer, surface);
    mounted.activate("Next outputs");
    mounted.activate("scene5");
    mounted.activate("Previous outputs");
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
    mounted.activate("Build saved");
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
