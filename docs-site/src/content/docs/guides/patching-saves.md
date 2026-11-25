---
title: Patching a shipped game
description: What happens to a player's save when a patch changes the dialogue, and how to check it before you ship.
---

Players save in the middle of conversations. When a patch changes the dialogue, Recite resumes each
save where the player left off if it can find that point again. If it can't, it hands your game a
fallback instead of an error. You can see which saves fall back before you ship the patch.

Saves taken between conversations are not affected: they hold no Recite session.

## What a save holds

A Recite save records where the conversation is. It uses the block's ID, the last line or prompt
before that point and the statements after it, all through their stable `@` IDs. It also keeps the
effects the conversation has already requested, exactly as they were requested. It never holds game
state.

When you restore a save against a compiled asset, Recite compares the asset with the one the save
was taken against:

| Result            | When                                                                                                                                                      | What the player sees                                        |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `exact`           | The asset is the one the save was taken against                                                                                                           | The conversation continues                                  |
| `unchanged_block` | The patch changed other blocks or files, the compiler or the schema, but not the saved block                                                              | The conversation continues                                  |
| `text_edited`     | Only text, speakers, interpolation or metadata changed in the saved block                                                                                 | The conversation continues with the corrected text          |
| `relocated`       | The saved block's structure changed, but the saved point was found again through its stable IDs                                                           | The conversation continues, including any newly added lines |
| fallback          | The block is gone, the line or prompt that located the save is gone or moved, or a statement between that line and the save was added, removed or changed | No session; your game decides what happens next             |

A prompt that was waiting when the player saved is presented again when the save is restored, so
your game does not need to remember it. If the patch only changed text in that block, the prompt
keeps the availability it had when the player saved. Any other change to the block, such as an added
choice or a new requirement on one, evaluates the prompt again from the patched content, querying
your conditions.

A save taken after a conversation ended restores no conversation, whatever the patch changed.
Adapters report that no session is active, exactly as for the unpatched build.

A blocking effect that was waiting is emitted again with its original request ID, even when the
patch moved it. Acknowledge that same ID. Deferred effects collected before the save are delivered
as they were saved, even if the patch removed the effect from the source.

## Edits and their effect on saves

| Edit                                                                                                                       | Saves in the edited block               | Saves elsewhere |
| -------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- | --------------- |
| Fix a typo or reword a line or choice                                                                                      | Resume, with the new text               | Unaffected      |
| Add a line before the last line the player saw, or after the save point                                                    | Resume; added lines after it are played | Unaffected      |
| Add a line between the last line the player saw and the save point, including before an `if` or `match` the save is inside | Fall back                               | Unaffected      |
| Add, remove, reorder or re-gate choices at a waiting prompt                                                                | Resume; the prompt is evaluated again   | Unaffected      |
| Change an effect's arguments or a condition next to the save                                                               | Fall back                               | Unaffected      |
| Delete or move the line the player saved after                                                                             | Fall back                               | Unaffected      |
| Rename or delete a block                                                                                                   | Fall back                               | Unaffected      |
| Change a stable `@` ID                                                                                                     | Fall back at points that ID located     | Unaffected      |
| Update the compiler or the schema                                                                                          | Resume, usually as `unchanged_block`    | Unaffected      |

Recite never guesses. When two prompts could be the one the player saved at, the save falls back
with `landmark_ambiguous`.

## Check a patch before you ship it

`recite save impact` saves at every point a player could and restores each save against your patched
build. Compare the project with your last release tag:

```sh
recite save impact --since v1.2.0 .
```

```text
build/dialogue.recitec: 7 save points: 0 exact, 3 in unchanged blocks, 2 with edited text, 1 relocated, 1 falls back
main.recite:17:1: a save waiting for blocking effect `open_map` in block `town` falls back: a statement between @l_town and the save point was added, removed or edited
```

To compare compiled files instead of git revisions, pass `--baseline shipped.recitec
patched.recitec`. In CI, add `--check`, which fails while any save point falls back. When a fallback
is intended, acknowledge its block with `--allow-fallback town`. `--format json` writes every save
point with its outcome.

While writing, `recite watch --save-baseline v1.2.0 .` prints the same report after each rebuild.

## Restore saves in your game

Every adapter restores against the asset you have now and tells you what happened. Pick how far a
save may move with a maximum level: `relocated` (the default) accepts any deterministic resume, and
`exact` resumes only against the asset the save was taken against.

- **Bevy:** send `ReciteRequest::Restore { asset, snapshot, variant, options: ResumeOptions::new()
  }`. You receive `ReciteOutputValue::Resumed { report, events }` or
  `ReciteOutputValue::ResumeFallback(fallback)`.
- **Godot:** `restore()` or `restore_with_options(asset, bytes, variant, "relocated")`. Read
  `result.restore_outcome()`: its `outcome` is `resumed` or `fallback`.
- **Unity:** `ReciteDialogueService.Resume(asset, snapshot)` returns a `ReciteRestoreResult`.
  `ReciteDialogueRunner.Restore` raises `ResumeFallback` when a save falls back.
- **C ABI:** `recite_session_prepare_resume` or `recite_session_resume` with a
  `RECITE_RESUME_LEVEL_*` constant. They write a MessagePack report with the same fields.
- **Rust:** `recite_runtime::resume::resume_session_messagepack`.

The older exact-only restores (`recite_session_prepare_restore`, Unity's `Restore`) still reject any
changed asset.

## Handle a fallback

A fallback names the block the player saved in, whether that block still exists, and why the save
could not resume. It also carries the effects the dialogue had already requested. A reasonable
policy:

1. Reconcile the pending blocking effect, if any. The conversation was waiting on it.
2. If the block still exists, restart it. In Rust, `ResumeFallback::restart_block` does this in the
   saved locale. Otherwise end the conversation and leave the player where the game put them.
3. Apply the deferred effects, which the conversation already promised your game. A restart replays
   the block, so its deferred statements before the save point are requested again and delivered
   with the restarted conversation's end. When you restart, apply only the saved effects the replay
   will not request again, or make those effects idempotent.
4. Log the reason, so support can match a report to the patch.

## Investigate a player's save

`recite save inspect player.sav` shows what a save holds without running the game: the block, the
point inside it, a waiting prompt or effect, and the deferred effects. Add `--asset patched.recitec`
to see how that save restores against a build. The output contains stable IDs and effect names, but
no line text.
