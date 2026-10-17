use super::*;

#[test]
fn active_prompt_labels_reuse_transcript_label_styles() {
    let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
    let palette = TuiPalette::default();
    let condition = transcript::prompt_header_line(
        TuiTranscriptKind::Condition,
        Some("trusts(mira)"),
        &messages,
        palette,
    );
    let prompt = transcript::prompt_header_line(
        TuiTranscriptKind::Prompt,
        Some("intro_001"),
        &messages,
        palette,
    );
    let effect = transcript::prompt_header_line(
        TuiTranscriptKind::Effect,
        Some("effect:intro#1"),
        &messages,
        palette,
    );

    assert_eq!(
        condition.spans[0].style,
        transcript::transcript_label(TuiTranscriptKind::Condition, &messages, palette).1
    );
    assert_eq!(
        prompt.spans[0].style,
        transcript::transcript_label(TuiTranscriptKind::Prompt, &messages, palette).1
    );
    assert_eq!(
        effect.spans[0].style,
        transcript::transcript_label(TuiTranscriptKind::Effect, &messages, palette).1
    );
}

#[test]
fn colorless_render_omits_foreground_colors_and_keeps_choice_affordances() {
    let state = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: vec![
            TuiTranscriptEntry {
                kind: TuiTranscriptKind::Prompt,
                id: Some("intro".to_owned()),
                text: "Welcome.".to_owned(),
            },
            TuiTranscriptEntry {
                kind: TuiTranscriptKind::Effect,
                id: Some("grant#1".to_owned()),
                text: "blocking grant_item (map)".to_owned(),
            },
        ],
        prompt: TuiPrompt::Choice {
            line: Some(TuiPromptLine {
                id: "intro".to_owned(),
                text: "Welcome.".to_owned(),
            }),
            choices: vec![
                TuiChoiceRow {
                    index: 1,
                    id: "help".to_owned(),
                    text: "Help.".to_owned(),
                    is_available: true,
                    unavailable_reason: None,
                    is_visible: true,
                },
                TuiChoiceRow {
                    index: 2,
                    id: "locked".to_owned(),
                    text: "Locked.".to_owned(),
                    is_available: false,
                    unavailable_reason: Some("needs key".to_owned()),
                    is_visible: true,
                },
            ],
            selected: 0,
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(false),
            input: TextBuffer::default(),
        },
        status: "choice id/index> ".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        palette: TuiPalette {
            color_enabled: false,
            contrast: TuiContrast::Standard,
        },
        ..TuiState::default()
    };
    let buffer = render_tui_buffer(&state, 90, 22);
    let content = buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(buffer.content.iter().all(|cell| cell.fg == Color::Reset));
    assert!(content.contains(">  1  Help."));
    assert!(content.contains("locked"));
    assert!(content.contains("unavailable: needs key"));
    assert!(content.contains("prompt intro"));
    assert!(content.contains("effect grant#1"));
    assert!(content.contains("choice id/index>"));
    assert!(content.contains("type an ID or number"));
}

#[test]
fn colorless_condition_and_effect_prompts_keep_textual_controls() {
    let condition = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        prompt: TuiPrompt::Condition {
            query: "trusts(mira)".to_owned(),
            selected: false,
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(false),
        },
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        palette: TuiPalette {
            color_enabled: false,
            contrast: TuiContrast::Standard,
        },
        ..TuiState::default()
    };
    let condition_content = render_tui_content(&condition, 80, 16);
    assert!(condition_content.contains("(y)es"));
    assert!(condition_content.contains("> (n)o"));

    let effect = TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        prompt: TuiPrompt::Effect {
            mode: "blocking".to_owned(),
            id: "grant#1".to_owned(),
            function: "grant_item".to_owned(),
            args: "(map)".to_owned(),
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(false),
            input: TextBuffer::default(),
        },
        key_hints: KeyHints::Contextual,
        keymap: Keymap::Standard,
        palette: TuiPalette {
            color_enabled: false,
            contrast: TuiContrast::Standard,
        },
        ..TuiState::default()
    };
    let effect_content = render_tui_content(&effect, 80, 18);
    assert!(effect_content.contains("effect grant#1"));
    assert!(effect_content.contains("runtime effect ID"));
    assert!(effect_content.contains("function"));
    assert!(effect_content.contains("Press Enter to acknowledge"));
}

#[test]
fn accessible_contrast_uses_alternate_palette_when_color_is_enabled() {
    let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
    let standard = TuiPalette {
        color_enabled: true,
        contrast: TuiContrast::Standard,
    };
    let accessible = TuiPalette {
        color_enabled: true,
        contrast: TuiContrast::Accessible,
    };

    assert_ne!(
        transcript::transcript_label(TuiTranscriptKind::Choice, &messages, standard)
            .1
            .fg,
        transcript::transcript_label(TuiTranscriptKind::Choice, &messages, accessible)
            .1
            .fg
    );
    assert_eq!(
        transcript::transcript_label(TuiTranscriptKind::Choice, &messages, accessible)
            .1
            .fg,
        Some(Color::Gray)
    );
}
