---
title: Twee and Twine source imports
description: Inspect passages and convert plain text with standalone static links.
template: splash
---

```sh
recite import fixtures/import/passages.twee --from twee
```

The reader accepts passage headers such as `:: Start`, plain text and a final group of standalone
links. Supported links are `[[Target]]`, `[[Label->Target]]`, `[[Target<-Label]]` and
`[[Label|Target]]`. A link group must follow a dialogue line. It becomes a Recite choice prompt;
passage names are mapped to generated block IDs in the report.

Story data, scripts, styles, header tags/metadata, story-format macros, inline links and formatting
require manual migration. A passage containing unsupported syntax is held back as a whole, with
source spans in the report. Targets that were not imported fail native validation. The importer does
not read a published Twine HTML game or run its story format.

Choose the intended entry block after reviewing the name mappings. The first converted passage
becomes the generated default; `StoryData` is held back and does not select an entry passage
automatically. Review the [report and generated source](/migration/importer-boundaries/) before
writing.

The
[Twee 3 specification](https://github.com/iftechfoundation/twine-specs/blob/master/twee-3-specification.md)
defines the wider container format. This reader supports only the documented plain passage/link
subset and makes no SugarCube or Harlowe runtime promise.
