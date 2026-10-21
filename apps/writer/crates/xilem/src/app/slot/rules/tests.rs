use super::*;
use crate::{
    app::view,
    navigation::{self, Target},
    tests::support::mounted::Mounted,
};
use recite_writer_session::location::{Location, Screen};
use tokio::sync::mpsc as async_mpsc;
use xilem::{WidgetView, masonry::widgets::Passthrough};

const SOURCE: &str = "# Keep manuscript comments.\n:: start default\n? reply@22222222222222222222 requires=(standing(3))\n  Let me through.\n  -> accepted\n:: accepted\n! immediate play_sfx(gate_latch)\n# Keep effect comments.\n-> END\n";

fn surface(app: &mut App) -> impl WidgetView<App, Widget = Passthrough> + use<> {
    crate::keyed::keyed((), view::app(app))
}

fn choose<V: WidgetView<App, Widget = Passthrough>>(
    mounted: &mut Mounted<App, V>,
    caption: &str,
    value: &str,
) {
    mounted.activate(caption);
    assert!(mounted.state.picker.is_some());
    mounted.replace_text("Search query", value);
    mounted.activate(&format!("Selected · {value}"));
    assert!(mounted.state.picker.is_none());
}

#[test]
fn mounted_rules_picker_grouping_effect_order_and_history_preserve_authored_bytes() {
    let directory = project(SOURCE).unwrap();
    std::fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n[project]\nschema='schema.json'\n",
    )
    .unwrap();
    std::fs::write(directory.path().join("schema.json"), r#"{
        "schema_version":1,
        "registries":{"sfx":{"values":["gate_latch","bell"]},"place":{"values":["floodgate"]}},
        "conditions":{"standing":{"params":[{"name":"level","type":"int"}]},"ready":{}},
        "effects":{"play_sfx":{"modes":["immediate","deferred"],"params":[{"name":"sound","type":"registry:sfx"}]},
            "mark_map":{"modes":["blocking"],"params":[{"name":"place","type":"registry:place"}]}}
    }"#).unwrap();
    let mut writer = open(&directory).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = async_mpsc::unbounded_channel::<navigation::worker::Job>();
    writer.navigation.sender = Some(sender.clone());
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.navigate(Target::Location(Location {
        screen: Screen::Rules,
        passage: Some("22222222222222222222".into()),
        ..Location::default()
    }));
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    let mut mounted = Mounted::new(App::with_writer(writer), surface);
    mounted.state.writer.as_mut().unwrap().navigation.sender = Some(sender);
    mounted.state.writer.as_mut().unwrap().navigation.runtime = Some(runtime.handle().clone());
    choose(&mut mounted, "Choose value…", "bell");
    assert!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .draft()
            .contains("play_sfx(bell)")
    );
    choose(&mut mounted, "Add effect…", "mark_map");
    mounted.activate("Effect 1: play_sfx · Immediate · Show actions");
    mounted.activate("Delivery: Deferred");
    mounted.activate("Move effect down");
    mounted.activate("Effect 2: play_sfx · Deferred · Show actions");
    mounted.activate("Move effect up");
    mounted.activate("Effect 2: mark_map · Blocking · Show actions");
    mounted.activate("Remove effect");
    choose(&mut mounted, "Add effect…", "mark_map");
    choose(&mut mounted, "Add condition…", "ready");
    mounted.activate("Condition: All of these · Show actions");
    mounted.activate("Match any");
    mounted.activate("Condition: Any of these · Show actions");
    mounted.activate("Negate");
    let writer = mounted.state.writer.as_ref().unwrap();
    assert!(writer.session.model().has_draft());
    assert_eq!(writer.session.model().document().source(), SOURCE);
    assert_eq!(
        std::fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
    mounted.activate("Apply reply");
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut jobs).unwrap();
    mounted.settle();
    assert!(
        !mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .has_draft(),
        "{}\n{}",
        mounted.state.writer.as_ref().unwrap().status,
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .draft()
    );
    let accepted = mounted
        .state
        .writer
        .as_ref()
        .unwrap()
        .session
        .model()
        .document()
        .source()
        .to_owned();
    for expected in [
        "# Keep manuscript comments.",
        "# Keep effect comments.",
        "reply@22222222222222222222",
        "play_sfx(bell)",
        "mark_map(floodgate)",
        "ready()",
        "not",
        " or ",
    ] {
        assert!(
            accepted.contains(expected),
            "missing {expected}: {accepted}"
        );
    }
    assert!(accepted.contains("! deferred play_sfx(bell)"));
    assert!(
        accepted.find("play_sfx(bell)").unwrap() < accepted.find("mark_map(floodgate)").unwrap()
    );
    choose(&mut mounted, "Commands", "Undo");
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut jobs).unwrap();
    mounted.settle();
    assert_eq!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .document()
            .source(),
        SOURCE
    );
    choose(&mut mounted, "Commands", "Redo");
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut jobs).unwrap();
    mounted.settle();
    assert_eq!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .document()
            .source(),
        accepted
    );
    assert_eq!(
        std::fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
}
