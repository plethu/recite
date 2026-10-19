use super::*;

#[test]
fn transcript_entries_render_as_separated_stacked_blocks() {
    let entries = [
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
    let rendered =
        transcript::render_transcript(entries.iter(), 40, 10, &messages, TuiPalette::default());
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
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(false),
            input: TextBuffer::default(),
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
    assert!(content.contains("type an ID or number"));
}

#[test]
fn active_choice_prompt_uses_transcript_prompt_header_and_wrapped_text() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        prompt: TuiPrompt::Choice {
            line: Some(TuiPromptLine {
                id: "intro_001".to_owned(),
                text: "The relay board is lit up. What do you check first?".to_owned(),
            }),
            choices: vec![TuiChoiceRow {
                index: 1,
                id: "ask_mira".to_owned(),
                text: "Ask Mira.".to_owned(),
                is_available: true,
                unavailable_reason: None,
                is_visible: true,
            }],
            selected: 0,
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(false),
            input: TextBuffer::default(),
        },
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 18);

    assert!(content.contains("prompt intro_001"));
    assert!(content.contains("| The relay board is lit up. What do you check first?"));
    assert!(content.contains(">  1  Ask Mira."));
    assert!(content.contains("ask_mira"));
    assert!(!content.contains("Choose a branch"));
}

#[test]
fn wrapped_choice_prompt_allocates_height_for_visible_choices() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        prompt: TuiPrompt::Choice {
            line: Some(TuiPromptLine {
                id: "intro_001".to_owned(),
                text: "The relay board is lit up beside a blinking maintenance console while Mira waits for a quick decision.".to_owned(),
            }),
            choices: vec![
                TuiChoiceRow {
                    index: 1,
                    id: "inspect_panel".to_owned(),
                    text: "Inspect the relay panel.".to_owned(),
                    is_available: true,
                    unavailable_reason: None,
                    is_visible: true,
                },
                TuiChoiceRow {
                    index: 2,
                    id: "ask_mira".to_owned(),
                    text: "Ask Mira.".to_owned(),
                    is_available: true,
                    unavailable_reason: None,
                    is_visible: true,
                },
                TuiChoiceRow {
                    index: 3,
                    id: "leave".to_owned(),
                    text: "Leave.".to_owned(),
                    is_available: true,
                    unavailable_reason: None,
                    is_visible: true,
                },
            ],
            selected: 0,
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(false),
            input: TextBuffer::default(),
        },
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 36, 28);

    assert!(content.contains("| The relay board is lit up beside"));
    assert!(content.contains("| a blinking maintenance console"));
    assert!(content.contains("  3  Leave."));
    assert!(content.contains("leave"));
    assert!(content.contains("Leave."));
}

#[test]
fn typed_choice_input_renders_only_in_status_footer() {
    let mut input = TextBuffer::default();
    for ch in "ask_mira".chars() {
        input.insert(ch);
    }
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        prompt: TuiPrompt::Choice {
            line: Some(TuiPromptLine {
                id: "intro_001".to_owned(),
                text: "The relay board is lit up.".to_owned(),
            }),
            choices: vec![TuiChoiceRow {
                index: 1,
                id: "inspect_panel".to_owned(),
                text: "Inspect the relay panel.".to_owned(),
                is_available: true,
                unavailable_reason: None,
                is_visible: true,
            }],
            selected: 0,
            interaction: TuiInteractionState::new(PromptMode::Insert),
            input,
        },
        status: "choice id/index> ask_mira".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 18);

    assert!(content.contains("choice id/index> ask_mira"));
    assert_eq!(content.matches("ask_mira").count(), 1);
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
        prompt: TuiPrompt::Finished {
            interaction: TuiInteractionState::new(PromptMode::Finished).with_help(false),
        },
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
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(true),
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
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(false),
        },
        status: String::new(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 16);

    assert!(content.contains("condition trusts(mira)"));
    assert!(content.contains("trusts(mira)"));
    assert!(content.contains("(y)es"));
    assert!(content.contains("> (n)o"));
    assert!(!content.contains("Condition"));
    assert!(!content.contains("answer     "));
    assert!(!content.contains("Enter selects | y/n | ? help"));
}

#[test]
fn tui_render_enum_condition_prompt_shows_query_and_variant_input() {
    let mut input = TextBuffer::default();
    for ch in "high".chars() {
        input.insert(ch);
    }
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: Vec::new(),
        prompt: TuiPrompt::EnumCondition {
            query: "memory_pressure(hazel, music_shop)".to_owned(),
            interaction: TuiInteractionState::new(PromptMode::Insert),
            input,
        },
        status: "enum variant> high".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 16);

    assert!(content.contains("condition memory_pressure(hazel, music_shop)"));
    assert!(content.contains("Type an enum variant and press Enter."));
    assert!(content.contains("variant high"));
    assert!(content.contains("enum variant> high"));
    assert!(content.contains("variant type"));
    assert!(!content.contains("prompt"));
    assert!(!content.contains("(y)es"));
    assert!(!content.contains("(n)o"));
    assert!(!content.contains("ID/index"));
}
