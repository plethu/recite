use crate::i18n::MsgId;
use crate::tui::{Keymap, PromptMode};

use super::super::state::{TuiPrompt, prompt_mode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ControlAvailability {
    All,
    Standard,
    Vim,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TuiControl {
    pub(super) keys: &'static str,
    pub(super) action: MsgId,
    pub(super) description: MsgId,
    pub(super) availability: ControlAvailability,
    pub(super) footer: bool,
    pub(super) compact_footer: bool,
}

impl TuiControl {
    const fn all(
        keys: &'static str,
        action: MsgId,
        description: MsgId,
        footer: bool,
        compact_footer: bool,
    ) -> Self {
        Self {
            keys,
            action,
            description,
            availability: ControlAvailability::All,
            footer,
            compact_footer,
        }
    }

    const fn standard(
        keys: &'static str,
        action: MsgId,
        description: MsgId,
        footer: bool,
        compact_footer: bool,
    ) -> Self {
        Self {
            keys,
            action,
            description,
            availability: ControlAvailability::Standard,
            footer,
            compact_footer,
        }
    }

    const fn vim(
        keys: &'static str,
        action: MsgId,
        description: MsgId,
        footer: bool,
        compact_footer: bool,
    ) -> Self {
        Self {
            keys,
            action,
            description,
            availability: ControlAvailability::Vim,
            footer,
            compact_footer,
        }
    }

    fn is_available(self, keymap: Keymap) -> bool {
        matches!(
            (self.availability, keymap),
            (ControlAvailability::All, _)
                | (ControlAvailability::Standard, Keymap::Standard)
                | (ControlAvailability::Vim, Keymap::Vim)
        )
    }
}

pub(super) fn controls_for_prompt(prompt: &TuiPrompt, keymap: Keymap) -> Vec<TuiControl> {
    let mut controls = Vec::new();
    let help_mode = prompt_mode(prompt) == PromptMode::Help;
    if help_mode {
        controls.push(TuiControl::all(
            "? / Esc",
            MsgId::TuiHelpActionClose,
            MsgId::TuiHelpDescriptionClose,
            true,
            true,
        ));
        controls.push(TuiControl::all(
            "Ctrl-C",
            MsgId::TuiHelpActionQuit,
            MsgId::TuiHelpDescriptionInterrupt,
            false,
            false,
        ));
    }
    match prompt {
        TuiPrompt::Choice { .. } => {
            controls.extend([
                TuiControl::all(
                    "Up/Down",
                    MsgId::TuiHelpActionMove,
                    MsgId::TuiHelpDescriptionMoveChoice,
                    true,
                    true,
                ),
                TuiControl::vim(
                    "j / k",
                    MsgId::TuiHelpActionMove,
                    MsgId::TuiHelpDescriptionMoveChoice,
                    true,
                    false,
                ),
                TuiControl::all(
                    "Enter",
                    MsgId::TuiHelpActionSubmit,
                    MsgId::TuiHelpDescriptionSubmitChoice,
                    true,
                    true,
                ),
                TuiControl::standard(
                    "ID/index",
                    MsgId::TuiHelpActionInput,
                    MsgId::TuiHelpDescriptionInputChoice,
                    true,
                    false,
                ),
                TuiControl::vim(
                    "i",
                    MsgId::TuiHelpActionInput,
                    MsgId::TuiHelpDescriptionInputChoice,
                    true,
                    false,
                ),
            ]);
        }
        TuiPrompt::Condition { .. } => {
            controls.extend([
                TuiControl::all(
                    "Up/Down",
                    MsgId::TuiHelpActionMove,
                    MsgId::TuiHelpDescriptionMoveCondition,
                    true,
                    true,
                ),
                TuiControl::standard(
                    "y / n",
                    MsgId::TuiHelpActionShortcut,
                    MsgId::TuiHelpDescriptionShortcutCondition,
                    true,
                    true,
                ),
                TuiControl::all(
                    "Enter",
                    MsgId::TuiHelpActionSubmit,
                    MsgId::TuiHelpDescriptionSubmitCondition,
                    true,
                    true,
                ),
            ]);
        }
        TuiPrompt::EnumCondition { .. } => {
            controls.extend([
                TuiControl::all(
                    "Enter",
                    MsgId::TuiHelpActionSubmit,
                    MsgId::TuiHelpDescriptionSubmitEnumCondition,
                    true,
                    true,
                ),
                TuiControl::standard(
                    "variant",
                    MsgId::TuiHelpActionInput,
                    MsgId::TuiHelpDescriptionInputEnumCondition,
                    true,
                    false,
                ),
                TuiControl::vim(
                    "i",
                    MsgId::TuiHelpActionInput,
                    MsgId::TuiHelpDescriptionInputEnumCondition,
                    true,
                    false,
                ),
            ]);
        }
        TuiPrompt::Effect { .. } => controls.push(TuiControl::all(
            "Enter",
            MsgId::TuiHelpActionSubmit,
            MsgId::TuiHelpDescriptionSubmitEffect,
            true,
            true,
        )),
        TuiPrompt::Finished { .. } => controls.push(TuiControl::all(
            "Enter/Esc/q",
            MsgId::TuiHelpActionQuit,
            MsgId::TuiHelpDescriptionFinished,
            true,
            true,
        )),
        TuiPrompt::None => {}
    }
    if !help_mode && !matches!(prompt, TuiPrompt::None) {
        controls.push(TuiControl::all(
            "?",
            MsgId::TuiHelpActionHelp,
            MsgId::TuiHelpDescriptionOpenHelp,
            true,
            true,
        ));
        controls.push(TuiControl::all(
            "Ctrl-C",
            MsgId::TuiHelpActionQuit,
            MsgId::TuiHelpDescriptionInterrupt,
            false,
            false,
        ));
    }
    if keymap == Keymap::Vim {
        controls.extend([
            TuiControl::vim(
                ":q",
                MsgId::TuiHelpActionQuit,
                MsgId::TuiHelpDescriptionQuit,
                true,
                false,
            ),
            TuiControl::vim(
                ":",
                MsgId::TuiHelpActionCommand,
                MsgId::TuiHelpDescriptionCommand,
                true,
                false,
            ),
        ]);
    }
    controls
        .into_iter()
        .filter(|control| control.is_available(keymap))
        .collect()
}
