extends Node

var failures: Array[String] = []
var output_count := 0
var last_line_text := ""
var roundtripped_catalog = null

func _ready() -> void:
	_run_catalog_persistence_checks()
	_run_node_registration_and_signal_checks()
	if failures.is_empty():
		print("Godot host conformance passed")
		get_tree().quit(0)
		return
	for failure in failures:
		push_error(failure)
	get_tree().quit(1)

func _require(condition: bool, message: String) -> void:
	if not condition:
		failures.append(message)

func _run_catalog_persistence_checks() -> void:
	_expect_catalog_rejects([42], {}, "non-dictionary persisted entry")

	var wrong_domain := {"kind": "singular", "locale": "fr", "id": "line", "source_text": "Source.", "translation": "Bonjour.", "domain": "line", "variant": ""}
	_expect_catalog_rejects([wrong_domain], {}, "wrong persisted domain type")

	var unknown_key := {"kind": "singular", "locale": "fr", "id": "line", "source_text": "Source.", "translation": "Bonjour.", "domain": 0, "variant": "", "unexpected": true}
	_expect_catalog_rejects([unknown_key], {}, "unknown persisted entry key")

	var plural_wrong_arm := {"kind": "plural", "locale": "fr", "id": "letters", "source_singular": "One letter.", "source_plural": "{count} letters.", "translations": [1], "variant": ""}
	_expect_catalog_rejects([plural_wrong_arm], {"fr": "nplurals=2; plural=(n != 1);"}, "wrong plural arm type")

	_expect_catalog_rejects([], {1: "nplurals=2; plural=(n != 1);"}, "non-string plural locale key")

	var resource = ClassDB.instantiate("ReciteDialogueCatalogResource")
	var plural_header := "nplurals=2; plural=(n != 1);"
	var plural_rule = resource.set_plural_forms("fr", plural_header)
	_require(plural_rule.is_ok(), "valid plural rule should be accepted")
	var translation = resource.add_translation("fr", "hello", "Hello.", "Bonjour.", "")
	_require(translation.is_ok(), "initial translation should be accepted")

	var entries: Array = resource.serialized_entries
	var record: Dictionary = entries[0]
	record["translation"] = "Salut."
	entries[0] = record
	resource.serialized_entries = entries
	var refreshed = resource.add_translation("fr", "hello", "Hello.", "Salut.", "")
	_require(refreshed.is_ok(), "mutating after persisted reload should use persisted fields")

	var fixture_line = resource.add_translation("fr", "10000000000000000001", "The signal clears.", "Le signal se libère.", "")
	_require(fixture_line.is_ok(), "fixture line should be accepted before save")
	var fixture_plural = resource.add_plural_translation(
		"fr", "letters", "One letter.", "{count} letters.", ["Une lettre.", "{count} lettres."], "")
	_require(fixture_plural.is_ok(), "fixture plural should be accepted before save")

	var save_path := "user://recite-catalog.tres"
	var save_error := ResourceSaver.save(resource, save_path)
	_require(save_error == OK, "catalogue Resource should save: %s" % save_error)
	var loaded := ResourceLoader.load(save_path, "", ResourceLoader.CACHE_MODE_IGNORE)
	_require(loaded != null, "catalogue Resource should reload")
	_require(loaded != resource, "reload should produce a distinct Resource instance")
	if loaded != null:
		var loaded_resource = loaded
		var loaded_entries: Array = loaded_resource.serialized_entries
		var found_fixture_line := false
		var found_fixture_plural := false
		for value in loaded_entries:
			if value is Dictionary:
				var loaded_record: Dictionary = value
				if loaded_record.get("id", "") == "10000000000000000001" and loaded_record.get("translation", "") == "Le signal se libère.":
					found_fixture_line = true
				if loaded_record.get("kind", "") == "plural" and loaded_record.get("id", "") == "letters":
					found_fixture_plural = true
		_require(found_fixture_line, "saved fixture line should survive Resource reload")
		_require(found_fixture_plural, "saved fixture plural should survive Resource reload")
		var loaded_plural_forms: Dictionary = loaded_resource.serialized_plural_forms
		_require(loaded_plural_forms.get("fr", "") == plural_header, "saved plural rule should survive Resource reload")
		roundtripped_catalog = loaded_resource

func _expect_catalog_rejects(entries: Array, plural_forms: Dictionary, label: String) -> void:
	var resource = ClassDB.instantiate("ReciteDialogueCatalogResource")
	resource.serialized_entries = entries
	resource.serialized_plural_forms = plural_forms
	var result = resource.add_translation("fr", "line", "Source.", "Bonjour.", "")
	_require(not result.is_ok(), "%s should be rejected" % label)

func _run_node_registration_and_signal_checks() -> void:
	var asset = ClassDB.instantiate("ReciteDialogueResource")
	var loaded = asset.load_from_path("res://dialogue/basic.recitec")
	_require(loaded.is_ok(), "compiled dialogue should load through the Resource binding")
	if not loaded.is_ok():
		return

	var node = ClassDB.instantiate("ReciteDialogueNode")
	add_child(node)
	node.output.connect(_on_output)
	if roundtripped_catalog != null:
		var catalog_result = node.set_locale_catalog(roundtripped_catalog)
		_require(catalog_result.is_ok(), "reloaded catalogue should install on a dialogue node")
	var started = node.start(asset, "start", "fr")
	_require(started.is_ok(), "dialogue node should start through the registered GDExtension class")
	_require(output_count == 2, "dialogue node should emit line and prompt signals")
	_require(last_line_text == "Le signal se libère.", "persisted catalogue translation should reach node output")
	node.queue_free()

func _on_output(_output) -> void:
	output_count += 1
	var data = _output.data()
	if data.get("kind", "") == "line":
		last_line_text = data["line"]["text"]
