---
title: JSON, CSV, and Engine-Native Formats
description: Migration notes for custom dialogue data, spreadsheets, and engine-specific resources.
---

Custom formats usually encode project decisions rather than a standard dialogue language. Treat the migration as data modeling first: identify text, speakers, choices, branch targets, conditions, effects, metadata, and localisation IDs before writing conversion code.

This page covers internal JSON, CSV, spreadsheets, ScriptableObjects, Godot resources, Unreal data assets, and similar engine-native content. Use official engine or tool docs for the source format parser, then keep the Recite mapping explicit and project-owned.

## Concept map

| Custom field | Recite |
| --- | --- |
| Row ID, node ID, asset name | Block ID, line ID, choice ID, or source metadata |
| Text column | Line or choice body |
| Speaker column | `speaker=` field |
| Portrait, emotion, camera, audio, category | Metadata or typed effect |
| Next node column | Target |
| Condition expression column | Pure condition function |
| Action column | Typed effect request |
| Localisation key | Stable Recite ID or source metadata |

## Clean migrations

- Tables with one row per line map cleanly if IDs and branch targets are stable.
- JSON nodes with explicit choices and targets map cleanly to blocks and choices.
- Engine resource references can be preserved as metadata when they are descriptive IDs.
- Localisation keys can become Recite IDs if they already follow a stable identity policy.

## Lossy migrations

- Free-form script snippets need manual review.
- Columns with overloaded meanings should be split into metadata, conditions, and effects.
- Engine object references may not be portable outside the source engine.
- Spreadsheet formulas, comments, colors, and hidden columns are easy to miss.

## Manual work

- Define a source schema before conversion.
- Reject or report rows with missing IDs, duplicate IDs, dangling targets, or ambiguous action fields.
- Decide which columns are canonical Recite data and which are source provenance metadata.
- Write fixture tests for at least one simple branch, one conditional choice, one effect, and one localisation row.

## Not imported or replaced

- Engine scenes, editor-only assets, spreadsheet formatting, custom runtime code, object references, and save data.
- Automatically inferred behavior from column names.
- Compatibility with every historical version of a project-specific data format.

## Before

```json
{
  "id": "market_01",
  "speaker": "seller",
  "text": "Fresh pears.",
  "choices": [
    { "id": "buy", "text": "Buy one", "next": "market_buy", "action": "coins:-1" }
  ]
}
```

## After

```text
:: market_01 default
> market_001@b78d4fb08772db37e008 speaker=seller source_id=market_01
  Fresh pears.
? market_buy_one@f3fef3a9609f2191ce0e
  Buy one.
  -> market_buy

:: market_buy
! blocking spend_currency(coins, 1)
! blocking grant_item(pear)
> market_002@4ccc3fb8501a3fe3017d speaker=seller
  Here you go.
-> END
```

Next workflow:

```bash
recite validate dialogue/market.recite
recite compile --output build/dialogue.recitec dialogue/market.recite
```

## Related docs

- [Importer Boundaries](/migration/importer-boundaries/)
- [Schema](/reference/schema/)
- [Testing Dialogue](/guides/testing-dialogue/)
- [Headless CLI Example](/examples/headless-cli/)
