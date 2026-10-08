extends Node

var failures: Array[String] = []
var output_count := 0
var last_line_text := ""
var roundtripped_catalog = null
var reentrant_node = null
var reentrant_kinds: Array[String] = []
var reentrant_result = null


func _ready() -> void:
	_run_catalog_persistence_checks()
	_run_node_registration_and_signal_checks()
	_run_import_and_resource_checks()
	_run_refresh_and_snapshot_checks()
	_run_node_error_conformance_checks()
	_run_schema_mismatch_restore_check()
	_run_plural_conformance_checks()
	_run_localisation_error_conformance_check()
	_run_po_persistence_check()
	_run_reentrant_signal_check()
	_run_schema_export_checks()
	_run_information_profile()
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

	var wrong_domain := {
		"kind": "singular",
		"locale": "fr",
		"id": "line",
		"source_text": "Source.",
		"translation": "Bonjour.",
		"domain": "line",
		"variant": ""
	}
	_expect_catalog_rejects([wrong_domain], {}, "wrong persisted domain type")

	var unknown_key := {
		"kind": "singular",
		"locale": "fr",
		"id": "line",
		"source_text": "Source.",
		"translation": "Bonjour.",
		"domain": 0,
		"variant": "",
		"unexpected": true
	}
	_expect_catalog_rejects([unknown_key], {}, "unknown persisted entry key")

	var plural_wrong_arm := {
		"kind": "plural",
		"locale": "fr",
		"id": "letters",
		"source_singular": "One letter.",
		"source_plural": "{count} letters.",
		"translations": [1],
		"variant": ""
	}
	_expect_catalog_rejects(
		[plural_wrong_arm], {"fr": "nplurals=2; plural=(n != 1);"}, "wrong plural arm type"
	)

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

	var fixture_line = resource.add_translation(
		"fr", "10000000000000000001", "The signal clears.", "Le signal se libère.", ""
	)
	_require(fixture_line.is_ok(), "fixture line should be accepted before save")
	var fixture_plural = resource.add_plural_translation(
		"fr", "letters", "One letter.", "{count} letters.", ["Une lettre.", "{count} lettres."], ""
	)
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
				if (
					loaded_record.get("id", "") == "10000000000000000001"
					and loaded_record.get("translation", "") == "Le signal se libère."
				):
					found_fixture_line = true
				if (
					loaded_record.get("kind", "") == "plural"
					and loaded_record.get("id", "") == "letters"
				):
					found_fixture_plural = true
		_require(found_fixture_line, "saved fixture line should survive Resource reload")
		_require(found_fixture_plural, "saved fixture plural should survive Resource reload")
		var loaded_plural_forms: Dictionary = loaded_resource.serialized_plural_forms
		_require(
			loaded_plural_forms.get("fr", "") == plural_header,
			"saved plural rule should survive Resource reload"
		)
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
	_require(
		last_line_text == "Le signal se libère.",
		"persisted catalogue translation should reach node output"
	)
	node.queue_free()


func _on_output(_output) -> void:
	output_count += 1
	var data = _output.data()
	if data.get("kind", "") == "line":
		last_line_text = data["line"]["text"]


func _run_import_and_resource_checks() -> void:
	var imported = ResourceLoader.load(
		"res://dialogue/basic.recitec", "", ResourceLoader.CACHE_MODE_IGNORE
	)
	_require(
		imported is ReciteDialogueResource,
		"editor importer should load .recitec as ReciteDialogueResource"
	)
	if not imported is ReciteDialogueResource:
		return
	_require(imported.is_loaded(), "imported Resource should contain validated bytes")
	var identity: String = imported.content_identity()
	_require(not identity.is_empty(), "imported Resource should expose canonical content identity")
	var save_error := ResourceSaver.save(imported, "user://recite-asset.tres")
	_require(save_error == OK, "compiled Resource should save to .tres")
	var restored = ResourceLoader.load(
		"user://recite-asset.tres", "", ResourceLoader.CACHE_MODE_IGNORE
	)
	_require(restored is ReciteDialogueResource, "saved compiled Resource should deserialize")
	if restored is ReciteDialogueResource:
		_require(
			restored.content_identity() == identity,
			"Resource roundtrip should preserve content identity"
		)
		var rejected = restored.load_from_bytes(PackedByteArray([1, 2, 3]))
		_require(not rejected.is_ok(), "invalid refresh should fail")
		_require(
			rejected.error().get("code") == "asset_load_or_decode_error",
			"invalid refresh should have structured category"
		)
		_require(
			restored.content_identity() == identity, "invalid refresh should retain last good asset"
		)
		_require(
			restored.last_error().get("code") == "asset_load_or_decode_error",
			"Resource should retain rejected import error"
		)


