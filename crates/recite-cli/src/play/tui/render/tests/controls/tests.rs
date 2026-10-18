use super::*;

#[test]
fn narrow_footer_keeps_the_help_key_visible() {
    let state = TuiState {
        prompt: choice_prompt(false),
        status: "choice".to_owned(),
        key_hints: KeyHints::Contextual,
        ..TuiState::default()
    };
    let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
    let line = footer::render_footer(&state, &messages, 58);
    assert!(line.width() <= 58);
    assert!(line.to_string().ends_with('?'));
    assert!(line.to_string().contains("Enter"));
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
            interaction: TuiInteractionState::new(PromptMode::Normal).with_help(false),
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
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(false),
        },
        status: String::new(),
        key_hints: KeyHints::Compact,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 16);

    assert!(content.contains("Up/Down · y / n · Enter · ?"));
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
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(false),
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
fn deferred_queue_finished_state_renders_ready_at_end() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        deferred_queue: vec![TuiDeferredEffectRow {
            id: "effect:flag#2".to_owned(),
            function: "record_flag".to_owned(),
            args: "(mira_helped)".to_owned(),
        }],
        deferred_queue_state: Some(TuiDeferredQueueState::Ready),
        deferred_queue_expanded: true,
        prompt: TuiPrompt::Finished {
            interaction: TuiInteractionState::new(PromptMode::Finished).with_help(false),
        },
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 80, 18);

    assert!(content.contains("ready at end"));
    assert!(!content.contains("dispatched"));
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
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(true),
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
        ["Up/Down", "Enter", "an ID or number", "?", "Ctrl-C"]
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

    let enum_condition_standard = control_keys(&enum_condition_prompt(false), Keymap::Standard);
    assert_eq!(enum_condition_standard, ["Enter", "variant", "?", "Ctrl-C"]);

    let enum_condition_vim = control_keys(&enum_condition_prompt(false), Keymap::Vim);
    assert_eq!(enum_condition_vim, ["Enter", "i", "?", "Ctrl-C", ":q", ":"]);

    let effect = control_keys(&effect_prompt(false), Keymap::Standard);
    assert_eq!(effect, ["Enter", "?", "Ctrl-C"]);

    let finished = control_keys(
        &TuiPrompt::Finished {
            interaction: TuiInteractionState::new(PromptMode::Finished).with_help(false),
        },
        Keymap::Standard,
    );
    assert_eq!(finished, ["Enter/Esc/q", "?", "Ctrl-C"]);

    let help = control_keys(&condition_prompt(true), Keymap::Standard);
    assert_eq!(help, ["? / Esc", "Ctrl-C", "Up/Down", "y / n", "Enter"]);

    let enum_help = control_keys(&enum_condition_prompt(true), Keymap::Standard);
    assert_eq!(enum_help, ["? / Esc", "Ctrl-C", "Enter", "variant"]);
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
            interaction: TuiInteractionState::new(PromptMode::Insert),
            input: TextBuffer::default(),
        },
        status: "ack grant#1 with Enter".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        ..TuiState::default()
    };
    let content = render_tui_content(&state, 60, 16);

    assert!(content.contains("recite play"));
    assert!(content.contains("effect grant#1"));
    assert!(content.contains("runtime effect ID"));
    assert!(content.contains("grant#1"));
    assert!(content.contains("Press Enter to acknowledge"));
    assert!(!content.contains("Enter or ack"));
    assert!(!content.contains("Blocking Effect"));
}
