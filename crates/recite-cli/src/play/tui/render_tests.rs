use ratatui::Terminal;

use crate::tui::Keymap;

use super::super::state::{TuiChoiceRow, TuiPromptLine};
use super::*;

#[test]
fn transcript_entries_render_as_separated_stacked_blocks() {
    let entries = vec![
        TuiTranscriptEntry {
            kind: TuiTranscriptKind::Prompt,
            id: Some("intro_001".to_owned()),
            text: "The relay board is lit up. What do you check first?".to_owned(),
        },
        TuiTranscriptEntry {
            kind: TuiTranscriptKind::Effect,
            id: Some("effect:intro:1".to_owned()),
            text: "immediate play_sfx (panel_open)".to_owned(),
        },
        TuiTranscriptEntry {
            kind: TuiTranscriptKind::Choice,
            id: Some("ask_mira".to_owned()),
            text: String::new(),
        },
        TuiTranscriptEntry {
            kind: TuiTranscriptKind::End,
            id: None,
            text: String::new(),
        },
    ];
    let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
    let rendered = render_transcript(&entries, 40, 10, &messages);
    let debug = format!("{rendered:?}");

    assert!(debug.contains("prompt"));
    assert!(debug.contains("intro_001"));
    assert!(debug.contains("  | The relay board is lit up."));
    assert!(debug.contains("effect"));
    assert!(debug.contains("immediate play_sfx"));
    assert!(debug.contains("effect:intro:1"));
    assert!(!debug.contains("id effect:intro:1"));
    assert!(debug.contains("ask_mira"));
    assert!(!debug.contains("selected"));
    assert!(!debug.contains("end end"));
}

#[test]
fn tui_render_includes_header_and_choice_prompt() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: vec![TuiTranscriptEntry {
            kind: TuiTranscriptKind::Line,
            id: Some("intro".to_owned()),
            text: "Welcome.".to_owned(),
        }],
        prompt: TuiPrompt::Choice {
            line: Some(TuiPromptLine {
                id: "intro".to_owned(),
                text: "Welcome.".to_owned(),
            }),
            choices: vec![TuiChoiceRow {
                index: 1,
                id: "help".to_owned(),
                text: "Help.".to_owned(),
                is_available: true,
                unavailable_reason: None,
                is_visible: true,
            }],
            selected: 0,
            mode: PromptMode::Insert,
            input: TextBuffer::default(),
            command: TextBuffer::default(),
            show_help: false,
        },
        status: "choice> ".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 20);

    assert!(content.contains("recite play"));
    assert!(content.contains("asset"));
    assert!(content.contains("block"));
    assert!(content.contains("intro"));
    assert!(content.contains("Welcome."));
    assert!(content.contains("help"));
    assert!(content.contains("Help."));
    assert!(content.contains("ID/index type"));
}

#[test]
fn tui_render_finished_state_without_inactive_prompt_filler() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: vec![
            TuiTranscriptEntry {
                kind: TuiTranscriptKind::Choice,
                id: Some("help".to_owned()),
                text: "selected".to_owned(),
            },
            TuiTranscriptEntry {
                kind: TuiTranscriptKind::Line,
                id: Some("helped".to_owned()),
                text: "Helped.".to_owned(),
            },
            TuiTranscriptEntry {
                kind: TuiTranscriptKind::End,
                id: None,
                text: "end".to_owned(),
            },
        ],
        prompt: TuiPrompt::Finished { show_help: false },
        status: "finished".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 20);

    assert!(content.contains("choice"));
    assert!(content.contains("help"));
    assert!(content.contains("line"));
    assert!(content.contains("Helped."));
    assert!(content.contains("end"));
    assert!(content.contains("Enter/Esc/q quit"));
    assert!(!content.contains("No active prompt"));
}

#[test]
fn tui_render_footer_uses_effective_help_mode() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: Vec::new(),
        prompt: TuiPrompt::Condition {
            query: "trusts(player)".to_owned(),
            selected: true,
            mode: PromptMode::Insert,
            command: TextBuffer::default(),
            show_help: true,
        },
        status: "answer> ".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 20);

    assert!(content.contains("? / Esc close"));
}

#[test]
fn tui_render_condition_prompt_uses_selectable_boolean_rows() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: Vec::new(),
        prompt: TuiPrompt::Condition {
            query: "trusts(mira)".to_owned(),
            selected: false,
            mode: PromptMode::Insert,
            command: TextBuffer::default(),
            show_help: false,
        },
        status: String::new(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 16);

    assert!(content.contains("trusts(mira)"));
    assert!(content.contains("(y)es"));
    assert!(content.contains("> (n)o"));
    assert!(!content.contains("answer     "));
    assert!(!content.contains("Enter selects | y/n | ? help"));
}

