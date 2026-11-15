use super::*;
use crate::test_support::{finish_navigation, open, project};
use tokio::sync::mpsc as async_mpsc;

fn scene(beats: usize) -> String {
    let mut source = String::new();
    for index in 0..beats {
        let default = if index == 0 { " default" } else { "" };
        let next = if index + 1 == beats {
            "END".to_owned()
        } else {
            format!("beat{}", index + 1)
        };
        source.push_str(&format!(
            ":: beat{index}{default}\n> @b{index:06}\n  Line of beat {index}.\n-> {next}\n\n"
        ));
    }
    source
}

#[test]
fn the_galley_sets_every_beat_of_a_short_scene_and_a_window_around_the_selection_of_a_long_one()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = project(&scene(4))?;
    let writer = open(&directory)?;
    assert_eq!(writer.reading.galley(), 0..4);
    for index in 0..4 {
        assert!(
            writer
                .reading
                .beat_rows(index)
                .is_some_and(|rows| !rows.is_empty())
        );
    }
    assert!(writer.reading.beat_rows(4).is_none());
    let directory = project(&scene(40))?;
    let writer = open(&directory)?;
    let galley = writer.reading.galley();
    assert_eq!(galley.len(), GALLEY);
    assert!(galley.contains(&writer.reading.position.as_ref().unwrap().index));
    Ok(())
}

#[test]
fn beat_steps_select_the_neighbouring_beat_and_stop_at_the_ends()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = project(&scene(3))?;
    let mut writer = open(&directory)?;
    let (sender, mut jobs) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    let runtime = tokio::runtime::Runtime::new()?;
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.inspect("beat0".into());
    finish_navigation(&mut writer, &mut jobs)?;
    writer.step_beat(false);
    assert!(!writer.navigation.busy(), "no beat before the first");
    for expected in ["beat1", "beat2"] {
        writer.step_beat(true);
        finish_navigation(&mut writer, &mut jobs)?;
        assert_eq!(writer.reading.selected.as_deref(), Some(expected));
    }
    writer.step_beat(true);
    assert!(!writer.navigation.busy(), "no beat after the last");
    writer.step_beat(false);
    finish_navigation(&mut writer, &mut jobs)?;
    assert_eq!(writer.reading.selected.as_deref(), Some("beat1"));
    Ok(())
}

#[test]
fn problems_are_attributed_to_the_beat_whose_source_holds_them()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = project(
        ":: first default\n> @ffee6p8\n  Fine.\n-> second\n\n:: second\n> @rthh7ab\n  Broken.\n-> nowhere\n",
    )?;
    let writer = open(&directory)?;
    let problems = writer.beat_problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].0, "second");
    assert!(!problems[0].1.is_empty());
    Ok(())
}

#[test]
fn editing_a_read_only_beat_selects_its_first_passage_for_the_caret()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = project(&scene(3))?;
    let mut writer = open(&directory)?;
    let (sender, mut jobs) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    let runtime = tokio::runtime::Runtime::new()?;
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.edit_beat(2);
    finish_navigation(&mut writer, &mut jobs)?;
    assert_eq!(writer.reading.selected.as_deref(), Some("beat2"));
    assert_eq!(
        writer.session.model().view(),
        &recite_writer_model::View::Passage("b000002".into())
    );
    assert!(
        writer.attention.edit_request().is_some(),
        "the caret follows"
    );
    Ok(())
}
