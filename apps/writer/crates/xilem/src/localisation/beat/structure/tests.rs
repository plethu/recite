use super::super::*;
use crate::test_support as support;

#[test]
fn mixed_beat_keeps_conditions_effects_plural_fragments_and_continuations_in_source_order() {
    let mut source = ":: start default\n:if ready()\n".to_owned();
    for index in 0..9 {
        source.push_str(&format!("  > @line_{index}\n    Source {index}.\n"));
    }
    source.push_str("  !blocking overlay(work)\n  ? @m3ffvrt\n    Leave now?\n    -> later\n:else\n  > bind=(count:int=$count) @plural\n    One letter.\n    | Many letters.\n-> later\n:: later\n-> start\n");
    let directory = support::project(&source).unwrap();
    let writer = support::open(&directory).unwrap();
    let model = writer.session.model();
    let selected = Beat::prepare(
        None,
        model,
        &Selection {
            name: None,
            anchor: Some("m3ffvrt".into()),
            page: 0,
        },
    )
    .unwrap();
    assert!(selected.page > 0);
    let reply = selected
        .rows
        .iter()
        .filter_map(Row::passage)
        .find(|row| row.source.id == "m3ffvrt")
        .unwrap();
    assert_eq!(reply.reply, Some(1));
    assert_eq!(reply.groups.as_ref(), [":if ready()"]);
    let mut records = Vec::new();
    for page in 0..selected.total.div_ceil(PAGE) {
        let beat = Beat::prepare(
            None,
            model,
            &Selection {
                name: None,
                anchor: None,
                page,
            },
        )
        .unwrap();
        assert!(beat.rows.len() <= PAGE);
        for row in &beat.rows {
            records.push(match row {
                Row::Passage(row) => row.source.text.clone(),
                Row::Context(row) => row.pages.concat(),
            });
        }
    }
    assert_eq!(records[0], ":if ready()");
    assert_eq!(records[1], "Source 0.");
    assert_eq!(records[9], "Source 8.");
    assert_eq!(records[10], "Effect · !blocking overlay(work)");
    assert_eq!(records[11], "Leave now?");
    assert_eq!(records[12], "Otherwise");
    assert!(
        records[13].contains("Source fragment (read only)") && records[13].contains("One letter.")
    );
    assert_eq!(records[14], "Source fragment (read only) · Many letters.");
    assert_eq!(records[15], "Continue → later");
    assert_eq!(selected.incoming.links.len(), 1);
    assert_eq!(selected.incoming.links[0].origin, "later");
    assert_eq!(model.document().source(), source);
}

#[test]
fn captured_beat_selection_wins_over_the_retained_manuscript_selection() {
    let directory = support::project(":: first default\n-> END\n:: second\n-> END\n").unwrap();
    let writer = support::open(&directory).unwrap();
    let model = writer.session.model();
    assert_eq!(model.selected_block().unwrap().as_deref(), Some("first"));
    let beat = Beat::prepare(
        None,
        model,
        &Selection {
            name: Some("second".into()),
            anchor: None,
            page: 0,
        },
    )
    .unwrap();
    assert_eq!(beat.name, "second");
    assert_eq!(model.selected_block().unwrap().as_deref(), Some("first"));
}