#[test]
fn vim_condition_prompt_omits_standard_yes_no_shortcut_labels() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: Vec::new(),
        prompt: TuiPrompt::Condition {
            query: "trusts(mira)".to_owned(),
            selected: true,
            mode: PromptMode::Normal,
            command: TextBuffer::default(),
            show_help: false,
        },
        status: String::new(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Vim,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 16);

    assert!(content.contains("> yes"));
    assert!(content.contains("no"));
    assert!(!content.contains("(y)es"));
    assert!(!content.contains("(n)o"));
    assert!(!content.contains("y / n shortcut"));
}

#[test]
fn compact_condition_footer_uses_only_compact_control_keys() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: Vec::new(),
        prompt: TuiPrompt::Condition {
            query: "trusts(mira)".to_owned(),
            selected: true,
            mode: PromptMode::Insert,
            command: TextBuffer::default(),
            show_help: false,
        },
        status: String::new(),
        key_hints: KeyHints::Compact,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 16);

    assert!(content.contains("Up/Down | y / n | Enter | ?"));
    assert!(!content.contains("move"));
    assert!(!content.contains("submit"));
    assert!(!content.contains("help open"));
}

#[test]
fn deferred_queue_renders_only_when_expanded() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        deferred_queue: vec![TuiDeferredEffectRow {
            id: "effect:flag#2".to_owned(),
            function: "record_flag".to_owned(),
            args: "(mira_helped)".to_owned(),
        }],
        deferred_queue_state: Some(TuiDeferredQueueState::Scheduled),
        deferred_queue_expanded: true,
        prompt: TuiPrompt::Condition {
            query: "trusts(mira)".to_owned(),
            selected: true,
            mode: PromptMode::Insert,
            command: TextBuffer::default(),
            show_help: false,
        },
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 18);

    assert!(content.contains("Deferred Queue"));
    assert!(content.contains("scheduled"));
    assert!(content.contains("effect:flag#2 record_flag (mira_helped)"));
    assert!(content.contains("Ctrl-D close"));

    let collapsed = TuiState {
        deferred_queue_expanded: false,
        ..state
    };
    let content = render_tui_content(&collapsed, 80, 18);

    assert!(!content.contains("Deferred Queue"));
    assert!(content.contains("Ctrl-D queue"));
}

#[test]
fn deferred_queue_help_row_is_contextual() {
    let state = TuiState {
        deferred_queue: vec![TuiDeferredEffectRow {
            id: "effect:flag#2".to_owned(),
            function: "record_flag".to_owned(),
            args: "(mira_helped)".to_owned(),
        }],
        prompt: condition_prompt(true),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 18);

    assert!(content.contains("Ctrl-D"));
    assert!(content.contains("expand or collapse deferred effect queue"));
}

#[test]
fn tui_render_help_overlay_replaces_prompt_with_table() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: vec![TuiTranscriptEntry {
            kind: TuiTranscriptKind::Line,
            id: Some("intro".to_owned()),
            text: "Welcome.".to_owned(),
        }],
        prompt: TuiPrompt::Condition {
            query: "trusts(mira)".to_owned(),
            selected: true,
            mode: PromptMode::Insert,
            command: TextBuffer::default(),
            show_help: true,
        },
        status: "condition".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 18);

    assert!(content.contains("Help"));
    assert!(content.contains("Key"));
    assert!(content.contains("Action"));
    assert!(content.contains("Description"));
    assert!(content.contains("move between yes and no"));
    assert!(content.contains("? / Esc close"));
    assert!(!content.contains("Welcome."));
}

#[test]
fn standard_help_overlay_hides_vim_only_commands() {
    let state = choice_help_state(Keymap::Standard);
    let content = render_tui_content(&state, 80, 18);

    assert!(content.contains("Up/Down"));
    assert!(!content.contains(":q"));
    assert!(!content.contains("enter command mode"));
    assert!(!content.contains("j / k"));
    assert!(!content.contains("i           "));
}

#[test]
fn vim_help_overlay_includes_vim_only_commands() {
    let state = choice_help_state(Keymap::Vim);
    let content = render_tui_content(&state, 80, 18);

    assert!(content.contains(":q"));
    assert!(content.contains("j / k"));
    assert!(content.contains("i           "));
}

#[test]
fn condition_help_overlay_matches_keymap_shortcuts() {
    let standard = condition_help_state(Keymap::Standard);
    let vim = condition_help_state(Keymap::Vim);

    let standard_content = render_tui_content(&standard, 80, 18);
    let vim_content = render_tui_content(&vim, 80, 18);

    assert!(standard_content.contains("y / n"));
    assert!(!standard_content.contains(":q"));
    assert!(!vim_content.contains("y / n"));
    assert!(vim_content.contains(":q"));
}