func _run_refresh_and_snapshot_checks() -> void:
	var asset := ReciteDialogueResource.new()
	_require(
		asset.load_from_path("res://dialogue/runtime.recitec").is_ok(),
		"runtime fixture should load"
	)
	var baseline_identity: String = asset.content_identity()
	var node := ReciteDialogueNode.new()
	add_child(node)
	node.register_condition("trusts", func(_query): return true)
	var started = node.start(asset, "start", "")
	_require(started.is_ok(), "runtime session should start")
	_require(
		node.active_content_identity() == baseline_identity,
		"active identity should match loaded asset"
	)
	var initial_state: Dictionary = node.asset_state(asset)
	_require(
		(
			initial_state["active"]["content_identity"]
			== initial_state["available"]["content_identity"]
		),
		"structured active and available identities should initially match"
	)
	_require(
		(
			initial_state["available"]["source_freshness"] == "unavailable"
			and initial_state["available"]["schema_freshness"] == "unavailable"
		),
		"compiled-only import should not claim source or schema freshness"
	)
	var snapshot = node.snapshot()
	_require(snapshot.is_ok(), "prompt snapshot should encode")
	_require(not snapshot.snapshot_bytes().is_empty(), "prompt snapshot should contain bytes")
	_require(
		asset.load_from_path("res://dialogue/runtime_changed.recitec").is_ok(),
		"changed revision should import"
	)
	var changed_identity: String = asset.content_identity()
	_require(
		changed_identity != baseline_identity,
		"changed fixture should have distinct canonical identity"
	)
	_require(
		node.active_content_identity() == baseline_identity,
		"active session should retain original revision"
	)
	var changed_state: Dictionary = node.asset_state(asset)
	_require(
		changed_state["active_differs_from_available"],
		"structured asset state should expose pending next-session revision"
	)
	_require(
		(
			changed_state["active"]["content_identity"] == baseline_identity
			and changed_state["available"]["content_identity"] == changed_identity
		),
		"structured state should preserve canonical identities"
	)
	_require(node.end_session().is_ok(), "active session should end")
	var next_started = node.start(asset, "start", "")
	_require(next_started.is_ok(), "next session should start from changed revision")
	_require(
		node.active_content_identity() == changed_identity,
		"next session should see available revision"
	)
	_require(node.end_session().is_ok(), "changed session should end")
	var restored = node.restore(asset, snapshot.snapshot_bytes())
	_require(not restored.is_ok(), "old snapshot should reject incompatible changed revision")
	_require(
		(
			restored.error().get("code") == "save_load_incompatibility_error"
			or restored.error().get("code") == "stale_or_incompatible_asset_error"
		),
		"snapshot incompatibility should have structured category"
	)
	_require(
		asset.load_from_path("res://dialogue/runtime.recitec").is_ok(),
		"original revision should reload"
	)
	var compatible = node.restore(asset, snapshot.snapshot_bytes())
	_require(compatible.is_ok(), "prompt snapshot should restore against original revision")
	node.queue_free()


