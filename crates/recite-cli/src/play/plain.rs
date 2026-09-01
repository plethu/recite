use std::io::{self, Write};

use recite_core::CompiledDialogue;

use crate::dialogue_locale::DialogueTraversalPreview;
use crate::error::CliError;
use crate::i18n::Messages;

use super::plain_ui::PlainPlayUi;
use super::preview::run_preview;

pub(super) fn run_plain_stdio(
    asset: &CompiledDialogue,
    block: &str,
    stdout: &mut dyn Write,
    messages: &Messages,
    dialogue_preview: Option<DialogueTraversalPreview<'_>>,
) -> Result<(), CliError> {
    let mut stdin = io::stdin().lock();
    let mut ui = PlainPlayUi::new(&mut stdin, stdout, messages);
    run_preview(asset, block, dialogue_preview, &mut ui)
}

#[cfg(test)]
mod tests;
