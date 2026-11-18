use super::located;
use recite_writer_session::comparison::ComparisonRow;

fn row(caption: &str) -> ComparisonRow {
    ComparisonRow {
        caption: caption.into(),
        before: Some("-> old\n".into()),
        after: Some("-> new\n".into()),
    }
}

#[test]
fn review_rows_name_the_file_the_beat_and_the_line() {
    let source = ":: start default\nHello.\n-> old\n:: old\nGone.\n-> END\n";
    assert_eq!(
        located(row("scene.recite:3"), source).caption,
        "scene.recite · start · line 3"
    );
    assert_eq!(
        located(row("scene.recite:6"), source).caption,
        "scene.recite · old · line 6"
    );
    assert_eq!(
        located(row("recite.project.toml:2"), "").caption,
        "recite.project.toml · line 2"
    );
}