func _run_node_error_conformance_checks() -> void:
	var asset := ReciteDialogueResource.new()
	_require(
		asset.load_from_path("res://dialogue/runtime.recitec").is_ok(), "error fixture should load"
	)
	var node := ReciteDialogueNode.new()
	add_child(node)
	var observed: Array = []
	node.output.connect(func(output): observed.append(output.data()))
	var unknown_block = node.start(asset, "missing", "")
	_require(
		(
			not unknown_block.is_ok()
			and unknown_block.error().get("code") == "unknown_start_block_error"
		),
		"unknown start block should have stable category"
	)
	var missing = node.start(asset, "start", "")
	_require(
		not missing.is_ok() and missing.error().get("code") == "missing_condition_handler_error",
		"unregistered condition should have structured category"
	)
	_require(
		missing.outputs().is_empty() and observed.is_empty(),
		"failed start should not emit a partial batch"
	)
	node.register_condition("trusts", func(_query): return false)
	var started = node.start(asset, "start", "")
	_require(started.is_ok(), "conditioned scenario should start")
	_require(
		not node.start(asset, "start", "").is_ok(), "second start should reject active session"
	)
	var second = node.start(asset, "start", "")
	_require(
		second.error().get("code") == "session_already_active_error",
		"second start should carry stable category"
	)
	var unknown = node.select_choice("missing_choice")
	_require(
		not unknown.is_ok() and unknown.error().get("code") == "invalid_choice_error",
		"unknown choice should carry stable category"
	)
	var unavailable = node.select_choice("8706986735003ec9aaec")
	_require(
		not unavailable.is_ok() and unavailable.error().get("code") == "unavailable_choice_error",
		"conditioned choice should be unavailable"
	)
	var work = node.select_choice("3f481d9991fa22e23b0c")
	_require(
		work.is_ok() and work.outputs().size() == 1 and work.outputs()[0].get("kind") == "effect",
		"work route should request blocking effect"
	)
	if work.is_ok() and not work.outputs().is_empty():
		var effect_id: String = work.outputs()[0]["effect"]["id"]
		var wrong_ack = node.acknowledge_effect("wrong", true, "")
		_require(
			(
				not wrong_ack.is_ok()
				and wrong_ack.error().get("code") == "effect_acknowledgement_error"
			),
			"wrong effect ID should be rejected"
		)
		var acknowledged = node.acknowledge_effect(effect_id, true, "")
		_require(
			acknowledged.is_ok() and acknowledged.outputs()[0].get("kind") == "prompt",
			"correct effect acknowledgement should reach later prompt"
		)
		var stale = node.select_choice("3f481d9991fa22e23b0c")
		_require(
			not stale.is_ok() and stale.error().get("code") == "stale_choice_error",
			"previously observed ID should be stale after later prompt"
		)
	_require(node.end_session().is_ok(), "error scenario should end")
	var no_active = node.select_choice("ced14b8623bba2198c6d")
	_require(
		not no_active.is_ok() and no_active.error().get("code") == "no_active_session_error",
		"operation after end should identify missing session"
	)
	var invalid_snapshot = node.restore(asset, PackedByteArray([1, 2, 3]))
	_require(
		(
			not invalid_snapshot.is_ok()
			and invalid_snapshot.error().get("code") == "save_load_incompatibility_error"
		),
		"malformed snapshot should have stable category"
	)
	node.queue_free()
	var invalid_result_node := ReciteDialogueNode.new()
	add_child(invalid_result_node)
	invalid_result_node.register_condition("trusts", func(_query): return "not a bool")
	var invalid_result = invalid_result_node.start(asset, "start", "")
	_require(
		(
			not invalid_result.is_ok()
			and invalid_result.error().get("code") == "invalid_condition_result_error"
		),
		"wrong callable type should have stable category"
	)
	invalid_result_node.queue_free()
	var failed_condition_node := ReciteDialogueNode.new()
	add_child(failed_condition_node)
	failed_condition_node.register_condition(
		"trusts",
		func(_query):
			var failure := ReciteConditionFailure.new()
			failure.message = "game query failed"
			return failure
	)
	var failed_condition = failed_condition_node.start(asset, "start", "")
	_require(
		(
			not failed_condition.is_ok()
			and failed_condition.error().get("code") == "condition_evaluation_error"
		),
		"explicit callable failure should have stable category"
	)
	_require(
		failed_condition.error().get("message", "").contains("game query failed"),
		"condition failure should retain caller detail"
	)
	failed_condition_node.queue_free()


