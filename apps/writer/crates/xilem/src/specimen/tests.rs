use super::*;
use crate::tests::gallery::{
    Variant,
    record::{Meta, Recorder},
    stage::{Look, Stage},
};
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

fn gallery_look(state: &mut Specimen, initial: bool) -> Look {
    state.theme.refresh(&state.config);
    Look {
        properties: state.theme.properties.clone(),
        canvas: state.theme.palette.canvas,
        initial: initial.then(|| state.theme.initial_properties()),
    }
}

/// The specimen's controls at rest and under the pointer and keyboard, for the design gallery.
pub(crate) fn gallery(record: &mut Recorder, variant: Variant) {
    record.scene(
        Meta {
            id: "specimen.controls",
            group: "Specimen",
            title: "Component specimen",
            intent: "Every production control and type style on one page: the design system in isolation.",
        },
        |record| {
            let mut state = Specimen::default();
            variant.configure(&mut state.config);
            let mut stage = Stage::new(state, surface, gallery_look, variant.frame);
            stage.shot(record, "At rest");
            for caption in ["Try scene", "Add beat", "Arrange automatically"] {
                if stage.hover(caption) {
                    stage.shot(record, &format!("{caption}: hover"));
                }
                if stage.press(caption) {
                    stage.shot(record, &format!("{caption}: pressed"));
                    stage.release();
                }
            }
            stage.park_pointer();
            if stage.focus("Find a destination") {
                stage.shot(record, "Search field focused");
            }
        },
    );
}
