use recite_core::LocaleId;
use recite_runtime::DialogueSessionOptions;

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
