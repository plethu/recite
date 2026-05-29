use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

#[test]
fn text_buffer_edits_with_cursor_awareness() {
    let mut buffer = TextBuffer::default();
    buffer.insert('a');
    buffer.insert('c');
    buffer.move_left();
    buffer.insert('b');
    assert_eq!(buffer.as_str(), "abc");
    buffer.delete_word_before_cursor();
    assert_eq!(buffer.as_str(), "c");
    buffer.move_end();
    buffer.backspace();
    assert_eq!(buffer.as_str(), "");
}

#[test]
fn text_buffer_applies_text_edit_intents() {
    let mut buffer = TextBuffer::default();
    buffer.apply_intent(TuiIntent::Text('a'));
    buffer.apply_intent(TuiIntent::Text('b'));
    buffer.apply_intent(TuiIntent::MoveCursorLeft);
    buffer.apply_intent(TuiIntent::Text('x'));
    buffer.apply_intent(TuiIntent::DeleteWord);

    assert_eq!(buffer.as_str(), "b");
}

#[test]
fn maps_standard_printable_keys_to_text() {
    let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        map_key(Keymap::Standard, PromptMode::Normal, key),
        TuiIntent::Text('j')
    );
}

#[test]
fn standard_colon_does_not_enter_command_mode() {
    let key = KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE);
    assert_eq!(
        map_key(Keymap::Standard, PromptMode::Normal, key),
        TuiIntent::Text(':')
    );
}

#[test]
fn vim_colon_enters_command_mode() {
    let key = KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE);
    assert_eq!(
        map_key(Keymap::Vim, PromptMode::Normal, key),
        TuiIntent::OpenCommand
    );
}

#[test]
fn condition_yes_no_shortcuts_are_standard_only_in_normal_mode() {
    let yes = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
    let no = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);

    assert_eq!(
        map_key(Keymap::Standard, PromptMode::Normal, yes),
        TuiIntent::Text('y')
    );
    assert_eq!(
        map_key(Keymap::Standard, PromptMode::Normal, no),
        TuiIntent::Text('n')
    );
    assert_eq!(
        map_key(Keymap::Vim, PromptMode::Normal, yes),
        TuiIntent::Ignore
    );
    assert_eq!(
        map_key(Keymap::Vim, PromptMode::Normal, no),
        TuiIntent::Ignore
    );
}

#[test]
fn control_d_toggles_auxiliary_panel() {
    let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key(Keymap::Standard, PromptMode::Insert, key),
        TuiIntent::ToggleAuxiliaryPanel
    );
}

#[test]
fn help_enter_maps_to_submit_for_context_specific_handling() {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        map_key(Keymap::Standard, PromptMode::Help, key),
        TuiIntent::Submit
    );
}

#[test]
fn maps_vim_navigation_keys_in_normal_mode() {
    let down = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let up = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(
        map_key(Keymap::Vim, PromptMode::Normal, down),
        TuiIntent::MoveNext
    );
    assert_eq!(
        map_key(Keymap::Vim, PromptMode::Normal, up),
        TuiIntent::MovePrevious
    );
}

#[test]
fn command_parser_accepts_quit_commands_only() {
    assert!(command_quits("q"));
    assert!(command_quits("quit"));
    assert!(!command_quits("write"));
}

#[test]
fn global_actions_classify_shared_intents() {
    assert_eq!(
        global_action(PromptMode::Normal, TuiIntent::Quit),
        Some(GlobalAction::Quit)
    );
    assert_eq!(
        global_action(PromptMode::Normal, TuiIntent::OpenCommand),
        Some(GlobalAction::OpenCommand)
    );
    assert_eq!(
        global_action(PromptMode::Normal, TuiIntent::ToggleAuxiliaryPanel),
        Some(GlobalAction::ToggleAuxiliaryPanel)
    );
    assert_eq!(
        global_action(PromptMode::Help, TuiIntent::Cancel),
        Some(GlobalAction::CloseHelp)
    );
    assert_eq!(global_action(PromptMode::Insert, TuiIntent::Cancel), None);
}

#[test]
fn interaction_state_tracks_help_and_command_modes() {
    let mut interaction = TuiInteractionState::new(PromptMode::Normal);

    interaction.toggle_help();
    assert_eq!(interaction.effective_mode(), PromptMode::Help);

    interaction.close_help();
    assert_eq!(interaction.effective_mode(), PromptMode::Normal);

    interaction.start_command();
    interaction.mutate_command(TuiIntent::Text('q'));
    assert_eq!(interaction.effective_mode(), PromptMode::Command);
    assert_eq!(interaction.command(), "q");
}

#[test]
fn settings_default_to_standard_contextual_hints_and_visible_unavailable_choices() {
    let settings = TuiSettings::default();

    assert_eq!(settings.keymap, Keymap::Standard);
    assert_eq!(settings.key_hints, KeyHints::Contextual);
    assert_eq!(settings.locale.to_string(), crate::i18n::DEFAULT_LOCALE);
    assert!(settings.show_unavailable_choices);
}

#[test]
fn settings_parse_toml_config() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("config.toml");
    fs::write(
        &path,
        r#"[ui]
locale = "en-GB"
keymap = "vim"
key_hints = "compact"

[play]
show_unavailable_choices = false
"#,
    )
    .expect("write config");

    let settings = TuiSettings::load_path(&path).expect("config loads");

    assert_eq!(settings.locale.to_string(), "en-GB");
    assert_eq!(settings.keymap, Keymap::Vim);
    assert_eq!(settings.key_hints, KeyHints::Compact);
    assert!(!settings.show_unavailable_choices);
}

#[test]
fn settings_reject_unknown_values() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("config.toml");
    fs::write(
        &path,
        r#"[ui]
keymap = "emacs"
"#,
    )
    .expect("write config");

    let error = TuiSettings::load_path(&path).expect_err("config fails");

    assert!(error.to_string().contains("failed to parse UI config"));
}

#[test]
fn settings_reject_malformed_locale() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("config.toml");
    fs::write(
        &path,
        r#"[ui]
locale = "not a locale"
"#,
    )
    .expect("write config");

    let error = TuiSettings::load_path(&path).expect_err("config fails");

    assert!(error.to_string().contains("invalid [ui].locale"));
}
