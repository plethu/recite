---
title: Authoring dialogue
description: Work from source edits through diagnostics, compilation and engine refresh.
template: splash
---

Keep `.recite` source, schema declarations, PO catalogues and runtime fixtures in version control.
Compiled assets and generated schema manifests are outputs; edit their owning source.

## Source and schema

Use the [source reference](/reference/source-format/) for syntax. Preserve anchors when editing
text. When the game API changes, update the schema producer, regenerate its manifest and compile
against it.

## Edit, check, preview

The [complete CLI example](/examples/headless-cli/) supplies a project to copy and commands from
validation through traces. Use a configured
[text editor](https://github.com/plethu/recite/tree/main/editors) or Writer for diagnostics and
guarded ID actions; fix diagnostics before engine testing. Follow
[testing dialogue](/guides/testing-dialogue/) for fixture decisions and
[localisation](/guides/localisation/) for PO preview.

## Watch and restart

The [engine authoring walkthrough](/adapters/authoring/) covers rebuild failure/recovery, accepted
imports, active versus next-session revisions and compatible saves. Game state remains host-owned.
Headless checks do not establish interactive editor or native accessibility acceptance.
