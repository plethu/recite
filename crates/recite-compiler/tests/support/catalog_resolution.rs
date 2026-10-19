use recite_compiler::authoring::{
    CatalogCoverageSummary, CatalogInput, CatalogResolution, CatalogResolutionPolicy,
    CatalogSummaryError,
};
use recite_core::po::PotDocument;

pub(super) fn build(
    expected: &PotDocument,
    inputs: impl IntoIterator<Item = CatalogInput>,
    policy: CatalogResolutionPolicy,
) -> Result<CatalogCoverageSummary, CatalogSummaryError> {
    let inputs = inputs.into_iter().collect::<Vec<_>>();
    let resolution = CatalogResolution::build(expected, inputs.clone(), policy.clone());
    let summary = CatalogCoverageSummary::build(expected, inputs, policy);
    assert_eq!(
        summary.as_ref().map(CatalogCoverageSummary::resolution),
        resolution.as_ref()
    );
    summary
}
