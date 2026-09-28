@tool
extends RefCounted

const SECTION_NAMES := ["types", "registries", "speakers", "conditions",
	"availability_reasons", "effects", "metadata_domains", "metadata", "projections"]

static func export_resource(resource: Resource, manifest_path: String, cli_path: String) -> Dictionary:
	if not resource is ReciteSchemaDeclarations:
		return {"ok": false, "error": "selected Resource is not ReciteSchemaDeclarations"}
	var declarations_resource := resource as ReciteSchemaDeclarations
	if declarations_resource.producer_id.strip_edges().is_empty():
		return {"ok": false, "error": "producer_id must be a stable nonempty ID"}
	if manifest_path.is_empty():
		return {"ok": false, "error": "recite/schema_manifest is required"}
	var lines: Array[String] = ["schema_version = 1"]
	for section in SECTION_NAMES:
		var declarations: Dictionary = declarations_resource.get(section)
		if not declarations.is_empty():
			var encoded := _toml_value(declarations)
			if not encoded.ok:
				return {"ok": false, "category": "transport_encoding", "error": "%s: %s" % [section, encoded.error]}
			lines.append("%s = %s" % [section, encoded.value])
	lines.append("")
	lines.append("[producer]")
	lines.append("id = %s" % (JSON.stringify(declarations_resource.producer_id)))
	var source_path := ProjectSettings.globalize_path("user://recite-schema-export.toml")
	var source := FileAccess.open(source_path, FileAccess.WRITE)
	if source == null:
		return {"ok": false, "error": "cannot create temporary schema source"}
	source.store_string("\n".join(lines) + "\n")
	source.close()
	var output := ProjectSettings.globalize_path(manifest_path) if manifest_path.begins_with("res://") or manifest_path.begins_with("user://") else manifest_path
	var staged := output + ".recite-stage-%d.json" % Time.get_ticks_usec()
	var command_output: Array = []
	var exit_code := OS.execute(cli_path, ["export-schema", "--schema", source_path,
		"--output", staged, "--producer-kind", "godot", "--output-format", "structured",
		"--producer-id", declarations_resource.producer_id], command_output)
	DirAccess.remove_absolute(source_path)
	var result := _read_cli_result(exit_code, "\n".join(command_output), staged)
	if not result.ok:
		if FileAccess.file_exists(staged):
			DirAccess.remove_absolute(staged)
		return result
	if not FileAccess.file_exists(staged) or FileAccess.get_file_as_bytes(staged).size() != result.artifact.size_bytes:
		if FileAccess.file_exists(staged):
			DirAccess.remove_absolute(staged)
		return {"ok": false, "category": "protocol", "error": "CLI artifact does not match staged export"}
	var published := DirAccess.rename_absolute(staged, output)
	if published != OK:
		DirAccess.remove_absolute(staged)
		return {"ok": false, "category": "io", "error": "cannot publish schema manifest (%d)" % published}
	result.path = output
	return result

static func _read_cli_result(exit_code: int, stream: String, output: String) -> Dictionary:
	var records: Array = []
	for line in stream.split("\n", false):
		var record = JSON.parse_string(line)
		if not record is Dictionary:
			return {"ok": false, "category": "protocol", "error": "CLI emitted a non-JSON protocol record"}
		records.append(record)
	if records.size() != 2 or records[0].get("version") != 1 or records[0].get("sequence") != 0 \
			or records[0].get("event") != "command.started" or records[0].get("command") != "export-schema":
		return {"ok": false, "category": "protocol", "error": "CLI export protocol start record is invalid"}
	var terminal: Dictionary = records[1]
	if terminal.get("version") != 1 or terminal.get("sequence") != 1 or terminal.get("command") != "export-schema" \
			or terminal.get("exit_code") != exit_code:
		return {"ok": false, "category": "protocol", "error": "CLI export protocol terminal record is invalid"}
	if terminal.get("event") == "command.result":
		if terminal.get("status") == "success" and exit_code == 0:
			var data: Dictionary = terminal.get("data", {})
			if not data.has("artifact") or not data.has("diagnostics"):
				return {"ok": false, "category": "protocol", "error": "successful export lacks artifact metadata"}
			return {"ok": true, "path": output, "artifact": data.artifact, "diagnostics": data.diagnostics}
		if terminal.get("status") == "content_diagnostics" and exit_code == 1:
			var data: Dictionary = terminal.get("data", {})
			if not data.has("diagnostics") or data.has("artifact"):
				return {"ok": false, "category": "protocol", "error": "invalid content diagnostics record"}
			return {"ok": false, "category": "schema", "diagnostics": data.diagnostics,
				"error": "schema validation failed (%d diagnostics)" % data.diagnostics.size()}
	elif terminal.get("event") == "command.error" and terminal.get("status") == "failure" and exit_code == 1:
		var error: Dictionary = terminal.get("error", {})
		if error.has("category") and error.has("code") and error.has("operation"):
			return {"ok": false, "category": error.category, "code": error.code,
				"details": error.get("details"), "error": "CLI %s: %s" % [error.category, error.code]}
	return {"ok": false, "category": "protocol", "error": "CLI export protocol result is invalid"}

static func _toml_value(value: Variant) -> Dictionary:
	match typeof(value):
		TYPE_STRING, TYPE_STRING_NAME:
			return {"ok": true, "value": JSON.stringify(str(value))}
		TYPE_BOOL:
			return {"ok": true, "value": "true" if value else "false"}
		TYPE_INT, TYPE_FLOAT:
			return {"ok": true, "value": str(value)}
		TYPE_ARRAY, TYPE_PACKED_STRING_ARRAY:
			var parts: Array[String] = []
			for item in value:
				var encoded := _toml_value(item)
				if not encoded.ok:
					return encoded
				parts.append(encoded.value)
			return {"ok": true, "value": "[%s]" % ", ".join(parts)}
		TYPE_DICTIONARY:
			var keys: Array = value.keys()
			for key in keys:
				if typeof(key) != TYPE_STRING and typeof(key) != TYPE_STRING_NAME:
					return {"ok": false, "error": "declaration keys must be strings"}
			keys.sort()
			var parts: Array[String] = []
			for key in keys:
				var encoded := _toml_value(value[key])
				if not encoded.ok:
					return encoded
				parts.append("%s = %s" % [JSON.stringify(str(key)), encoded.value])
			return {"ok": true, "value": "{%s}" % ", ".join(parts)}
		_:
			return {"ok": false, "error": "unsupported declaration value type %s" % type_string(typeof(value))}
