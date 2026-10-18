---
title: Edit and refresh dialogue
description: Try source diagnostics, watch builds, and a new dialogue revision in your engine.
---

An edit becomes playable after Recite compiles it and the engine imports the result. A running
dialogue keeps its original revision. Start another session to hear the new text.

## Set up a small project

Install the adapter using the [Bevy](/adapters/bevy/), [Godot](/adapters/godot/), or
[Unity](/adapters/unity/) guide. Build the CLI with `cargo build --locked -p recite-cli` from the
Recite checkout, then put its `target/debug` directory on your shell's `PATH`.

Create a directory for this exercise inside the engine's asset directory: `assets/dialogue` for a
default Bevy asset source, `dialogue` in a Godot project, or `Assets/Dialogue` in Unity. Put this
`recite.project.toml` inside it:

```toml
format_version = 1

[[scenes]]
id = "refresh_demo"
asset = "build/dialogue.recitec"
block = "start"
participants = ["narrator"]
```

Beside the manifest, create `main.recite`:

```recite
:: start default speaker=narrator
> greeting@11111111111111111111
  The door is open.
? leave@22222222222222222222
  Leave.
  -> END
```

The anchors after `@` identify the line and choice. Keep them when editing their text. For larger
projects, use the editor's stable-ID actions to add missing anchors. If you use game conditions or
effects, export the engine's schema first and set `[project].schema` to that manifest's relative
path.

## Watch an edit

From the directory containing `recite.project.toml`, run:

```sh
recite check-ids main.recite
recite watch .
```

Wait for the first successful build. Load `build/dialogue.recitec` in your engine and start the
`start` block. Leave the dialogue at its choice prompt.

Open `main.recite` in an editor with the Recite language server enabled. Change `-> END` to `->
missing_block`. The editor should report the missing target; save to see the build diagnostic in the
watch terminal too. The failed build leaves the previous compiled asset in place.

Restore `-> END`, change the line to `The door is closed.`, and save. Wait for the successful
rebuild, then refresh the engine asset:

| Engine | Import the new revision                                                                                                                                        | Start a new session                                                                     |
| ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Bevy   | Call `AssetServer::reload("dialogue/build/dialogue.recitec")` and wait for `ReciteAssetImport::Accepted`. The adapter does not enable automatic file watching. | Send `ReciteRequest::End`, then `ReciteRequest::Start` with the same handle.            |
| Godot  | Let the editor finish importing the changed file. Stop and rerun the scene to reload its imported Resource.                                                    | Start the dialogue through `ReciteDialogueNode` as in the example.                      |
| Unity  | Return to the Editor and let it import the file. If automatic refresh is disabled, use **Assets > Refresh**.                                                   | Stop and re-enter Play Mode with the imported asset assigned to `ReciteDialogueRunner`. |

The dialogue already at the prompt must remain usable with its original choice ID. Its compiled
revision stays unchanged. The new session should show `The door is closed.`. Restarting a Godot
scene or Unity Play Mode is the simplest way to try this in the editor; the adapter also preserves
an active session when its available asset is updated programmatically.

In a second terminal, run `recite check-fresh .` after the successful build. It compares compiled
fingerprints with the source and schema. A game that loads only the compiled file cannot make that
comparison.

## If the new text does not appear

Check the watch result first, then the engine's import result. A failed build does not replace the
compiled file. A rejected engine import may retain its last valid revision and report an error. A
successful import still leaves a running session on its original revision.

Restore saved sessions against the revision they were saved from. Stable line and choice IDs survive
a text edit, but they do not make an old snapshot compatible with changed compiled content. Keep the
previous asset when you need to finish an old session; otherwise start a new one.

The
[workflow evidence](https://github.com/plethu/recite/blob/main/docs/engine-authoring-workflows.md)
lists automated checks separately from this editor walkthrough.
