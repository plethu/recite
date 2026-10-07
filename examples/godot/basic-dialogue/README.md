# Recite Godot Basic Dialogue

This project ships in `recite-godot-addon.tar.gz`. Copy the archive's `addons/recite` directory here
and run `recite watch .` from this directory. The included `recite.project.toml` builds
`dialogue/basic.recitec` from the authored source. Open the folder with Godot 4.6.3; the enabled
Recite editor plugin imports the compiled file, and the scene loads its native
`ReciteDialogueResource` through `ResourceLoader`.

For a source edit, keep the watcher running, change text in `dialogue/basic.recite`, and check
editor diagnostics or run `recite validate dialogue/basic.recite` and `recite check-ids
dialogue/basic.recite`. Wait for a successful watch rebuild, then let the Godot editor reimport
`dialogue/basic.recitec`. End the current dialogue session and start another to see the accepted
revision. A failed compile leaves the previous compiled file in place; a rejected import retains the
last valid imported Resource. The shared
[authoring walkthrough](https://github.com/plethu/recite/blob/main/docs-site/content/adapters/authoring.md)
has a complete diagnostic, correction, and rebuild exercise.

The refresh policy is `reload_for_next_session_only`: reimported compiled assets affect new
sessions, not the session already owned by a `ReciteDialogueNode`.
