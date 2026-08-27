use std::alloc::System;
use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard};

use recite_benchmarks::runtime_allocations::{
    RuntimeAllocationOptions, build_runtime_allocation_report,
};
use recite_benchmarks::{BenchmarkFixture, BenchmarkScale};
use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

static ALLOCATION_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn tiny_runtime_allocation_report_covers_hot_path_operations()
-> Result<(), Box<dyn std::error::Error>> {
    let _guard = allocation_test_guard();
    let report = build_runtime_allocation_report(
        &RuntimeAllocationOptions::new(vec![BenchmarkFixture::Synthetic(BenchmarkScale::Tiny)]),
        GLOBAL,
    )?;

    let fixture = report.fixtures.first().expect("tiny fixture report");
    assert_eq!(fixture.fixture, "tiny");

    let operations = fixture
        .operations
        .iter()
        .map(|operation| operation.operation)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        operations,
        BTreeSet::from([
            "acknowledge_blocking",
            "choose_first",
            "condition_dispatch",
            "effect_blocking",
            "effect_deferred",
            "effect_immediate",
            "full_traversal",
            "localised_next",
            "next_line",
            "next_prompt",
            "session_decode",
            "session_encode",
            "start_scene",
        ])
    );
    assert!(
        fixture
            .operations
            .iter()
            .any(|operation| operation.stats.allocations > 0)
    );
    assert!(
        fixture
            .operations
            .iter()
            .all(|operation| operation.stats.bytes_allocated >= operation.stats.allocations)
    );
    Ok(())
}

#[test]
fn runtime_allocation_markdown_contains_caveats_and_operation_table()
-> Result<(), Box<dyn std::error::Error>> {
    let _guard = allocation_test_guard();
    let report = build_runtime_allocation_report(
        &RuntimeAllocationOptions::new(vec![BenchmarkFixture::Synthetic(BenchmarkScale::Tiny)]),
        GLOBAL,
    )?;
    let markdown = report.to_markdown();

    assert!(markdown.contains("# Recite Runtime Allocation Pressure"));
    assert!(markdown.contains("| Operation | Allocations | Allocated bytes |"));
    assert!(markdown.contains("| start_scene |"));
    assert!(markdown.contains("Clone pressure is inferred"));
    assert!(markdown.contains("#109 establishes the release benchmark baseline profile"));
    Ok(())
}

fn allocation_test_guard() -> MutexGuard<'static, ()> {
    ALLOCATION_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
