# Adapter Conformance Fixtures

This directory publishes host-agnostic adapter conformance artifacts for external adapter test
suites.

The v1 fixtures live in [`v1/`](v1/) and include:

- a versioned scenario manifest (`scenarios.json`);
- a stable manifest schema contract;
- a stable operation/result schema contract.

## Why This Directory Exists

Most Recite source fixtures belong under `fixtures/recite/` so parser, compiler, runtime, CLI, and
LSP tests can share them directly. Adapter conformance needs one additional layer: operation
sequences, capability gates, changed-asset policy declarations, and expected host-observable
results.

Those adapter-driver concerns are published here so external adapters can reuse one contract surface
without copying internal Rust test helpers.

## Source Fixture Rule

- Keep `.recite` source fixtures under `fixtures/recite/` whenever possible.
- Use adapter-conformance manifests only for operation steps, capability gates, changed-asset policy
  declarations, and expected observations/errors.
- Do not duplicate parser/compiler/runtime snapshot expectations in this tree.

## Observation Modes

The runtime reference driver observes individual `advance` calls. The adapter contract also permits
a host operation to drain an ordered batch. The engine companions use `transactional_drained_batch`:
start, select, or acknowledge returns a complete batch up to the next prompt, blocking effect, or
end.

Adapter tests must name their observation mode and the scenario IDs they exercise. For successful
operations, compare the ordered events in the batch with the corresponding reference observations. A
failure can surface during start or select instead of a later reference `advance`; assert the same
error category, no partial failed batch, and preservation of the previous session. Do not add a
second traversal implementation merely to reproduce the reference driver's call shape.

Some misuse operations are absent by construction. An owner that captures its asset cannot receive a
different asset on `advance`. Test that ownership invariant and incompatible restore through its
public API, and report this as an invariant check rather than claiming to execute the unavailable
operation. Capability-gated and non-selected refresh-policy scenarios remain explicitly not
applicable; an unavailable engine host is an unrun check.

The mandatory `adapter_runner_required` scenarios must run through the actual adapter surface and
assert their structured fields. Passing the reference suite does not cover those scenarios. A
managed harness using the native ABI establishes that managed/native boundary only; engine import,
lifecycle, and player checks require the engine host.
