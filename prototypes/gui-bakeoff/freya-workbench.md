# First Freya file-backed slice

Freya is selected; this is the start of the retained workbench, still hosted in
the isolated experimental workspace. Run from the repository root:

```sh
cargo run --locked --manifest-path prototypes/gui-bakeoff/Cargo.toml -p recite-bakeoff-freya -- --project /path/to/project
```

The project argument pre-fills the path field. Click **Open project** to use
Recite's shared manifest discovery and display its source files. Select a file,
edit in Script or Source, and click **Save**. Save applies the current field
draft first; rejected prose remains available to repair. Opening another project
or file refuses while the current document has unsaved changes.

This run command opens a native window normally. On an actively used Hyprland
desktop, set a floating/silent rule or use the isolated launcher:

```sh
python3 prototypes/gui-bakeoff/scripts/try-writer.py freya --project /path/to/project
```

## Save boundary

The editor compares the file's current bytes with the version it opened or
last saved. A conflict refuses replacement and retains the in-memory work.
Saves use an exclusive cooperative sidecar lock, a synced temporary file in
the same directory and atomic replacement. Each replacement keeps the previous
bytes in a hidden `.recite-editor-backup-*` file beside the source.

Existing permissions are preserved. Symlink/non-regular save targets and
read-only files are refused. On Unix the parent directory is synced, including
on a no-op retry after a failed directory sync. Failure after replacement can
mean the new bytes are already visible; retry Save to complete synchronization.

The lock coordinates these editor instances, not arbitrary third-party writers.
A non-cooperating writer can still race the final content check. A process crash
can leave `<source>.recite-editor.lock`; remove it only after confirming no
editor is saving that file. Backups are retained for inspection and are not
automatically pruned. ACLs/xattrs and crash-injection durability have not been
accepted on every platform.

## Current limits

Save before closing: there is not yet a close prompt or recovery of unsaved
field drafts. Backups preserve previous saved bytes, not your unsaved session.
Use copies while this lifecycle is being completed.

Preview and diagnostics still operate on the current file, without the project's
schema or other files. This is not project validation. Conditions, effects and
cross-file references remain outside the bounded preview; effects never execute
game operations. Full project context must be integrated before that workflow
is accepted.

The mode uses real project-relative document keys when planning new stable IDs.
Files outside the supported Script subset open in Source. Project localisation,
catalogue editing, completion, scene-manifest navigation and the complete
accessibility/platform requirements remain outstanding.
