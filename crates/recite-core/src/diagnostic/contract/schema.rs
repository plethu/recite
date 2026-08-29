use super::DiagnosticPresentationContract;

macro_rules! schema_contract {
    ($name:ident, $code:literal, $id:literal, [$($arg:literal => $kind:ident),* $(,)?]) => {
        const $name: $crate::DiagnosticPresentationContract =
            $crate::DiagnosticPresentationContract::new(
            $code,
            $id,
            &[$($crate::DiagnosticArgumentSpec::new(
                $arg,
                $crate::DiagnosticArgumentType::$kind,
            )),*],
        );
    };
}

pub(super) use schema_contract;

mod availability;
mod manifest;
mod projection;
mod source;

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    source::contracts()
        .chain(manifest::contracts())
        .chain(availability::contracts())
        .chain(projection::contracts())
}
