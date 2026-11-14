use super::*;
use crate::{
    keyed::keyed,
    tests::support::{mounted::Mounted, open, project},
};
use recite_config::WriterPresentationField;
use xilem::masonry::{kurbo::Rect, widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn revealing_a_long_reply_id_never_moves_the_following_reply() {
    let long_id = "long_source_id_".repeat(12);
    let source = format!(
        ":: start default\n> @ffee6p8\n  Hello.\n? @{long_id}\n  First reply\n  -> next\n? @bqyvpqq\n  Second reply\n  -> next\n:: next\n> @z0g6c80\n  Goodbye.\n-> END\n"
    );
    let directory = project(&source).unwrap();
    let mut writer = open(&directory).unwrap();
    let mut config = writer.config.clone();
    config.writer.presentation = config
        .writer
        .presentation
        .with_value(WriterPresentationField::UiScale, 200)
        .unwrap();
    writer.configure(config);
    let mut mounted = Mounted::new(writer, surface);
    let following = mounted.find("Edit passage 3");
    let before = mounted
        .harness
        .get_widget_with_id(following)
        .ctx()
        .window_transform()
        .translation()
        .y;
    mounted.state.change(|model| {
        model.select(View::Passage(long_id.clone()))?;
        model.set_draft("Edited first reply".into());
        Ok(())
    });
    mounted.settle();
    assert_eq!(mounted.find("Edit passage 3"), following);
    let after = mounted
        .harness
        .get_widget_with_id(following)
        .ctx()
        .window_transform()
        .translation()
        .y;
    assert_eq!(
        before, after,
        "source identity must share the destination row's fixed space"
    );
}

#[test]
fn destinations_share_one_left_aligned_column_and_the_reply_count_sits_with_its_caption() {
    let mut writer = Writer::from_session(crate::session::Session::examples().unwrap(), None);
    writer
        .follow_location(&recite_writer_session::location::Location {
            document: "relay_hub.recite".into(),
            ..Default::default()
        })
        .unwrap();
    writer.viewport.width = 1200.;
    let mounted = Mounted::new(writer, surface);
    let left = |name: &str| {
        let widget = mounted.harness.get_widget_with_id(mounted.find(name));
        widget
            .ctx()
            .window_transform()
            .transform_rect_bbox(widget.ctx().border_box())
            .x0
    };
    let column = left("→ Missing Courier");
    for name in ["→ Station History", "→ Insult", "→ Goodbye"] {
        assert_eq!(left(name), column, "{name}");
    }
    assert!(mounted.contains_text("4 replies"));
    // The file and beat identity belong to the folio, not a second line under the title.
    assert!(!mounted.contains_text("relay_hub.recite  ::  relay_desk"));
}

#[test]
fn resting_replies_keep_the_column_rhythm_without_a_reserved_label_line() {
    let mut writer = Writer::from_session(crate::session::Session::examples().unwrap(), None);
    writer
        .follow_location(&recite_writer_session::location::Location {
            document: "relay_hub.recite".into(),
            ..Default::default()
        })
        .unwrap();
    writer.viewport.width = 1200.;
    let properties = crate::presentation::Theme::new(&writer.config.writer).properties;
    let mut mounted = Mounted::new(writer, surface);
    mounted.harness.set_default_properties(properties);
    mounted.settle();
    let bounds = |name: &str| {
        let widget = mounted.harness.get_widget_with_id(mounted.find(name));
        widget
            .ctx()
            .window_transform()
            .transform_rect_bbox(widget.ctx().border_box())
    };
    let (second, third) = (bounds("Edit passage 2"), bounds("Edit passage 3"));
    // The taller of a row's reply and destination ends the row; then the 16px column gap. A
    // reserved source-label line would add about 22px more.
    let row_end = second.y1.max(bounds("→ Missing Courier").y1);
    assert!(
        third.y0 - row_end <= 20.,
        "resting rows reserve nothing below themselves: {second:?} {third:?}"
    );
    // Editing reserves the label line once, at selection; typing then moves nothing below.
    let reply = mounted
        .state
        .reading
        .rows
        .iter()
        .find_map(|row| match &row.content {
            crate::reading::Content::Passage { passage, .. }
                if matches!(passage.kind, PassageKind::Choice { .. }) =>
            {
                Some(passage.id.clone())
            }
            _ => None,
        })
        .unwrap();
    mounted
        .state
        .change(|model| model.select(View::Passage(reply.clone())));
    mounted.settle();
    let selected = bounds_of(&mounted, "Edit passage 3");
    mounted.state.change(|model| {
        model.set_draft("Anything I can do to help with the relay?".into());
        Ok(())
    });
    mounted.settle();
    assert_eq!(bounds_of(&mounted, "Edit passage 3"), selected);
}

fn bounds_of<V: WidgetView<Writer, Widget = Passthrough>>(
    mounted: &Mounted<Writer, V>,
    name: &str,
) -> Rect {
    let widget = mounted.harness.get_widget_with_id(mounted.find(name));
    widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box())
}
