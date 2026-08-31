use std::io::{self, IsTerminal, Write};

use crate::args::{PlayArgs, PlayUi};
use crate::dialogue_locale::{LoadedDialoguePreview, dialogue_preview_from_play_args};
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};
use crate::runtime_fixture::load_compiled_asset;
use crate::tui::TuiSettings;

mod choice_selection;
mod plain;
mod plain_choice;
mod plain_input;
mod plain_output;
mod plain_ui;
mod preview;
mod tui;

pub(crate) fn run_play_command(
    args: PlayArgs,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), CliError> {
    let asset = load_compiled_asset(&args.asset)?;
    let settings = TuiSettings::load(args.keymap)?;
    let messages = Messages::load(&settings.locale)?;
    let dialogue_preview =
        dialogue_preview_from_play_args(args.dialogue_locale, args.dialogue_catalog)?
            .map(LoadedDialoguePreview::load)
            .transpose()?;
    let dialogue_preview = dialogue_preview
        .as_ref()
        .map(LoadedDialoguePreview::traversal_preview);
    match resolve_ui(args.ui)? {
        ResolvedUi::Plain => {
            plain::run_plain_stdio(&asset, &args.block, stdout, &messages, dialogue_preview)
        }
        ResolvedUi::Tui => {
            if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
                return Err(CliError::PlayTuiRequiresTerminal);
            }
            writeln!(stderr, "{}", messages.text(MsgId::PlayTuiStarting))?;
            tui::run_tui_stdio(&asset, &args.block, settings, messages, dialogue_preview)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResolvedUi {
    Tui,
    Plain,
}

fn resolve_ui(ui: PlayUi) -> Result<ResolvedUi, CliError> {
    match ui {
        PlayUi::Plain => Ok(ResolvedUi::Plain),
        PlayUi::Tui => Ok(ResolvedUi::Tui),
        PlayUi::Auto => {
            if io::stdin().is_terminal() && io::stdout().is_terminal() {
                Ok(ResolvedUi::Tui)
            } else {
                Ok(ResolvedUi::Plain)
            }
        }
    }
}
