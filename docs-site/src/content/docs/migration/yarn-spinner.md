---
title: Yarn Spinner imports
description: Inspect Yarn source and convert plain nodes, line IDs and static jumps.
---

```sh
recite import fixtures/import/nodes.yarn --from yarn
```

The checked subset accepts `title: Name`, `---`, plain body lines and a closing
`===`. A body may finish with a static `<<jump Target>>`. A speaker ID followed
by `: ` becomes the structured speaker, including Unicode names such as `Élodie`.
IDs must fit one bare native header value; whitespace, control characters,
quotes, backslashes, brackets and parentheses require manual migration.
A trailing `#line:ID` is retained or mapped according
to the [import ID rules](/migration/importer-boundaries/).

```text
title: Start
---
Operator: The station is quiet. #line:11111111111111111111
<<jump End>>
===
```

Write the node delimiters and body at the start of each line. The checked fixture contains a
complete two-node document with the target present. Plain `//` comment lines are
ignored.

Options, indented bodies, expressions, commands other than static jumps, general
tags, custom headers, saliency, detours and once/visit behavior require manual
migration. The reader holds back the affected node and reports its lines; it
does not remove a conditional command and import its body unconditionally.
Incomplete nodes are invalid. A jump into a held-back or missing node fails
native validation.

Map game commands to schema-checked effects and state reads to conditions after
review. Yarn variable storage, presenters, localisation databases and save data
are not imported. The source reader does not execute Yarn commands.

References: Yarn Spinner's [nodes and lines](https://docs.yarnspinner.dev/2.2/getting-started/writing-in-yarn/lines-nodes-and-options)
and [tags and metadata](https://docs.yarnspinner.dev/write-yarn-scripts/advanced-scripting/tags-metadata).
These references explain syntax; the supported input contract is the bounded
fixture above.
