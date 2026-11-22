---
title: Checking spelling and grammar
description: Check dialogue and choice prose for spelling and grammar in editors and CI, without flagging Recite syntax.
---

Recite checks the prose authors write, the bodies of lines and choices, and leaves headers, stable
IDs, speakers, conditions, effects, diverts and comments alone. Checking runs locally with the
bundled [Harper](https://github.com/automattic/harper) engine: no prose leaves your machine, and no
account, server or network connection is needed. Findings are warnings and never stop `validate` or
`compile`.

## Turn it on for a project

Add a `[prose]` table to `recite.project.toml`:

```toml
[prose]
enabled = true
language = "en-GB"
dictionaries = ["prose-words.txt"]
```

| Key               | Meaning                                                                                        |
| ----------------- | ---------------------------------------------------------------------------------------------- |
| `enabled`         | Editors check prose as you type. Defaults to `false`; `recite prose check` runs regardless.    |
| `language`        | `en-US` (default), `en-GB`, `en-AU`, `en-CA` or `en-IN`.                                       |
| `dictionaries`    | Project-relative word lists, one word per line, `#` for comments. Added words go to the first. |
| `ignore_rules`    | Checker rule names never reported, such as `SentenceCapitalization`.                           |
| `ignore_speakers` | Speakers whose lines are skipped, for characters with intentional nonstandard speech.          |
| `ignore_ids`      | Stable IDs whose prose is skipped; `prefix*` matches a group.                                  |
| `metadata_keys`   | Metadata keys whose quoted values are checked too, such as `caption`.                          |

Commit the dictionary with the project so every writer and CI share character names, places and
invented vocabulary.

## What counts as prose

```text
:: harbour default
# Notes for the team are comments and are not checked.
> speaker=rhea caption="The door creaks open" @rhea_arrival
  [slow]Welcome back, {name}.[/slow] The lamps in Vexmoor
  were already lit.

  ? @go_market
    To the market.
    -> market
```

The line body and the choice body are checked as sentences, with the wrapped line joined and blank
lines treated as paragraph breaks. Markup tags such as `[slow]` are removed before checking, so the
words around them keep their context, and placeholders such as `{name}` stay opaque: they are never
flagged and never trigger an article or agreement rule. `\{` and `\}` are read as braces. Bracketed
stage directions such as `[The stalls are shuttered.]` are checked as prose. The `caption` value is
checked only when `metadata_keys` lists it.

`Vexmoor` would be flagged until it is added to a project dictionary. Incomplete lines are checked
while you type, even before they have IDs.

## In your editor

The Recite language server publishes prose findings with the codes `RECITE_PROSE001` (spelling),
`RECITE_PROSE002` (grammar and usage) and `RECITE_PROSE003` (style), alongside its other
diagnostics. Quick fixes replace the flagged text with a suggestion or add a word to the project's
first dictionary. Fixes only ever edit the prose itself.

- **VS Code and VSCodium:** findings appear in Problems and quick fixes in the light-bulb menu.
- **Neovim:** findings use `vim.diagnostic`; quick fixes come from `vim.lsp.buf.code_action()`.
- **Zed:** findings appear in the diagnostics panel and code actions menu.

Adding a word edits the dictionary file through your editor. If the editor opens that file instead
of saving it, save it to refresh the findings. Turn off other spell-checking extensions for
`.recite` files, or they will flag syntax again.

## In Writer

When the open project enables `[prose]`, Writer checks the open document in the background and
counts its findings with the document's other problems: in the status line, at the beat they belong
to and in the list under the manuscript. A finding listed there offers its suggested replacements
and, for a spelling, adding the word to the project's first dictionary. A replacement is one
undoable edit; apply or discard a pending draft first. A setup problem, such as an unsupported
language, appears as one warning.

## In CI

```bash
recite prose check dialogue/
recite prose check dialogue/ --output-format structured
```

`prose check` exits with status 1 when it reports any finding, so a CI job can require clean prose.
Structured output adds byte-exact fixes for automated review tools. Use `recite explain
RECITE_PROSE001` for guidance on a code.

## Limits

- Harper checks English only; other languages report `RECITE_PROSE101`.
- Grammar rules can misread deliberate dialogue. Suppress a rule, a speaker or a range of IDs rather
  than rewriting a character's voice.
- There is no inline suppression for a single sentence, because comments end a prose body.