#[test]
fn shared_control_filtering_matches_prompt_and_keymap() {
    let choice_standard = control_keys(&choice_prompt(false), Keymap::Standard);
    assert_eq!(
        choice_standard,
        ["Up/Down", "Enter", "ID/index", "?", "Ctrl-C"]
    );

    let choice_vim = control_keys(&choice_prompt(false), Keymap::Vim);
    assert_eq!(
        choice_vim,
        ["Up/Down", "j / k", "Enter", "i", "?", "Ctrl-C", ":q", ":"]
    );

    let condition_standard = control_keys(&condition_prompt(false), Keymap::Standard);
    assert_eq!(
        condition_standard,
        ["Up/Down", "y / n", "Enter", "?", "Ctrl-C"]
    );

    let condition_vim = control_keys(&condition_prompt(false), Keymap::Vim);
    assert_eq!(
        condition_vim,
        ["Up/Down", "Enter", "?", "Ctrl-C", ":q", ":"]
    );

    let effect = control_keys(&effect_prompt(false), Keymap::Standard);
    assert_eq!(effect, ["Enter", "?", "Ctrl-C"]);

    let finished = control_keys(&TuiPrompt::Finished { show_help: false }, Keymap::Standard);
    assert_eq!(finished, ["Enter/Esc/q", "?", "Ctrl-C"]);

    let help = control_keys(&condition_prompt(true), Keymap::Standard);
    assert_eq!(help, ["? / Esc", "Ctrl-C", "Up/Down", "y / n", "Enter"]);
}

#[test]
fn tui_render_stays_structured_on_narrow_terminal() {
    let state = TuiState {
        asset: "/tmp/recite-play.recitec".to_owned(),
        block: "start".to_owned(),
        transcript: vec![TuiTranscriptEntry {
            kind: TuiTranscriptKind::Effect,
            id: Some("grant#1".to_owned()),
            text: "blocking grant_item (map)".to_owned(),
        }],
        prompt: TuiPrompt::Effect {
            mode: "blocking".to_owned(),
            id: "grant#1".to_owned(),
            function: "grant_item".to_owned(),
            args: "(map)".to_owned(),
            input_mode: PromptMode::Insert,
            input: TextBuffer::default(),
            command: TextBuffer::default(),
            show_help: false,
        },
        status: "ack grant#1 with Enter".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 60, 16);

    assert!(content.contains("recite play"));
    assert!(content.contains("Blocking Effect"));
    assert!(content.contains("runtime effect ID"));
    assert!(content.contains("grant#1"));
    assert!(content.contains("Press Enter to acknowledge"));
    assert!(!content.contains("Enter or ack"));
}

fn choice_help_state(keymap: Keymap) -> TuiState {
    TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: Vec::new(),
        prompt: TuiPrompt::Choice {
            line: None,
            choices: vec![TuiChoiceRow {
                index: 1,
                id: "help".to_owned(),
                text: "Help.".to_owned(),
                is_available: true,
                unavailable_reason: None,
                is_visible: true,
            }],
            selected: 0,
            mode: PromptMode::Normal,
            input: TextBuffer::default(),
            command: TextBuffer::default(),
            show_help: true,
        },
        status: "choice".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap,
        ..TuiState::default()
    }
}

fn choice_prompt(show_help: bool) -> TuiPrompt {
    TuiPrompt::Choice {
        line: None,
        choices: vec![TuiChoiceRow {
            index: 1,
            id: "help".to_owned(),
            text: "Help.".to_owned(),
            is_available: true,
            unavailable_reason: None,
            is_visible: true,
        }],
        selected: 0,
        mode: PromptMode::Normal,
        input: TextBuffer::default(),
        command: TextBuffer::default(),
        show_help,
    }
}

fn condition_help_state(keymap: Keymap) -> TuiState {
    TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: Vec::new(),
        prompt: TuiPrompt::Condition {
            query: "trusts(mira)".to_owned(),
            selected: true,
            mode: PromptMode::Normal,
            command: TextBuffer::default(),
            show_help: true,
        },
        status: "condition".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap,
        ..TuiState::default()
    }
}

fn condition_prompt(show_help: bool) -> TuiPrompt {
    TuiPrompt::Condition {
        query: "trusts(mira)".to_owned(),
        selected: true,
        mode: PromptMode::Normal,
        command: TextBuffer::default(),
        show_help,
    }
}

fn effect_prompt(show_help: bool) -> TuiPrompt {
    TuiPrompt::Effect {
        mode: "blocking".to_owned(),
        id: "grant#1".to_owned(),
        function: "grant_item".to_owned(),
        args: "(map)".to_owned(),
        input_mode: PromptMode::Insert,
        input: TextBuffer::default(),
        command: TextBuffer::default(),
        show_help,
    }
}

fn control_keys(prompt: &TuiPrompt, keymap: Keymap) -> Vec<&'static str> {
    controls_for_prompt(prompt, keymap)
        .into_iter()
        .map(|control| control.keys)
        .collect()
}

fn render_tui_content(state: &TuiState, width: u16, height: u16) -> String {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");

    terminal
        .draw(|frame| render_tui(frame, state, &messages))
        .expect("draw");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}