func _run_schema_mismatch_restore_check() -> void:
	var first := ReciteDialogueResource.new()
	var second := ReciteDialogueResource.new()
	_require(
		first.load_from_path("res://dialogue/schema_a.recitec").is_ok(),
		"first schema fixture should load"
	)
	_require(
		second.load_from_path("res://dialogue/schema_b.recitec").is_ok(),
		"second schema fixture should load"
	)
	var node := ReciteDialogueNode.new()
	add_child(node)
	_require(node.start(first, "start", "").is_ok(), "schema fixture should reach prompt")
	var snapshot = node.snapshot()
	_require(snapshot.is_ok(), "schema fixture should snapshot")
	_require(node.end_session().is_ok(), "schema fixture should end")
	var rejected = node.restore(second, snapshot.snapshot_bytes())
	_require(
		not rejected.is_ok() and rejected.error().get("code") == "schema_mismatch_error",
		"restore against different canonical schema should reject with schema_mismatch_error"
	)
	node.queue_free()


func _run_plural_conformance_checks() -> void:
	var asset := ReciteDialogueResource.new()
	_require(
		asset.load_from_path("res://dialogue/plural.recitec").is_ok(), "plural fixture should load"
	)
	var catalog := ReciteDialogueCatalogResource.new()
	_require(
		catalog.set_plural_forms("fr-FR", "nplurals=2; plural=(n != 1);").is_ok(),
		"French plural rule should load"
	)
	_require(
		(
			catalog
			. add_plural_translation(
				"fr-FR",
				"5fcf9a1f7b20211f4a92",
				"You have one letter.",
				"You have {count} letters.",
				["Vous avez une lettre.", "Vous avez {count} lettres."],
				""
			)
			. is_ok()
		),
		"French plural arms should load"
	)
	var node := ReciteDialogueNode.new()
	add_child(node)
	_require(node.set_locale_catalog(catalog).is_ok(), "catalog should install")
	_require(node.set_interpolation_values({"count": 2}).is_ok(), "count binding should install")
	var started = node.start(asset, "start", "fr-FR")
	_require(started.is_ok(), "plural scenario should start")
	if started.is_ok():
		var outputs: Array = started.outputs()
		_require(
			outputs.size() >= 1 and outputs[0].get("kind") == "line",
			"plural scenario should emit a line"
		)
		if outputs.size() >= 1 and outputs[0].get("kind") == "line":
			var line: Dictionary = outputs[0]["line"]
			var plural: Dictionary = line["plural"]
			var resolution: Dictionary = plural["resolution"]
			_require(line["text"] == "Vous avez 2 lettres.", "French count should render")
			_require(
				plural["singular_source_text"] == "You have one letter.",
				"singular source should be retained"
			)
			_require(
				plural["plural_source_text"] == "You have {count} letters.",
				"plural source should be retained"
			)
			_require(
				plural["count"] == 2 and plural["selected_arm"] == 1,
				"count and selected arm should be structured"
			)
			_require(resolution["outcome"] == "translated", "plural outcome should be translated")
			_require(
				(
					resolution["matched_locale"] == "fr-FR"
					and resolution["matched_context"] == "5fcf9a1f7b20211f4a92"
				),
				"matched locale and context should be structured"
			)
			_require(
				(
					resolution["matched_key"] == "5fcf9a1f7b20211f4a92"
					and resolution["matched_arm"] == 1
				),
				"matched key and arm should be structured"
			)
			_require(
				resolution["source_fallback_arm"] == null,
				"translated line should not claim source fallback"
			)
			var attempts: Array = resolution["attempts"]
			_require(
				(
					attempts.size() == 1
					and (
						attempts[0]
						== {
							"locale": "fr-FR",
							"context": "5fcf9a1f7b20211f4a92",
							"key": "5fcf9a1f7b20211f4a92",
							"selected_arm": 1,
							"outcome": "matched"
						}
					)
				),
				"ordered lookup provenance should match conformance manifest"
			)
	node.queue_free()
	var fallback := ReciteDialogueNode.new()
	add_child(fallback)
	_require(
		fallback.set_interpolation_values({"count": 2}).is_ok(), "fallback count should install"
	)
	var source = fallback.start(asset, "start", "")
	_require(source.is_ok(), "source fallback session should start")
	if source.is_ok():
		var line: Dictionary = source.outputs()[0]["line"]
		var plural: Dictionary = line["plural"]
		_require(
			line["text"] == "You have 2 letters.",
			"missing catalogue should render authored plural source"
		)
		_require(
			(
				plural["resolution"]["outcome"] == "english_source_fallback"
				and plural["resolution"]["source_fallback_arm"] == 1
			),
			"source fallback provenance should be explicit"
		)
	fallback.queue_free()


