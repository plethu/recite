mod asset;
mod conditions;
mod effects;
mod ids;
mod markup;
mod match_arms;
mod metadata;
mod project;

use super::{DiagnosticAuxiliaryPresentationContract, DiagnosticPresentationContract};

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    asset::contracts()
        .chain(conditions::contracts())
        .chain(effects::contracts())
        .chain(ids::contracts())
        .chain(markup::contracts())
        .chain(match_arms::contracts())
        .chain(metadata::contracts())
        .chain(project::contracts())
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    asset::auxiliary_contracts()
        .chain(conditions::auxiliary_contracts())
        .chain(effects::auxiliary_contracts())
        .chain(ids::auxiliary_contracts())
        .chain(markup::auxiliary_contracts())
        .chain(match_arms::auxiliary_contracts())
        .chain(metadata::auxiliary_contracts())
        .chain(project::auxiliary_contracts())
}
