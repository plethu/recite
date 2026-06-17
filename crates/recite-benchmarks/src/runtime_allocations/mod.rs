use std::alloc::System;
use std::hint::black_box;

use recite_runtime::{DialogueEvent, DialogueSession};
use serde::Serialize;
use stats_alloc::{Region, Stats, StatsAlloc};

use crate::compiler::CompilerProject;
use crate::project::BenchmarkProject;
use crate::runtime::{RuntimeProject, TraversalDriver};
use crate::{BenchmarkFixture, BenchmarkResult};

mod markdown;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeAllocationOptions {
    fixtures: Vec<BenchmarkFixture>,
}

impl RuntimeAllocationOptions {
    #[must_use]
    pub fn new(fixtures: Vec<BenchmarkFixture>) -> Self {
        Self { fixtures }
    }

    #[must_use]
    pub fn fixtures(&self) -> &[BenchmarkFixture] {
        &self.fixtures
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeAllocationReport {
    pub generated_by: &'static str,
    pub fixtures: Vec<FixtureRuntimeAllocationReport>,
    pub caveats: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FixtureRuntimeAllocationReport {
    pub fixture: &'static str,
    pub operations: Vec<RuntimeAllocationOperation>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeAllocationOperation {
    pub operation: &'static str,
    pub stats: RuntimeAllocationStats,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct RuntimeAllocationStats {
    pub allocations: usize,
    pub bytes_allocated: usize,
    pub reallocations: usize,
    pub bytes_reallocated: isize,
    pub deallocations: usize,
    pub bytes_deallocated: usize,
}

pub fn build_runtime_allocation_report(
    options: &RuntimeAllocationOptions,
    allocator: &StatsAlloc<System>,
) -> BenchmarkResult<RuntimeAllocationReport> {
    let mut fixtures = Vec::with_capacity(options.fixtures().len());
    for fixture in options.fixtures() {
        fixtures.push(build_fixture_report(*fixture, allocator)?);
    }

    Ok(RuntimeAllocationReport {
        generated_by: "runtime_allocation_report",
        fixtures,
        caveats: vec![
            "Counts come from an instrumented process-global allocator and can vary by platform, build profile, allocator, and surrounding process activity.",
            "Each operation measures the runtime hot-path body after fixture, asset, and setup-session preparation where possible.",
            "Returned events, sessions, and buffers are dropped after the measured region, so deallocation counts do not necessarily mirror allocation counts.",
            "Clone pressure is inferred from allocation and byte spikes; this report does not count individual Clone calls.",
            "Thresholds remain review evidence only until #169 establishes the release benchmark baseline profile.",
        ],
    })
}

fn build_fixture_report(
    fixture: BenchmarkFixture,
    allocator: &StatsAlloc<System>,
) -> BenchmarkResult<FixtureRuntimeAllocationReport> {
    let project = BenchmarkProject::load_fixture(fixture)?;
    let compiler = CompilerProject::load(&project)?;
    let compiled = compiler.compile_with_schema()?;
    let runtime = RuntimeProject::load(&project, &compiled)?;
    let driver = runtime.driver();

    Ok(FixtureRuntimeAllocationReport {
        fixture: project.fixture_label(),
        operations: runtime_operations(&driver, allocator)?,
    })
}

fn runtime_operations(
    driver: &TraversalDriver<'_>,
    allocator: &StatsAlloc<System>,
) -> BenchmarkResult<Vec<RuntimeAllocationOperation>> {
    let encoded_prompt_session = driver.encoded_prompt_session()?;
    Ok(vec![
        measure_operation(allocator, "start_scene", || driver.start_scene())?,
        measure_with_session(
            allocator,
            "next_line",
            driver.session_before_first_line()?,
            |session| driver.next_line(session),
        )?,
        measure_with_session(
            allocator,
            "next_prompt",
            driver.session_before_first_prompt()?,
            |session| driver.next_prompt(session),
        )?,
        measure_with_session(
            allocator,
            "choose_first",
            driver.session_with_prompt()?,
            |session| driver.choose_first(session),
        )?,
        measure_with_session(
            allocator,
            "condition_dispatch",
            driver.session_before_condition_prompt()?,
            |session| driver.condition_dispatch(session),
        )?,
        measure_with_session(
            allocator,
            "effect_immediate",
            driver.start_scene()?,
            |session| driver.immediate_effect(session),
        )?,
        measure_with_session(
            allocator,
            "effect_deferred",
            driver.session_before_deferred_effect()?,
            |session| driver.deferred_effect(session),
        )?,
        measure_with_session(
            allocator,
            "effect_blocking",
            driver.session_before_blocking_effect()?,
            |session| driver.blocking_effect(session),
        )?,
        measure_blocking_ack(allocator, driver)?,
        measure_with_session(
            allocator,
            "localised_next",
            driver.localised_session_before_first_line()?,
            |session| driver.localised_next(session),
        )?,
        measure_with_session(
            allocator,
            "session_encode",
            driver.session_with_prompt()?,
            |session| driver.encode_session(session),
        )?,
        measure_operation(allocator, "session_decode", || {
            driver.decode_session(&encoded_prompt_session)
        })?,
        measure_operation(allocator, "full_traversal", || driver.full_traversal())?,
    ])
}

fn measure_with_session<T>(
    allocator: &StatsAlloc<System>,
    operation: &'static str,
    mut session: DialogueSession,
    measure: impl FnOnce(&mut DialogueSession) -> BenchmarkResult<T>,
) -> BenchmarkResult<RuntimeAllocationOperation> {
    measure_operation(allocator, operation, || measure(&mut session))
}

fn measure_blocking_ack(
    allocator: &StatsAlloc<System>,
    driver: &TraversalDriver<'_>,
) -> BenchmarkResult<RuntimeAllocationOperation> {
    let mut session = driver.session_before_blocking_effect()?;
    let event = driver.blocking_effect(&mut session)?;
    assert_blocking_effect(event);
    measure_operation(allocator, "acknowledge_blocking", || {
        driver.acknowledge_blocking(&mut session)
    })
}

fn measure_operation<T>(
    allocator: &StatsAlloc<System>,
    operation: &'static str,
    measure: impl FnOnce() -> BenchmarkResult<T>,
) -> BenchmarkResult<RuntimeAllocationOperation> {
    let region = Region::new(allocator);
    let value = measure()?;
    black_box(&value);
    let stats = region.change();
    Ok(RuntimeAllocationOperation {
        operation,
        stats: stats.into(),
    })
}

fn assert_blocking_effect(event: DialogueEvent) {
    match event {
        DialogueEvent::Effect(effect)
            if effect.mode == recite_runtime::DialogueEffectMode::Blocking => {}
        other => panic!("expected blocking effect before acknowledgement, got {other:?}"),
    }
}

impl From<Stats> for RuntimeAllocationStats {
    fn from(stats: Stats) -> Self {
        Self {
            allocations: stats.allocations,
            bytes_allocated: stats.bytes_allocated,
            reallocations: stats.reallocations,
            bytes_reallocated: stats.bytes_reallocated,
            deallocations: stats.deallocations,
            bytes_deallocated: stats.bytes_deallocated,
        }
    }
}
