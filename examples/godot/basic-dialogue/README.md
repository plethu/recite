# Recite Godot Basic Dialogue

Build the extension from the repository root, copy the library into `bin/`,
compile `dialogue/basic.recite` to `dialogue/basic.recitec`, then open this
folder with Godot 4.

The adapter policy for this MVP is `reload_for_next_session_only`: reimported
compiled assets affect new sessions, not the session already owned by a
`ReciteDialogueNode`.
