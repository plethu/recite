use crate::buffer::ReciteBuffer;
use crate::condition::FfiContext;
use crate::error::ReciteStatus;
use crate::output::{encode_batch, encode_batch_output, should_continue};

use recite_core::CompiledDialogue;
use recite_runtime::{
    DialogueEvent, DialogueSession, InterpolationValues, LocaleResolution, next_with,
};

use crate::locale::FfiLocaleProvider;

pub(crate) fn drain_to_batch(
    dialogue: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &FfiContext<'_>,
    interpolation_values: &InterpolationValues,
    locale_provider: Option<&FfiLocaleProvider>,
    locale_variant: Option<&str>,
) -> Result<ReciteBuffer, (ReciteStatus, String)> {
    let resolution = locale_resolution(interpolation_values, locale_provider, locale_variant);
    match next_with(dialogue, session, context, resolution) {
        Ok(first_event) => drain_after_event(
            dialogue,
            session,
            context,
            first_event,
            interpolation_values,
            locale_provider,
            locale_variant,
        ),
        Err(error) if super::is_boundary_error(&error) => empty_batch(),
        Err(error) => Err((ReciteStatus::from(error.clone()), error.to_string())),
    }
}

pub(crate) fn drain_restored(
    dialogue: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &FfiContext<'_>,
    interpolation_values: &InterpolationValues,
    locale_provider: Option<&FfiLocaleProvider>,
    locale_variant: Option<&str>,
) -> Result<ReciteBuffer, (ReciteStatus, String)> {
    // After restore, a pending prompt is valid — return an empty batch rather
    // than an error so the host can re-display its own state. A pending
    // blocking effect is re-emitted once by `next_with` so the host receives
    // its stable request ID for reconciliation.
    // A restored ended session propagates NoActiveSession to the caller.
    drain_to_batch(
        dialogue,
        session,
        context,
        interpolation_values,
        locale_provider,
        locale_variant,
    )
}

pub(crate) fn drain_after_event(
    dialogue: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &FfiContext<'_>,
    first_event: DialogueEvent,
    interpolation_values: &InterpolationValues,
    locale_provider: Option<&FfiLocaleProvider>,
    locale_variant: Option<&str>,
) -> Result<ReciteBuffer, (ReciteStatus, String)> {
    let mut events = Vec::new();
    let mut current = first_event;
    loop {
        let continues = should_continue(&current);
        events.push(current);
        if !continues {
            break;
        }
        let resolution = locale_resolution(interpolation_values, locale_provider, locale_variant);
        match next_with(dialogue, session, context, resolution) {
            Ok(next_event) => current = next_event,
            Err(error) if super::is_boundary_error(&error) => break,
            Err(error) => return Err((ReciteStatus::from(error.clone()), error.to_string())),
        }
    }
    encode_batch_output(events, encode_batch)
}

fn locale_resolution<'a>(
    interpolation_values: &'a InterpolationValues,
    locale_provider: Option<&'a FfiLocaleProvider>,
    locale_variant: Option<&'a str>,
) -> LocaleResolution<'a> {
    let resolution = LocaleResolution::new().with_values(interpolation_values);
    let resolution =
        locale_provider.map_or(resolution, |provider| resolution.with_provider(provider));
    locale_variant.map_or(resolution, |variant| resolution.with_variant(variant))
}

fn empty_batch() -> Result<ReciteBuffer, (ReciteStatus, String)> {
    encode_batch_output(Vec::new(), encode_batch)
}
