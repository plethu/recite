use super::*;
use crate::{
    keyed::keyed,
    tests::support::{mounted::Mounted, open, project},
};
use recite_config::WriterPresentationField;
use xilem::masonry::{
    kurbo::Rect,
    widgets::{Label, Passthrough},
};

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
fn destinations_share_one_left_aligned_column_in_the_grid() {
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
    // The reply numbers count the replies; no separate count is spelled out.
    assert!(!mounted.contains_text("4 replies"));
    // The file and beat identity belong to the folio, not a second line under the title.
    assert!(!mounted.contains_text("relay_hub.recite  ::  relay_desk"));
}

#[test]
fn replies_keep_the_column_rhythm_and_never_move_while_one_is_edited() {
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
    // The taller of a row's reply and destination ends the row; then its id's line and the
    // column gap.
    let row_end = second.y1.max(bounds("→ Missing Courier").y1);
    assert!(
        third.y0 - row_end <= 64.,
        "rows keep one id line below themselves: {second:?} {third:?}"
    );
    // Typing never moves a row below.
    let resting = bounds_of(&mounted, "Edit passage 3");
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
    // Every row keeps its id's line, so selecting a reply moves nothing.
    let selected = bounds_of(&mounted, "Edit passage 3");
    assert_eq!(selected, resting);
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

/// The bounds of each label reading `text`, top to bottom.
fn text_bounds<V: WidgetView<Writer, Widget = Passthrough>>(
    mounted: &Mounted<Writer, V>,
    text: &str,
) -> Vec<Rect> {
    let mut found = Vec::new();
    let mut pending = vec![mounted.harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if !widget.ctx().is_stashed()
            && widget
                .downcast::<Label>()
                .is_some_and(|label| label.inner().text().as_ref() == text)
        {
            found.push(
                widget
                    .ctx()
                    .window_transform()
                    .transform_rect_bbox(widget.ctx().border_box()),
            );
        }
        pending.extend(widget.children());
    }
    found.sort_by(|a, b| a.y0.total_cmp(&b.y0));
    found
}

#[test]
fn the_speaker_label_and_destination_are_the_controls_in_their_own_places() {
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
    let select = |mounted: &mut Mounted<Writer, _>, passage: &str, line: &str| {
        mounted
            .state
            .change(|model| model.select(View::Passage(passage.into())));
        mounted.settle();
        (
            text_bounds(mounted, "MARA")[0],
            text_bounds(mounted, "Missing Courier")[0],
            bounds_of(mounted, line),
        )
    };
    let resting = select(&mut mounted, "ask_station", "Edit passage 1");
    let speaking = select(&mut mounted, "desk_greeting", "Manuscript");
    // The label's words and the line keep their places, to the pixel.
    assert_eq!((speaking.0.x1, speaking.0.y0), (resting.0.x1, resting.0.y0));
    assert_eq!(speaking.2, resting.2);
    let control = bounds_of(&mounted, "Change speaker, mara…");
    assert!(control.contains(speaking.0.center()), "{control:?}");
    assert!(
        control.width() >= 24. && control.height() >= 24.,
        "{control:?}"
    );
    // No separate speaker control waits after the beat.
    assert!(!mounted.contains_text("Speaker…"));
    let replying = select(&mut mounted, "ask_work", "Manuscript");
    assert_eq!(replying.1, resting.1);
    let control = bounds_of(&mounted, "Change destination, Missing Courier…");
    assert!(control.contains(replying.1.center()), "{control:?}");
    assert!(control.height() >= 24., "{control:?}");
    assert!(!mounted.contains_text("Destination…"));
}

#[test]
fn a_beat_s_continuation_is_the_route_on_its_last_line_s_row() {
    let source = ":: start default\n> speaker=mara @ffee6p8\n  Hello.\n> speaker=mara @rthh7ab\n  Again.\n-> next\n:: next\n> @z0g6c80\n  Goodbye.\n-> END\n";
    let directory = project(source).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.viewport.width = 1200.;
    let properties = crate::presentation::Theme::new(&writer.config.writer).properties;
    let mut mounted = Mounted::new(writer, surface);
    mounted.harness.set_default_properties(properties);
    mounted
        .state
        .change(|model| model.select(View::Passage("ffee6p8".into())));
    mounted.settle();
    let line = bounds_of(&mounted, "Edit passage 2");
    // The route names what it changes and where the beat goes now; it is the only control.
    let route = bounds_of(&mounted, "Change continuation, Next…");
    assert!(route.x0 >= line.x1 - 1., "{route:?} beside {line:?}");
    assert!(
        route.y0 < line.y1 && route.y1 > line.y0,
        "{route:?} on the row of {line:?}"
    );
    assert!(route.height() >= 24., "{route:?}");
}

#[test]
fn only_a_beat_that_ends_the_conversation_shows_the_end_rule() {
    // Start's only reply leads on, so it never ends there; next has no jump and ends.
    let source = ":: start default\n> @ffee6p8\n  Hello.\n? @rthh7ab\n  Go on.\n  -> next\n:: next\n> @z0g6c80\n  Goodbye.\n";
    let directory = project(source).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.viewport.width = 1200.;
    let mut mounted = Mounted::new(writer, surface);
    mounted.settle();
    assert_eq!(text_bounds(&mounted, "END").len(), 1);
}

#[test]
fn a_written_row_s_id_has_a_row_of_its_own_at_every_scale() {
    for scale in [100, 200] {
        let mut writer = Writer::from_session(crate::session::Session::examples().unwrap(), None);
        writer
            .follow_location(&recite_writer_session::location::Location {
                document: "relay_hub.recite".into(),
                ..Default::default()
            })
            .unwrap();
        writer.viewport.width = 1200.;
        let mut config = writer.config.clone();
        config.writer.presentation = config
            .writer
            .presentation
            .with_value(WriterPresentationField::UiScale, scale)
            .unwrap();
        writer.configure(config);
        let properties = crate::presentation::Theme::new(&writer.config.writer).properties;
        let mut mounted = Mounted::new(writer, surface);
        mounted.harness.set_default_properties(properties);
        mounted.settle();
        let passages = mounted
            .state
            .reading
            .rows
            .iter()
            .filter_map(|row| match &row.content {
                crate::reading::Content::Passage { passage, .. } => Some(passage.id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for passage in passages {
            mounted
                .state
                .change(|model| model.select(View::Passage(passage.clone())));
            mounted.settle();
            let id = format!("{STABLE_ID_FIELD_PREFIX}{passage}");
            let [caption] = text_bounds(&mounted, &id)[..] else {
                panic!("one id for {passage} at {scale}%");
            };
            for other in row_bounds(&mounted, &id) {
                let overlap = caption.intersect(other);
                assert!(
                    other.contains(caption.center()) && other.area() > 4. * caption.area()
                        || overlap.width() <= 0.
                        || overlap.height() <= 0.,
                    "{id} at {scale}%: {caption:?} meets {other:?}"
                );
            }
        }
    }
}

/// The bounds of every label but `except` and every focusable control: the rows an id must clear.
fn row_bounds<V: WidgetView<Writer, Widget = Passthrough>>(
    mounted: &Mounted<Writer, V>,
    except: &str,
) -> Vec<Rect> {
    let mut found = Vec::new();
    let mut pending = vec![mounted.harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() {
            continue;
        }
        let label = widget
            .downcast::<Label>()
            .map(|label| label.inner().text().as_ref() != except);
        if label.unwrap_or_else(|| widget.ctx().accepts_focus()) {
            found.push(
                widget
                    .ctx()
                    .window_transform()
                    .transform_rect_bbox(widget.ctx().border_box()),
            );
        }
        pending.extend(widget.children());
    }
    found
}
