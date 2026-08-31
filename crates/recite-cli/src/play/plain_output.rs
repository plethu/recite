use std::io::Write;

use recite_core::{CompiledDialogue, EffectId};
use recite_runtime::{
    DialogueEffectRequest, DialogueLine, PreviewConditionRequest, PreviewConditionResult,
};

use crate::error::CliError;
use crate::i18n::MsgId;
use crate::runtime_format::format_effect_arguments;

use super::plain_ui::PlainPlayUi;

impl<R: ?Sized, W: Write + ?Sized> PlainPlayUi<'_, R, W> {
    pub(super) fn write_start(
        &mut self,
        asset: &CompiledDialogue,
        block: &str,
    ) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayStart,
                [
                    ("asset", asset.header.asset_id.as_str().to_owned()),
                    ("block", block.to_owned()),
                ]
            )
        )?;
        Ok(())
    }

    pub(super) fn write_line(&mut self, line: &DialogueLine) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayLine,
                [
                    ("id", line.id.as_str().to_owned()),
                    ("text", line.text.clone()),
                ]
            )
        )?;
        Ok(())
    }

    pub(super) fn write_selected_choice(
        &mut self,
        choice_id: &recite_core::ChoiceId,
    ) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlaySelectedChoice,
                [("id", choice_id.as_str().to_owned()),]
            )
        )?;
        Ok(())
    }

    pub(super) fn write_condition_result(
        &mut self,
        request: &PreviewConditionRequest,
        result: &PreviewConditionResult,
    ) -> Result<(), CliError> {
        let PreviewConditionResult::Value(value) = result else {
            return Ok(());
        };
        let query = super::plain_input::condition_query_text(request)?;
        let result = match value {
            recite_runtime::ConditionValue::Bool(value) => value.to_string(),
            recite_runtime::ConditionValue::EnumVariant(value) => value.clone(),
        };
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayConditionResult,
                [("query", query), ("result", result),]
            )
        )?;
        Ok(())
    }

    pub(super) fn write_effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayEffect,
                [
                    ("mode", effect.mode.to_string()),
                    ("id", effect.id.as_str().to_owned()),
                    ("function", effect.function.clone()),
                    ("args", format_effect_arguments(&effect.args)),
                ]
            )
        )?;
        Ok(())
    }

    pub(super) fn write_acknowledged(&mut self, effect_id: &EffectId) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayAckCompleted,
                [("id", effect_id.as_str().to_owned()),]
            )
        )?;
        Ok(())
    }

    pub(super) fn write_end(
        &mut self,
        deferred_effects: &[DialogueEffectRequest],
    ) -> Result<(), CliError> {
        writeln!(self.output, "{}", self.messages.text(MsgId::PlayEnd))?;
        if !deferred_effects.is_empty() {
            writeln!(
                self.output,
                "{}",
                self.messages.text(MsgId::PlayDeferredEffects)
            )?;
            for effect in deferred_effects {
                writeln!(
                    self.output,
                    "{}",
                    self.messages.format(
                        MsgId::PlayDeferredEffectRow,
                        [
                            ("function", effect.function.clone()),
                            ("args", format_effect_arguments(&effect.args)),
                        ]
                    )
                )?;
            }
        }
        Ok(())
    }
}
