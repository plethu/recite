---
title: Migrating from Clyde manually
description: Plan a manual migration without an automatic Clyde importer.
---

There is no Clyde importer in this release scope. Keep the original project and choose one
conversation to migrate by hand. Use the native [source reference](/reference/source-format/) and
your game's schema to define blocks, lines, choices and targets. Preserve existing identifiers in a
mapping record when they cannot be used as Recite anchors.

Review state-dependent behavior separately. Put host state queries in declared conditions, game work
in typed effects, and presentation data in metadata. Do not assume that matching text establishes
matching traversal or save behavior.

Add a deterministic runtime fixture for each path you intend to preserve, then follow
[testing dialogue](/guides/testing-dialogue/). Engine bindings, localisation data and existing saves
require explicit project decisions; this guide promises no automatic conversion or runtime
compatibility.
