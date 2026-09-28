---
title: Unity
description: Install and use the Unity GameObject adapter.
---

Recite's Unity Package Manager package focuses on Unity 6.7+, with tested,
best-effort Mono compatibility on Unity 2022.3. The current bundle includes a
Linux x86_64 native plugin for the Editor and standalone player. The base
package has no Entities dependency.

Build the tarball from the Recite checkout and add it through Unity Package
Manager's **Add package from tarball** action:

```bash
CARGO_TARGET_DIR=/path/on/disk/recite-target scripts/unity/build-upm.sh
```

Import the **Basic Dialogue** sample, then run **Tools > Recite > Migrate Legacy
Compiled Asset References** so Unity binds the imported object's actual local
ID. It shows a `ReciteDialogueRunner` scene
with an imported `.recitec` asset, condition registration, structured output,
choice selection, blocking effect acknowledgement, and snapshot restore.
Register handlers in `Start` or later, after the runner's `Awake`.

For translated dialogue, pass PO bytes to `SetPoCatalog` before `Start`, then
choose a locale. Recite's native parser owns gettext validation, locale
fallback, plural selection, and translation lookup. Missing translations use
authored source text. A failed catalogue update retains the last valid one.

For project schema, create a `ReciteSchemaRegistration` asset and use
**Tools > Recite > Export Schema**. The editor passes the asset's stable GUID
to the `recite export-schema` CLI, which validates and writes the canonical
manifest. Build dialogue with the CLI or `recite watch`; Unity imports valid
compiled `.recitec` files. A rejected reimport uses a GUID-keyed last-valid
copy in `Library/Recite/CompiledCache` when available. Active sessions keep
the revision they started with, while a new session takes the latest valid
available revision. Deleting `Library` removes this derived fallback cache.

Projects upgrading scenes that referenced `.recitec` as a `TextAsset` should
keep the file's `.meta` GUID and run **Tools > Recite > Migrate Legacy Compiled
Asset References**. The command backs up changed scene and prefab YAML in
`Library/Recite/MigrationBackups`.

Follow the [authoring walkthrough](/adapters/authoring/) to try an edit,
diagnostic, rebuild, and session restart.

The tested versions are 6000.7.0b2 and 2022.3.62f3 on Linux x86_64. Both pass
EditMode and PlayMode tests; desktop player checks pass on 6.7 IL2CPP and
2022.3 Mono. A separate 6.7 CoreCLR player check also passes, but that backend
is experimental. Other platforms have not been verified.

See the [package README](https://github.com/plethu/recite/tree/main/Packages/com.recite.dialogue)
for APIs, build commands, and the support matrix. The
shared [adapter contract](https://github.com/plethu/recite/blob/main/docs/engine-adapter-contract.md) defines Recite's
cross-engine behavior.
