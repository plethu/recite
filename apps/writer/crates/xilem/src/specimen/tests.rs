use super::*;
use crate::tests::support::mounted::Mounted;
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(state: &mut Specimen) -> impl WidgetView<Specimen, Widget = Passthrough> + use<> {
    crate::keyed::keyed((), view::specimen(state))
}

#[test]
fn specimen_controls_preserve_live_fields_across_appearance_and_scale_changes() {
    let mut mounted = Mounted::new(Specimen::default(), surface);
    let manuscript = mounted.find("Specimen manuscript");
    mounted.type_text("Specimen manuscript", " 🦊");
    let text = mounted.state.manuscript.clone();
    assert!(text.contains("🦊"));
    for caption in [
        "Dark appearance",
        "Monochrome",
        "200% UI scale",
        "Reduced motion",
        "Light appearance",
        "100% UI scale",
    ] {
        mounted.activate(caption);
        assert_eq!(mounted.state.manuscript, text);
        assert_eq!(mounted.find("Specimen manuscript"), manuscript);
    }
    assert!(mounted.state.config.monochrome);
    assert!(mounted.state.config.reduced_motion);
    assert_eq!(
        mounted
            .state
            .config
            .presentation
            .value(WriterPresentationField::UiScale),
        100
    );
    for (caption, workspace) in [
        ("Map", Workspace::Map),
        ("Split", Workspace::Split),
        ("Script", Workspace::Script),
    ] {
        mounted.activate(caption);
        assert_eq!(mounted.state.workspace, workspace);
    }
    mounted.activate("Reviewed translation");
    assert!(mounted.state.reviewed);
    for (caption, expected) in [
        ("Try scene", "Primary action activated."),
        ("Add beat", "Secondary action activated."),
        ("Arrange automatically", "Quiet action activated."),
    ] {
        mounted.activate(caption);
        assert_eq!(mounted.state.message, expected);
    }
    mounted.activate("Save changes");
    assert_eq!(mounted.state.message, "Quiet action activated.");
    mounted.type_text("Find a destination", "Station");
    assert_eq!(mounted.state.query, "Station");
    mounted.activate("Station History");
    assert_eq!(mounted.state.destination, "Station History");
    assert_eq!(
        mounted.state.message,
        "Destination selected: Station History."
    );
    mounted.activate("Close specimen");
    assert!(!mounted.state.keep_running());
}
