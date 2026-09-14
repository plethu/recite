# Freya file-backed writer

Freya is selected; this is the start of the retained workbench, still hosted in
the isolated experimental workspace. Run from the repository root:

```sh
cargo run --locked --manifest-path prototypes/gui-bakeoff/Cargo.toml -p recite-bakeoff-freya -- --project /path/to/project
```

The project argument opens that project through Recite's shared manifest
discovery. Use **Project** to reveal the path, open another project, or refresh
its context. Select a scene in the sidebar,
edit in Script or Source, and click **Save** or press Ctrl+S (Cmd+S on macOS). Save applies the current field
draft first; rejected prose remains available to repair. Opening another project
or file refuses while the current document has unsaved changes.

Use **Hide scenes** to reclaim sidebar space and click a section heading to
fold its passages. **Passage actions ▾** opens the selected passage's details and
Add choice menu. Arrow keys move between actions; Escape or Tab returns to the
trigger. Draft actions stay beside the active field.

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

## Recovery and closing

The writer keeps an atomic JSON recovery snapshot beside the current file,
including applied source, the original saved bytes, the selected field, and any
unaccepted field draft. Opening that file again restores the session. Source
files are changed only by Save. A process crash releases the recovery lock;
another writer cannot own the same file's recovery snapshot at the same time.
The empty `.recite-editor-recovery.lock` file may remain and need not be removed.

Closing an edited file offers **Keep editing**, **Keep recovery and close**, or
**Save and close**. Save failures leave the window open. Tab and Shift+Tab cycle
within the dialog, and Escape returns focus to the previous control.

If the source changed while the writer was closed, recovered work still uses
its original baseline: Save refuses to overwrite the external edit. **Keep
recovery copy and reload disk** retains the full local session in a named
`.recite-recovered-*.json` file before loading the current source. The message
shows that copy's path. These copies are retained until you remove them.
To restore a retained copy, close the writer, preserve any existing
`<source>.recite-editor-recovery.json`, and copy the retained JSON to that path
before reopening the source. The original baseline still protects external edits.

Recovery is written after buffer updates, so an abrupt crash can lose changes
that have not reached that update. A corrupt or unsupported snapshot is left
untouched and reported; move it aside for inspection before retrying. Recovery
writes can fail on read-only directories or a full disk. Keep the window open
when that happens, and retry Save after resolving the error. ACL/xattr and
crash-injection acceptance remain outstanding across platforms.

## Project context

Diagnostics use all discovered sources, the current document's applied unsaved
overlay, and the configured generated schema manifest. File locations and
codes accompany each diagnostic. **Refresh project context** rereads project
sources and schema while retaining the current source, field draft, and undo
history. It marks an existing preview stale. A failed refresh retains the last
accepted context and reports the error.

Preview compiles that same set of effective sources and schema. Script starts
at the selected passage's block; Source starts at the compiled default block.
Cross-file links use the normal Recite reference rules. Existing previews keep
their compiled input until explicitly restarted. Conditions and effects still
stop this bounded preview; no game operation is executed. Scene-manifest-specific
build outputs and runtime fixture inputs remain to be connected.

Project localisation, catalogue editing, completion, and the complete
accessibility/platform requirements remain outstanding. Field drafts must be
applied before their content contributes to diagnostics or preview.

## Writing trial

Use a copy of a real project. Edit in Script and Source, leave a field draft,
close with recovery, and reopen. Then change the source in another editor and
exercise the conflict/reload path. Check keyboard save, dialog navigation,
focus restoration, and whether the recovered field matches what you left.

Component and filesystem tests cover these transitions. Physical IME/BiDi,
screen-reader operation, native window-manager closing, and macOS/Windows
acceptance still need hands-on checks. The pinned CodeEditor preedit limitation
remains recorded in the [text-input evidence](evidence.md).

Focused verification for this isolated workspace:

```sh
mise exec -- cargo test --locked --manifest-path prototypes/gui-bakeoff/Cargo.toml -p recite-bakeoff-authoring -p recite-bakeoff-freya
mise exec -- cargo clippy --locked --manifest-path prototypes/gui-bakeoff/Cargo.toml -p recite-bakeoff-authoring -p recite-bakeoff-freya --all-targets -- -D warnings
mise exec -- cargo fmt --manifest-path prototypes/gui-bakeoff/Cargo.toml --all -- --check
scripts/check-test-organization.sh
scripts/check-git-policy.sh
```
