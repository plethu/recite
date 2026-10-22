use super::*;
use crate::{
    external_editor::tests::Driver as EditorDriver, keyed::keyed, tests::support::mounted::Mounted,
    workspaces::tests::ToolDriver,
};
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn settings_conflict_stays_guarded_through_manifest_repair_and_a_fresh_import() {
    let (directory, mut writer, _) = fixture();
    writer.change(|model| {
        model.set_draft("Preserved manuscript field.".into());
        Ok(())
    });
    let manifest = directory.path().join("recite.project.toml");
    let original = fs::read_to_string(&manifest).unwrap();
    let mut mounted = Mounted::new(writer, surface);
    let mut jobs = ToolDriver::new(&mut mounted.state);
    let mut editors = EditorDriver::new(&mut mounted.state);
    mounted.activate("Export settings draft");
    jobs.finish(&mut mounted.state);
    mounted.settle();
    let artifact = mounted
        .state
        .workspaces
        .settings
        .artifact()
        .unwrap()
        .to_owned();
    mounted.activate("Open settings draft in editor");
    editors.complete(&mut mounted.state, &artifact, Ok(()));
    assert!(mounted.state.status.contains("Import settings draft"));
    fs::write(&artifact, original.replace("trial", "candidate")).unwrap();
    mounted.activate("Import settings draft");
    jobs.finish(&mut mounted.state);
    mounted.settle();
    let external = original.replace("trial", "external");
    fs::write(&manifest, &external).unwrap();
    mounted.activate("Apply reviewed settings");
    jobs.finish(&mut mounted.state);
    mounted.settle();
    assert!(mounted.state.project_settings().unwrap().requires_reload());
    assert_eq!(fs::read_to_string(&manifest).unwrap(), external);
    mounted.activate("Open saved manifest in editor");
    editors.complete(&mut mounted.state, &manifest, Ok(()));
    assert!(mounted.state.status.contains("Reload saved settings"));
    fs::write(&manifest, "broken TOML [").unwrap();
    mounted.settle();
    mounted.activate("Reload saved settings");
    jobs.finish(&mut mounted.state);
    mounted.settle();
    assert!(mounted.state.project_settings().unwrap().requires_reload());
    assert!(!mounted.contains_text("Apply reviewed settings"));
    mounted.state.apply_project_settings();
    assert!(!mounted.state.project_jobs_busy());
    assert_eq!(fs::read_to_string(&manifest).unwrap(), "broken TOML [");
    fs::write(&manifest, &external).unwrap();
    mounted.activate("Reload saved settings");
    jobs.finish(&mut mounted.state);
    mounted.settle();
    assert!(!mounted.state.project_settings().unwrap().requires_reload());
    assert!(
        mounted
            .state
            .project_settings()
            .unwrap()
            .reviewed_source()
            .is_none()
    );
    assert!(
        !mounted
            .state
            .session
            .project()
            .unwrap()
            .files()
            .configuration_refresh_required()
    );
    mounted.activate("Export settings draft");
    jobs.finish(&mut mounted.state);
    mounted.settle();
    let fresh = mounted
        .state
        .workspaces
        .settings
        .artifact()
        .unwrap()
        .to_owned();
    let accepted = external.replace("external", "accepted");
    fs::write(&fresh, &accepted).unwrap();
    mounted.replace_text("Settings draft path", fresh.to_str().unwrap());
    mounted.activate("Import settings draft");
    jobs.finish(&mut mounted.state);
    mounted.settle();
    assert_eq!(fs::read_to_string(&manifest).unwrap(), external);
    mounted.activate("Apply reviewed settings");
    jobs.finish(&mut mounted.state);
    assert_eq!(fs::read_to_string(&manifest).unwrap(), accepted);
    assert_eq!(
        mounted.state.session.model().draft(),
        "Preserved manuscript field."
    );
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Hello.")
    );
    assert!(!mounted.state.project_jobs_busy());
}
