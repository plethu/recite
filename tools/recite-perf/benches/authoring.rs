//! Compiler/schema, LSP and engine-shared import boundaries under the existing budgets.
#[path = "support/budget.rs"]
mod budget;
#[path = "support/project_build.rs"]
mod build;
#[path = "support/catalog.rs"]
mod catalog;
#[path = "support/catalog_coverage.rs"]
mod coverage;
#[path = "support/lsp.rs"]
mod lsp;
#[path = "support/lsp_change.rs"]
mod lsp_change_support;
#[path = "support/schema.rs"]
mod schema;
#[path = "support/schema_workspace.rs"]
mod schema_workspace;
#[path = "support/ui.rs"]
mod ui;
use gungraun::{library_benchmark, library_benchmark_group, main};
use lsp_types::{CompletionResponse, GotoDefinitionResponse, Location, PublishDiagnosticsParams};
use recite_core::schema::{
    SchemaLoadReport, SchemaSource, SchemaSourceLoadReport, load_schema_manifest_str,
};
use std::hint::black_box;

#[library_benchmark(teardown=schema::validate_json)]
#[bench::declarations_100(args=(100),setup=schema::json)]
#[bench::declarations_1000(args=(1_000),setup=schema::json)]
fn json_schema(input: schema::Input) -> (schema::Input, SchemaLoadReport) {
    let result = load_schema_manifest_str("schema.json", &input.text);
    black_box((input, result))
}
#[library_benchmark(teardown=schema::validate_toml)]
#[bench::declarations_100(args=(100),setup=schema::toml)]
#[bench::declarations_1000(args=(1_000),setup=schema::toml)]
fn toml_schema(input: schema::Input) -> (schema::Input, SchemaSourceLoadReport) {
    let result = SchemaSource::load_str("schema.toml", &input.text);
    black_box((input, result))
}
#[library_benchmark(teardown=catalog::validate)]
#[bench::cold_100(args=(0),setup=catalog::prepare)]
#[bench::populated_10000(args=(10_000),setup=catalog::prepare)]
fn catalog_import(mut input: catalog::Import) -> catalog::Import {
    input
        .catalog
        .import_po("fr", "incoming.po", &input.po)
        .unwrap_or_else(|error| panic!("valid import: {error}"));
    black_box(input)
}
#[library_benchmark(teardown=coverage::validate)]
#[bench::entries_100(args=(100),setup=coverage::prepare)]
#[bench::entries_1000(args=(1_000),setup=coverage::prepare)]
fn catalog_coverage(
    input: coverage::Input,
) -> (
    coverage::Input,
    recite_compiler::authoring::CatalogCoverageSummary,
) {
    let summary = coverage::summarize(&input);
    black_box((input, summary))
}
#[library_benchmark(teardown=lsp::validate_completion)]
#[bench::wide(args=(10),setup=lsp::prepare)]
#[bench::dense(args=(1_000),setup=lsp::prepare)]
fn lsp_completion(input: lsp::Input) -> (lsp::Input, Option<CompletionResponse>) {
    let result = input.driver.completion(&input.position);
    black_box((input, result))
}
#[library_benchmark(teardown=lsp::validate_definition)]
#[bench::wide(args=(10),setup=lsp::prepare)]
#[bench::dense(args=(1_000),setup=lsp::prepare)]
fn lsp_definition(input: lsp::Input) -> (lsp::Input, Option<GotoDefinitionResponse>) {
    let result = input.driver.definition(&input.position);
    black_box((input, result))
}
#[library_benchmark(teardown=lsp::validate_references)]
#[bench::wide(args=(10),setup=lsp::prepare)]
#[bench::dense(args=(1_000),setup=lsp::prepare)]
fn lsp_references(input: lsp::Input) -> (lsp::Input, Option<Vec<Location>>) {
    let result = input.driver.references(&input.position);
    black_box((input, result))
}
#[library_benchmark(teardown=lsp::validate_publication)]
#[bench::wide(args=(10),setup=lsp::prepare)]
#[bench::dense(args=(1_000),setup=lsp::prepare)]
fn lsp_publication(input: lsp::Input) -> (lsp::Input, Option<PublishDiagnosticsParams>) {
    let result = input.driver.diagnostics_publication(&input.document);
    black_box((input, result))
}
#[library_benchmark(teardown=schema_workspace::validate)]
#[bench::declarations_100(args=(100),setup=schema_workspace::prepare)]
#[bench::declarations_1000(args=(1_000),setup=schema_workspace::prepare)]
fn schema_workspace_fork(
    input: schema_workspace::Input,
) -> (
    schema_workspace::Input,
    recite_lsp::bench_support::LspBenchmarkDriver,
) {
    let fork = input.driver.fork();
    black_box((input, fork))
}
#[library_benchmark(teardown=ui::validate)]
#[bench::static_1000(setup=ui::static_labels)]
#[bench::parameters_1000(setup=ui::parameterized_labels)]
fn ui_labels(input: ui::Input) -> (ui::Input, Vec<String>) {
    black_box(ui::format(input))
}
#[library_benchmark(teardown=schema_workspace::validate_actions)]
#[bench::declarations_100(args=(100),setup=schema_workspace::prepare)]
#[bench::declarations_1000(args=(1_000),setup=schema_workspace::prepare)]
fn lsp_schema_actions(
    input: schema_workspace::Input,
) -> (
    schema_workspace::Input,
    Option<lsp_types::CodeActionResponse>,
) {
    black_box(schema_workspace::actions(input))
}
#[library_benchmark(teardown=schema_workspace::validate_change)]
#[bench::declarations_100(args=(100),setup=schema_workspace::prepare)]
#[bench::declarations_1000(args=(1_000),setup=schema_workspace::prepare)]
fn lsp_change_saved_schema(input: schema_workspace::Input) -> (schema_workspace::Input, usize) {
    black_box(schema_workspace::change(input))
}

#[library_benchmark(teardown=schema_workspace::validate_change)]
#[bench::declarations_100(args=(100),setup=schema_workspace::open_schema)]
#[bench::declarations_1000(args=(1_000),setup=schema_workspace::open_schema)]
fn lsp_change_open_schema(input: schema_workspace::Input) -> (schema_workspace::Input, usize) {
    black_box(schema_workspace::change(input))
}
#[library_benchmark(teardown=lsp_change_support::validate)]
#[bench::files_40(args=(40),setup=lsp_change_support::prepare)]
#[bench::files_400(args=(400),setup=lsp_change_support::prepare)]
fn lsp_change(mut input: lsp_change_support::Input) -> (lsp_change_support::Input, usize) {
    let count = input.driver.change_file(&input.document);
    black_box((input, count))
}
#[library_benchmark(teardown=build::validate)]
#[bench::files_40(args=(40),setup=build::prepare)]
#[bench::files_400(args=(400),setup=build::prepare)]
fn project_build(input: build::Input) -> build::Built {
    black_box(build::run(input))
}

library_benchmark_group!(name=authoring_workflows;config=budget::budget(20.0);benchmarks=json_schema,toml_schema,catalog_import,catalog_coverage,lsp_completion,lsp_definition,lsp_references,lsp_publication,schema_workspace_fork,ui_labels,lsp_schema_actions,lsp_change_saved_schema,lsp_change_open_schema,lsp_change,project_build);
main!(library_benchmark_groups = authoring_workflows);
