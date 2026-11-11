---
title: Working as a team
description: Split a project between writers, translators and programmers, and integrate it in CI.
---

A Recite project is plain files, so a team integrates it with the repository tools it already uses:
branches, pull requests and merges. Recite adds checks for the problems a text merge cannot see. It
has no server, file locks or approval state of its own; your repository's review and merge process
stays in charge.

The [team project](https://github.com/plethu/recite/tree/main/fixtures/realistic/team-pack) is a
small ferry crossing written this way, and the repository check below replays a complete team
journey on it.

## Divide the files

One workable layout gives each role its own files:

- `scenes/*.recite` belong to writers, one area or chapter per file. Lines link across files with
  targets such as `-> scenes/office.recite::office`.
- `schema/team.schema.toml` belongs to the programmer and holds the authoritative declarations.
- `schema/generated/team.schema.json` is exported from it and committed, so reviewers see
  declaration changes. Nobody edits it by hand.
- `locales/*.po` belong to translators, one catalogue per locale.
- `fixture-*.toml` hold the answers a game would give on each tested path.
- `build/` holds compiled output and stays out of version control.

Smaller files mean fewer merge conflicts. Nothing requires this layout: `recite.project.toml` names
the source roots and catalogue patterns.

## Check every integration

Run the same gate on each branch and again on the merged result:

```sh
recite validate scenes --schema schema/generated/team.schema.json
recite id check scenes --since v1
recite compile scenes --schema schema/generated/team.schema.json -o build/crossing.recitec
recite project validate .
recite trace build/crossing.recitec --block dock --fixture fixture-source.toml
recite trace build/crossing.recitec --block dock --fixture fixture-fr.toml
```

`--since` names the last published tag. Every stable ID that existed there must still exist, and
removing one needs `--allow-removed ID`, so dropping a recorded or translated line is a visible
decision.

Two clean checkouts of the same revision compile identical bytes and produce identical traces. Add
`--output-format structured` when a CI step needs the diagnostic codes as JSON.

## What a merge does not catch

- **Edits to the same line.** Git stops with conflict markers. If markers are committed by mistake,
  validation fails with RECITE_PARSE001 at the marker.
- **Duplicate IDs.** Two writers who pick the same new ID in different files merge cleanly.
  Validation reports RECITE_ID003 and compilation is refused. Keep the ID on the line published
  first and give the other a fresh one with `recite id reassign scenes/office.recite:12`, using the
  file and line of its header.
- **Renamed blocks.** A block renamed on one branch, while another branch adds a link to its old
  name, merges cleanly. Validation reports RECITE_VALIDATE007 at the stale link.
- **Translations of changed lines.** PO changes merge like any text, translator comments included.
  When a writer rewords a line, its translation no longer matches, and the runtime shows the source
  text rather than an outdated translation. Refreshing the catalogue in Writer marks those entries
  for review; the CLI does not list them yet, and headless reporting is tracked in
  [#239](https://github.com/plethu/recite/issues/239).

A refused compile leaves the previous asset in place, so a game build never picks up a half-merged
project.

## Change the schema

When the programmer renames a condition or changes an effect parameter:

1. Edit `schema/team.schema.toml`.
2. CI detects that the committed export is stale by exporting to a temporary file and comparing:

   ```sh
   recite schema export --schema schema/team.schema.toml --producer-kind standalone \
     --producer-id team-pack --output build/fresh.schema.json
   recite schema check-fresh --expected schema/generated/team.schema.json \
     --actual build/fresh.schema.json
   ```

3. Regenerate explicitly by exporting to `schema/generated/team.schema.json`. The pull request then
   shows both the source and the generated change.
4. Validation reports every affected line, such as RECITE_VALIDATE034 for an unknown condition.
   Compilation keeps the last asset, and `recite project check-fresh .` reports that asset as built
   against an older schema (RECITE_FRESH002).
5. Update the dialogue and the runtime fixtures. A fixture that still answers the old condition name
   makes the trace fail instead of guessing.

## Saves across changes

A save made at a choice or at a blocking effect restores against the same compiled asset and emits
the same effect request ID again. A save from an asset that has since changed is refused; the game
decides whether to discard it or keep the old asset for that session.
[Testing dialogue](/guides/testing-dialogue/) covers the session API.

## Writer and other editors

Writer refuses to save over a file that changed on disk, including through a branch switch, and
keeps your draft in a recovery copy. It refuses to save read-only files rather than replacing them.
Recite has no lock integration: with Perforce or another lock-based system, check the file out so it
is writable before editing. Writer's own locks only coordinate Writer windows.

## Working alone

The same files and commands work for one person, with or without a repository server. Keep `id check
--since` pointed at your last release tag, and treat the gate above as a pre-release checklist.

## Repository check

From a Recite checkout, `mise exec -- just engines workflow` replays this journey in separate clones
of a scratch repository: two writers and a translator merge independent changes; an edit to the same
line, a duplicate ID, a stale link and a removed published ID are each refused until repaired; a
schema rename is detected, exported and repaired; and two clean checkouts build identical output. A
runtime test saves and restores the same project at a choice and a blocking effect.
