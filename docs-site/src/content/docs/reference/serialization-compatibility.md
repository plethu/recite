---
title: Serialization Compatibility
description: Recite's current wire-format decision and future migration gate.
---

Recite keeps its current MessagePack contracts. The full, checked-in decision
is in the [serialization compatibility decision](https://github.com/plethu/recite/blob/main/docs/serialization-compatibility.md);
this page makes that contract part of the published reference.

The boundaries are separate:

- compiled assets use deterministic MessagePack with `format_version = 0` and
  `compiler_compatibility_version = 0`;
- runtime snapshots use the current MessagePack codec with snapshot format
  `1`;
- FFI output batches use named-map MessagePack with `batch_format_version = 0`;
- FFI condition payloads use the current MessagePack callback contract and
  have no independent version.

Compact JSON is inspection-only. A future encoding needs a named artifact,
measured Recite evidence, an explicit versioned boundary, typed-model
migration, and conformance for every shipped host. New FFI encodings first
need a separate ABI design under [#171](https://github.com/plethu/recite/issues/171);
Unity v0 batch rejection is required there and is not claimed as passing here.
