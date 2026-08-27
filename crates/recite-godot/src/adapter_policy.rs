use recite_core::LocaleId;
use recite_runtime::{
    DialogueEffectMode, DialogueEffectRequest, DialogueEvent, DialogueSessionOptions,
};

use crate::adapter::{AdapterError, AdapterErrorKind, AdapterResult};

pub(super) fn session_options(locale: Option<&str>) -> AdapterResult<DialogueSessionOptions> {
    let Some(locale) = locale.filter(|locale| !locale.is_empty()) else {
        return Ok(DialogueSessionOptions::new());
    };
    let locale = LocaleId::new(locale).map_err(|error| {
        AdapterError::with_detail(AdapterErrorKind::Localisation, error.to_string())
    })?;
    Ok(DialogueSessionOptions::new().with_locale(locale))
}

pub(super) fn should_continue_after_event(event: &DialogueEvent) -> bool {
    matches!(
        event,
        DialogueEvent::Line(_)
            | DialogueEvent::Effect(DialogueEffectRequest {
                mode: DialogueEffectMode::Immediate,
                ..
            })
    )
}
