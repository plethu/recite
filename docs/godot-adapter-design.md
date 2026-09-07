# Godot adapter localisation

`ReciteDialogueCatalogResource` is the Godot-facing owner for translated
dialogue. Add it as a Resource, install a complete gettext plural rule before
its plural entries, and assign it to `ReciteDialogueNode`:

```gdscript
var catalog := ReciteDialogueCatalogResource.new()
catalog.set_plural_forms("fr", "nplurals=2; plural=(n != 1);")
catalog.add_translation("fr", "greeting", "Hello {name}.", "Bonjour {name}.", "formal")
catalog.add_plural_translation(
    "fr", "letters", "One letter.", "{count} letters.",
    ["Une lettre.", "{count} lettres."], "formal")

$ReciteDialogueNode.set_locale_catalog(catalog)
$ReciteDialogueNode.start_with_variant(dialogue, "start", "fr-CA", "formal")
```

The Resource stores entries and plural headers in serializable Godot
properties. When a Resource is loaded, the Rust adapter rebuilds a validated,
owned catalogue from those properties. Placeholder names must be preserved;
plural entries must have exactly the rule's `nplurals` arms. Empty translated
arms use the normal source-text fallback. The lookup order is explicit variant
context, unqualified context, BCP-47 locale fallback, and authored source.

The runtime snapshot preserves the selected locale and deterministic session
state, but not the Resource, interpolation values, or grammatical variant.
On restore, keep the Resource installed, restore the same typed values, and
call `restore_with_variant` with the selected variant before traversal resumes.
No catalogue lookup or traversal operation performs game-side effects.

Rust unit tests cover the catalogue's line, choice, availability-reason,
presentation-label, plural, variant, fallback, placeholder, and restore
semantics. The executable host conformance lane is
`scripts/check-godot-host.sh`. It builds the GDExtension, compiles the basic
dialogue fixture, initializes a clean Godot project, and exercises malformed
persisted array/dictionary/plural shapes, reload-before-mutation, a `.tres`
round trip, class registration, and output signal delivery. It uses the
official Godot 4.6.3 stable Linux x86_64 standard build
(`4.6.3.stable.official.7d41c59c4`), matching the crate's `api-4-6` feature;
set `GODOT` to that binary when it is not on `PATH`. The ordinary Cargo lane
remains host-independent, while this script is the required engine-hosted
evidence before changing the Resource format.