func _run_localisation_error_conformance_check() -> void:
	var catalog := ReciteDialogueCatalogResource.new()
	var result = catalog.load_po_from_bytes(
		"fr-FR", "malformed.po", 'msgid "unfinished'.to_utf8_buffer()
	)
	_require(not result.is_ok(), "malformed PO should fail through Godot Resource")
	_require(
		result.error().get("code") == "localisation_error",
		"malformed PO should return structured localisation_error"
	)


func _run_po_persistence_check() -> void:
	var catalog := ReciteDialogueCatalogResource.new()
	var po := (
		'msgctxt "10000000000000000001"\n'
		+ 'msgid "The signal clears."\n'
		+ 'msgstr "Le signal se libère."\n'
	)
	var imported = catalog.load_po_from_bytes("fr-FR", "demo.po", po.to_utf8_buffer())
	_require(imported.is_ok(), "valid PO should load through catalogue Resource")
	if not imported.is_ok():
		return
	_require(
		ResourceSaver.save(catalog, "user://po-catalog.tres") == OK, "PO catalogue should save"
	)
	var restored = ResourceLoader.load(
		"user://po-catalog.tres", "", ResourceLoader.CACHE_MODE_IGNORE
	)
	_require(restored is ReciteDialogueCatalogResource, "PO catalogue should deserialize")
	if not restored is ReciteDialogueCatalogResource:
		return
	var asset := ReciteDialogueResource.new()
	_require(asset.load_from_path("res://dialogue/basic.recitec").is_ok(), "PO fixture should load")
	var node := ReciteDialogueNode.new()
	add_child(node)
	_require(node.set_locale_catalog(restored).is_ok(), "deserialized PO should install")
	var started = node.start(asset, "start", "fr-FR")
	_require(started.is_ok(), "PO session should start")
	if started.is_ok():
		_require(
			started.outputs()[0]["line"]["text"] == "Le signal se libère.",
			"PO translation should survive Resource persistence"
		)
	node.queue_free()


func _run_reentrant_signal_check() -> void:
	var asset := ReciteDialogueResource.new()
	_require(
		asset.load_from_path("res://dialogue/basic.recitec").is_ok(), "reentry fixture should load"
	)
	reentrant_node = ReciteDialogueNode.new()
	add_child(reentrant_node)
	reentrant_node.output.connect(_on_reentrant_output)
	var started = reentrant_node.start(asset, "start", "")
	_require(started.is_ok(), "reentrant start should succeed")
	_require(
		reentrant_result != null and reentrant_result.is_ok(),
		"choice requested from signal should succeed"
	)
	_require(
		reentrant_kinds.slice(0, 5) == ["line", "prompt", "effect", "line", "effect"],
		"nested output batch should follow full original batch"
	)
	reentrant_node.queue_free()
	reentrant_node = null


func _on_reentrant_output(output: ReciteOutput) -> void:
	var kind: String = output.kind()
	reentrant_kinds.append(kind)
	if reentrant_kinds.size() == 1:
		reentrant_result = reentrant_node.select_choice("10000000000000000003")


