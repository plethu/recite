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

The Resource stores entries and plural headers in exported serializable Godot
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
semantics. The Resource source tests also cover malformed persisted array,
dictionary, and plural shapes plus reload-before-mutation, but are marked
host-required because Godot's `VarArray`/`VarDictionary` property
serialisation requires an initialized Godot 4 host. The repository's headless
Rust gate therefore cannot claim that engine-hosted save/load round-trip. Run a
Godot-hosted conformance scene before shipping a Resource format change.
