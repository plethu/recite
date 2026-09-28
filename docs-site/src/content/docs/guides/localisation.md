---
title: Localising dialogue
description: Extract stable contexts and test a PO catalogue in a deterministic run.
---

Start with the [complete example](/examples/headless-cli/). Its source anchors
identify the lines and choices independently of their text. Keep those anchors
when editing; changing an anchor changes the localisation identity.

```sh
recite extract src --schema schema/realistic.schema.json -o dialogue.pot
```

The POT gives translators the source text, stable context and source references.
Update a PO catalogue from that template with your gettext tooling, preserving
contexts and reviewing fuzzy or missing translations. The example includes
`locales/en-US.po`; it is a test catalogue, not a claim of additional language
coverage.

The example's `runtime-fixture.toml` selects it explicitly:

```toml
[dialogue]
locale = "en-US"
[dialogue.catalogs]
en-US = ["locales/en-US.po"]
```

Catalogue paths are relative to the fixture. Run the same compiled scene with
that fixture to check the translated text and inspect the trace for resolution
and fallback decisions:

```sh
recite run build/dialogue.recitec --block arrival --fixture runtime-fixture.toml
recite trace build/dialogue.recitec --block arrival --fixture runtime-fixture.toml
```

For interactive CLI preview, select a locale and catalogue explicitly:

```sh
recite play build/dialogue.recitec --block arrival --ui plain \
  --dialogue-locale en-US --dialogue-catalog en-US=locales/en-US.po
```

Interactive play asks for decisions; it does not read the runtime fixture.
The CLI's locale tests cover catalogue errors, fallback, markup and interpolation.
The shared adapter tests cover singular and plural resolution. Preserve markup,
interpolation names and plural structure when translating; an apparently readable
translation can still violate those contracts.

Recite Writer can edit PO entries while retaining comments, contexts and unknown
fields. Translator review and language quality remain human work. Engine
catalogue installation is covered by the corresponding adapter guide.
