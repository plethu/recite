use std::io::{self, IsTerminal, Write};

use crate::args::{PlayArgs, PlayUi};
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};
use crate::runtime_fixture::load_compiled_asset;
use crate::tui::TuiSettings;

mod driver;
mod format;
mod plain;
mod tui_play;
mod tui_render;
mod tui_state;

pub(crate) fn run_play_command(
    args: PlayArgs,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), CliError> {
    let asset = load_compiled_asset(&args.asset)?;
    let settings = TuiSettings::load(args.keymap)?;
    let messages = Messages::load(&settings.locale)?;
    match resolve_ui(args.ui)? {
        ResolvedUi::Plain => plain::run_plain_stdio(&asset, &args.block, stdout, &messages),
        ResolvedUi::Tui => {
            if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
                return Err(CliError::PlayTuiRequiresTerminal);
            }
            writeln!(stderr, "{}", messages.text(MsgId::PlayTuiStarting))?;
            tui_play::run_tui_stdio(&asset, &args.block, settings, messages)
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
