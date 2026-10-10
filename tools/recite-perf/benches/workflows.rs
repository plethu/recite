//! Fixed small stress and authored realistic corpora; preparation is excluded.
mod support;

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use recite_benchmarks::compiler::{CompiledProject, CompilerProject, serialize_compiled_asset};
use recite_benchmarks::preview::{PreviewEvidenceReport, PreviewProject};
use recite_benchmarks::runtime::RuntimeProject;
use recite_benchmarks::{BenchmarkFixture, BenchmarkScale};
use recite_core::compiled::{CompiledDialogue, decode_compiled_dialogue_messagepack};
use support::{
    Steps, budget, bytes, compiled, compiler, line, localised_line, must, prompt, runtime,
};

#[library_benchmark]
#[bench::small(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Small)), setup = compiler)]
#[bench::realistic(args = (BenchmarkFixture::RealisticV1Pack), setup = compiler)]
#[bench::medium(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Medium)), setup = compiler)]
fn compile(project: CompilerProject) -> (CompilerProject, CompiledProject) {
    let result = must(project.compile_with_schema());
    black_box((project, result))
}

#[library_benchmark]
#[bench::small(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Small)), setup = compiled)]
#[bench::realistic(args = (BenchmarkFixture::RealisticV1Pack), setup = compiled)]
#[bench::medium(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Medium)), setup = compiled)]
fn encode(project: CompiledProject) -> (CompiledProject, Vec<u8>) {
    let result = must(serialize_compiled_asset(&project));
    black_box((project, result))
}

#[library_benchmark]
#[bench::small(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Small)), setup = bytes)]
#[bench::realistic(args = (BenchmarkFixture::RealisticV1Pack), setup = bytes)]
#[bench::medium(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Medium)), setup = bytes)]
fn decode(bytes: Vec<u8>) -> (Vec<u8>, CompiledDialogue) {
    let result = decode_compiled_dialogue_messagepack(&bytes)
        .unwrap_or_else(|error| panic!("validated benchmark asset failed: {error}"));
    black_box((bytes, result))
}

#[library_benchmark(teardown = support::validate_steps)]
#[bench::small(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Small)), setup = line)]
#[bench::realistic(args = (BenchmarkFixture::RealisticV1Pack), setup = line)]
#[bench::medium(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Medium)), setup = line)]
fn next_line((project, mut sessions, mut events): Steps) -> Steps {
    for session in &mut sessions {
        events.push(must(project.driver().next_line(session)));
    }
    black_box((project, sessions, events))
}

#[library_benchmark(teardown = support::validate_steps)]
#[bench::small(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Small)), setup = localised_line)]
#[bench::realistic(args = (BenchmarkFixture::RealisticV1Pack), setup = localised_line)]
#[bench::medium(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Medium)), setup = localised_line)]
fn localised_next((project, mut sessions, mut events): Steps) -> Steps {
    for session in &mut sessions {
        events.push(must(project.driver().localised_next(session)));
    }
    black_box((project, sessions, events))
}

#[library_benchmark(teardown = support::validate_steps)]
#[bench::small(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Small)), setup = prompt)]
#[bench::realistic(args = (BenchmarkFixture::RealisticV1Pack), setup = prompt)]
#[bench::medium(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Medium)), setup = prompt)]
fn choose((project, mut sessions, mut events): Steps) -> Steps {
    for session in &mut sessions {
        events.push(must(project.driver().choose_first(session)));
    }
    black_box((project, sessions, events))
}

#[library_benchmark]
#[bench::small(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Small)), setup = runtime)]
#[bench::realistic(args = (BenchmarkFixture::RealisticV1Pack), setup = runtime)]
#[bench::medium(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Medium)), setup = runtime)]
fn traverse(project: RuntimeProject) -> (RuntimeProject, usize) {
    let count = must(project.driver().full_traversal());
    black_box((project, count))
}

fn preview(fixture: BenchmarkFixture) -> PreviewProject {
    must(PreviewProject::load(fixture))
}

#[library_benchmark]
#[bench::small(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Small)), setup = preview)]
#[bench::realistic(args = (BenchmarkFixture::RealisticV1Pack), setup = preview)]
#[bench::medium(args = (BenchmarkFixture::Synthetic(BenchmarkScale::Medium)), setup = preview)]
fn preview_recovery(project: PreviewProject) -> (PreviewProject, PreviewEvidenceReport) {
    let evidence = must(project.evidence_report());
    assert!(
        evidence.restore.events_match,
        "restore changed subsequent events"
    );
    black_box((project, evidence))
}

library_benchmark_group!(name = compiler_workflows; config = budget(20.0); benchmarks = compile, encode, decode);
library_benchmark_group!(name = runtime_workflows; config = budget(10.0); benchmarks = next_line, localised_next, choose, traverse);
library_benchmark_group!(name = preview_workflows; config = budget(20.0); benchmarks = preview_recovery);
main!(
    library_benchmark_groups = compiler_workflows,
    runtime_workflows,
    preview_workflows
);
