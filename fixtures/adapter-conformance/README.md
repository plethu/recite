# Adapter Conformance Fixtures

This directory publishes host-agnostic adapter conformance artifacts for
external adapter test suites.

The v1 fixtures live in [`v1/`](v1/) and include:

- a versioned scenario manifest (`scenarios.json`);
- a stable manifest schema contract;
- a stable operation/result schema contract.

## Why This Directory Exists

Most Recite source fixtures belong under `fixtures/recite/` so parser,
compiler, runtime, CLI, and LSP tests can share them directly. Adapter
conformance needs one additional layer: operation sequences, capability gates,
changed-asset policy declarations, and expected host-observable results.

Those adapter-driver concerns are published here so external adapters can reuse
one contract surface without copying internal Rust test helpers.

## Source Fixture Rule

- Keep `.recite` source fixtures under `fixtures/recite/` whenever possible.
- Use adapter-conformance manifests only for operation steps, capability gates,
  changed-asset policy declarations, and expected observations/errors.
- Do not duplicate parser/compiler/runtime snapshot expectations in this tree.
