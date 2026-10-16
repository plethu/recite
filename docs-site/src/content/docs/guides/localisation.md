---
title: Localising dialogue
description: Extract stable contexts and test a PO catalogue in a deterministic run.
template: splash
---

Start with the [complete example](/examples/headless-cli/). Its source anchors identify the lines
and choices independently of their text. Keep those anchors when editing; changing an anchor changes
the localisation identity.

```sh
recite extract src --schema schema/realistic.schema.json -o dialogue.pot
```

The POT gives translators the source text, stable context and source references. Update a PO
catalogue from that template with your gettext tooling, preserving contexts and reviewing fuzzy or
missing translations. The example includes `locales/en-US.po`; it is a test catalogue, not a claim
of additional language coverage.

The example's `runtime-fixture.toml` selects it explicitly:

```toml
[dialogue]
locale = "en-US"
[dialogue.catalogs]
en-US = ["locales/en-US.po"]
```

Catalogue paths are relative to the fixture. Run the same compiled scene with that fixture to check
the translated text and inspect the trace for resolution and fallback decisions:

```sh
recite run build/dialogue.recitec --block arrival --fixture runtime-fixture.toml
recite trace build/dialogue.recitec --block arrival --fixture runtime-fixture.toml
```

For interactive CLI preview, select a locale and catalogue explicitly:

```sh
recite play build/dialogue.recitec --block arrival --ui plain \
  --dialogue-locale en-US --dialogue-catalog en-US=locales/en-US.po
```

Interactive play asks for decisions; it does not read the runtime fixture. The CLI's locale tests
cover catalogue errors, fallback, markup and interpolation. The shared adapter tests cover singular
and plural resolution. Preserve markup, interpolation names and plural structure when translating;
an apparently readable translation can still violate those contracts.

Recite Writer can edit PO entries while retaining comments, contexts and unknown fields. Translator
review and language quality remain human work. Engine catalogue installation is covered by the
corresponding adapter guide.

## Dialogue in other scripts

Recite stores dialogue as UTF-8 and leaves shaping to the host, so any script a game can render
works in source, catalogues and the runtime. This site bundles Latin, Cyrillic and Greek faces;
other scripts use the reader's system fonts.

<div class="script-samples">
  <p lang="ru">Если ты насчёт передатчика — возьми номерок.</p>
  <p lang="el">Αν ήρθες για τον πομπό, πάρε αριθμό.</p>
  <p lang="ja">送信機のことなら、番号札を取ってください。</p>
  <p lang="zh-Hans">如果你是为了发射机来的，请先取号。</p>
  <p lang="ko">송신기 때문에 왔다면 번호표를 뽑으세요.</p>
  <p lang="ar" dir="rtl">إذا جئت بخصوص جهاز الإرسال، فخذ رقماً.</p>
  <p lang="he" dir="rtl">אם באת בגלל המשדר, קח מספר.</p>
  <p lang="hi">अगर आप ट्रांसमीटर के बारे में आए हैं, तो नंबर ले लीजिए।</p>
  <p lang="th">ถ้าคุณมาเรื่องเครื่องส่งสัญญาณ กรุณารับบัตรคิว</p>
</div>