func _run_schema_export_checks() -> void:
	var Exporter = preload("res://addons/recite/schema_export.gd")
	var declaration := ReciteSchemaDeclarations.new()
	declaration.producer_id = "project-dialogue-schema"
	declaration.types = {"mood": {"kind": "enum", "values": ["calm", "tense"]}}
	var output_path := ProjectSettings.globalize_path("user://native-schema.json")
	var cli_path := OS.get_environment("RECITE_CLI")
	var result: Dictionary = Exporter.export_resource(declaration, output_path, cli_path)
	_require(
		result.ok, "native schema Resource should export canonically: %s" % result.get("error", "")
	)
	if not result.ok:
		return
	var before := FileAccess.get_file_as_string(output_path)
	var manifest = JSON.parse_string(before)
	_require(manifest is Dictionary, "schema manifest should be JSON")
	if manifest is Dictionary:
		_require(
			manifest["producer"] == {"kind": "godot", "id": "project-dialogue-schema"},
			"manifest should carry native producer identity"
		)
		_require(
			manifest["producer_fingerprints"][0]["kind"] == "godot",
			"producer fingerprint should be canonical"
		)
		_require(
			manifest["types"]["mood"]["values"] == ["calm", "tense"],
			"schema declarations should survive export"
		)
	declaration.types = {"bad": Vector3.ONE}
	var transport_error: Dictionary = Exporter.export_resource(declaration, output_path, cli_path)
	_require(
		not transport_error.ok and transport_error.get("category") == "transport_encoding",
		"unsupported Variant should be a transport error"
	)
	_require(
		FileAccess.get_file_as_string(output_path) == before,
		"transport error should preserve published schema"
	)
	declaration.types = {1: {"kind": "enum", "values": ["calm"]}}
	var key_error: Dictionary = Exporter.export_resource(declaration, output_path, cli_path)
	_require(
		not key_error.ok and key_error.get("category") == "transport_encoding",
		"non-string declaration key should be a transport error"
	)
	declaration.types = {"mood": {"kind": "unexpected", "values": ["calm"]}}
	var invalid: Dictionary = Exporter.export_resource(declaration, output_path, cli_path)
	_require(
		not invalid.ok and invalid.get("category") == "schema",
		"invalid declaration should return structured schema diagnostics"
	)
	_require(
		invalid.get("diagnostics", []).size() > 0, "invalid declaration should carry diagnostics"
	)
	_require(
		FileAccess.get_file_as_string(output_path) == before,
		"failed canonical export should preserve previous manifest"
	)
	declaration.types = {"mood": {"kind": "enum", "values": ["calm", "tense"]}}
	var malformed_protocol: Dictionary = Exporter.export_resource(
		declaration, output_path, "/bin/true"
	)
	_require(
		not malformed_protocol.ok and malformed_protocol.get("category") == "protocol",
		"malformed CLI response should be rejected"
	)
	_require(
		FileAccess.get_file_as_string(output_path) == before,
		"malformed protocol should preserve previous manifest"
	)


func _run_information_profile() -> void:
	var bytes := FileAccess.get_file_as_bytes("res://dialogue/runtime.recitec")
	const SAMPLES := 25
	var start_us := Time.get_ticks_usec()
	for _index in SAMPLES:
		var asset := ReciteDialogueResource.new()
		asset.load_from_bytes(bytes)
	var load_us := Time.get_ticks_usec() - start_us
	var asset := ReciteDialogueResource.new()
	asset.load_from_bytes(bytes)
	var node := ReciteDialogueNode.new()
	add_child(node)
	node.register_condition("trusts", func(_query): return true)
	start_us = Time.get_ticks_usec()
	for _index in SAMPLES:
		node.start(asset, "start", "")
		node.end_session()
	var event_conversion_us := Time.get_ticks_usec() - start_us
	start_us = Time.get_ticks_usec()
	for _index in SAMPLES:
		node.start(asset, "start", "")
		node.select_choice("3f481d9991fa22e23b0c")
		node.end_session()
	var condition_effect_us := Time.get_ticks_usec() - start_us
	start_us = Time.get_ticks_usec()
	for _index in 500:
		node.notification(Node.NOTIFICATION_PROCESS)
	var inactive_500_us := Time.get_ticks_usec() - start_us
	node.queue_free()
	print(
		(
			"Godot informational profile: "
			+ JSON.stringify(
				{
					"engine": "4.6.3",
					"samples": SAMPLES,
					"load_total_us": load_us,
					"event_conversion_total_us": event_conversion_us,
					"condition_effect_total_us": condition_effect_us,
					"inactive_500_callbacks_us": inactive_500_us
				}
			)
		)
	)
