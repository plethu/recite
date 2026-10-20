use super::*;

#[test]
fn pane_prefix_accepts_only_exact_directions_and_normal_chords_share_command_owners() {
    for (key, command) in [
        ("h", WriterCommand::PaneLeft),
        ("j", WriterCommand::PaneDown),
        ("k", WriterCommand::PaneUp),
        ("l", WriterCommand::PaneRight),
    ] {
        assert_eq!(
            navigation(Chord::Plain(key), true),
            Some(Action::Command(command))
        );
        assert_eq!(
            navigation(Chord::Control(key), true),
            Some(Action::Command(command))
        );
    }
    for chord in [
        Chord::Other,
        Chord::Plain("H"),
        Chord::Plain("Enter"),
        Chord::Control("s"),
    ] {
        assert_eq!(navigation(chord, true), None);
    }
    assert_eq!(
        navigation(Chord::Control("W"), false),
        Some(Action::PanePrefix)
    );
    assert_eq!(
        navigation(Chord::Control("o"), false),
        Some(Action::Command(WriterCommand::Back))
    );
    assert_eq!(
        navigation(Chord::Control("i"), false),
        Some(Action::Command(WriterCommand::Forward))
    );
    assert_eq!(
        navigation(Chord::Plain("/"), false),
        Some(Action::Command(WriterCommand::Find))
    );
    assert_eq!(
        navigation(Chord::Plain("n"), false),
        Some(Action::Command(WriterCommand::NextMatch))
    );
    assert_eq!(
        navigation(Chord::Plain("N"), false),
        Some(Action::Command(WriterCommand::PreviousMatch))
    );
}
