@tool
extends RefCounted

const ExportJob = preload("res://addons/recite/schema_export_job.gd")

const SECTION_NAMES := [
	"types",
	"registries",
	"speakers",
	"conditions",
	"availability_reasons",
	"effects",
	"metadata_domains",
	"metadata",
	"projections"
]

static var _next_export := 0


static func begin_export(resource: Resource, manifest_path: String, cli_path: String) -> ExportJob:
	var prepared := _prepare(resource, manifest_path)
	return ExportJob.new(prepared, cli_path, _finish_export)


# Synchronous automation API; the editor menu owns and polls begin_export instead.
static func export_resource(
	resource: Resource, manifest_path: String, cli_path: String
) -> Dictionary:
	var job := begin_export(resource, manifest_path, cli_path)
	while not job.poll():
		OS.delay_msec(1)
	return job.result


static func _prepare(resource: Resource, manifest_path: String) -> Dictionary:
	if not resource is ReciteSchemaDeclarations:
		return {"ok": false, "error": "selected Resource is not ReciteSchemaDeclarations"}
	var declarations_resource := resource as ReciteSchemaDeclarations
	if declarations_resource.producer_id.strip_edges().is_empty():
		return {"ok": false, "error": "producer_id must be a stable nonempty ID"}
	if manifest_path.is_empty():
		return {"ok": false, "error": "recite/schema_manifest is required"}
	var snapshot := _snapshot_text(declarations_resource)
	if not snapshot.ok:
		return snapshot
	var source_text: String = snapshot.text
	var saved_path := resource.resource_path.get_slice("::", 0)
	if not saved_path.is_empty() and not FileAccess.file_exists(saved_path):
		saved_path = ""
	_next_export += 1
	var token := "%d-%d-%d" % [OS.get_process_id(), Time.get_ticks_usec(), _next_export]
	var source_path := ProjectSettings.globalize_path("user://recite-schema-export-%s.toml" % token)
	var source := FileAccess.open(source_path, FileAccess.WRITE)
	if source == null:
		return {"ok": false, "error": "cannot create temporary schema source"}
	source.store_string(source_text)
	source.close()
	var output := (
		ProjectSettings.globalize_path(manifest_path)
		if manifest_path.begins_with("res://") or manifest_path.begins_with("user://")
		else manifest_path
	)
	var staged := output.get_base_dir().path_join(".recite-schema-%s.json" % token)
	return {
		"ok": true,
		"source": source_path,
		"output": output,
		"staged": staged,
		"arguments":
		PackedStringArray(
			[
				"export-schema",
				"--schema",
				source_path,
				"--output",
				staged,
				"--producer-kind",
				"godot",
				"--output-format",
				"structured",
				"--producer-id",
				declarations_resource.producer_id
			]
		),
		"resource": declarations_resource,
		"snapshot": source_text,
		"saved_path": saved_path,
		"saved_hash": FileAccess.get_sha256(saved_path) if not saved_path.is_empty() else ""
	}


static func _finish_export(prepared: Dictionary, exit_code: int, stream: String) -> Dictionary:
	var staged: String = prepared.staged
	var output: String = prepared.output
	var result := _read_cli_result(exit_code, stream, staged)
	if not result.ok:
		if FileAccess.file_exists(staged):
			DirAccess.remove_absolute(staged)
		return result
	# Inspect the length without copying a potentially large manifest into memory.
	var artifact := FileAccess.open(staged, FileAccess.READ)
	if artifact == null or artifact.get_length() != result.artifact.size_bytes:
		return {
			"ok": false,
			"category": "protocol",
			"error": "CLI artifact does not match staged export"
		}
	artifact.close()
	# Changes made while the child ran must not publish an old declaration snapshot.
	var current := _snapshot_text(prepared.resource)
	if (
		not current.ok
		or current.text != prepared.snapshot
		or (
			not prepared.saved_path.is_empty()
			and (
				not FileAccess.file_exists(prepared.saved_path)
				or FileAccess.get_sha256(prepared.saved_path) != prepared.saved_hash
			)
		)
	):
		return {
			"ok": false,
			"category": "stale",
			"error": "schema declarations changed during export; export again"
		}
	var published := DirAccess.rename_absolute(staged, output)
	if published != OK:
		DirAccess.remove_absolute(staged)
		return {
			"ok": false,
			"category": "io",
			"error": "cannot publish schema manifest (%d)" % published
		}
	result.path = output
	return result


static func _read_cli_result(exit_code: int, stream: String, output: String) -> Dictionary:
	var records: Array = []
	for line in stream.split("\n", false):
		var record = JSON.parse_string(line)
		if not record is Dictionary:
			return {
				"ok": false,
				"category": "protocol",
				"error": "CLI emitted a non-JSON protocol record"
			}
		records.append(record)
	if (
		records.size() != 2
		or records[0].get("version") != 1
		or records[0].get("sequence") != 0
		or records[0].get("event") != "command.started"
		or records[0].get("command") != "export-schema"
	):
		return {
			"ok": false,
			"category": "protocol",
			"error": "CLI export protocol start record is invalid"
		}
	var terminal: Dictionary = records[1]
	if (
		terminal.get("version") != 1
		or terminal.get("sequence") != 1
		or terminal.get("command") != "export-schema"
		or terminal.get("exit_code") != exit_code
	):
		return {
			"ok": false,
			"category": "protocol",
			"error": "CLI export protocol terminal record is invalid"
		}
	if terminal.get("event") == "command.result":
		if terminal.get("status") == "success" and exit_code == 0:
			var data: Dictionary = terminal.get("data", {})
			if not data.has("artifact") or not data.has("diagnostics"):
				return {
					"ok": false,
					"category": "protocol",
					"error": "successful export lacks artifact metadata"
				}
			return {
				"ok": true,
				"path": output,
				"artifact": data.artifact,
				"diagnostics": data.diagnostics
			}
		if terminal.get("status") == "content_diagnostics" and exit_code == 1:
			var data: Dictionary = terminal.get("data", {})
			if not data.has("diagnostics") or data.has("artifact"):
				return {
					"ok": false,
					"category": "protocol",
					"error": "invalid content diagnostics record"
				}
			return {
				"ok": false,
				"category": "schema",
				"diagnostics": data.diagnostics,
				"error": "schema validation failed (%d diagnostics)" % data.diagnostics.size()
			}
	elif (
		terminal.get("event") == "command.error"
		and terminal.get("status") == "failure"
		and exit_code == 1
	):
		var error: Dictionary = terminal.get("error", {})
		if error.has("category") and error.has("code") and error.has("operation"):
			return {
				"ok": false,
				"category": error.category,
				"code": error.code,
				"details": error.get("details"),
				"error": "CLI %s: %s" % [error.category, error.code]
			}
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
			return {
				"ok": false,
				"error": "unsupported declaration value type %s" % type_string(typeof(value))
			}


static func _snapshot_text(declarations_resource: ReciteSchemaDeclarations) -> Dictionary:
	var lines: Array[String] = ["schema_version = 1"]
	for section in SECTION_NAMES:
		var declarations: Dictionary = declarations_resource.get(section)
		if not declarations.is_empty():
			var encoded := _toml_value(declarations)
			if not encoded.ok:
				return {
					"ok": false,
					"category": "transport_encoding",
					"error": "%s: %s" % [section, encoded.error]
				}
			lines.append("%s = %s" % [section, encoded.value])
	lines.append("")
	lines.append("[producer]")
	lines.append("id = %s" % (JSON.stringify(declarations_resource.producer_id)))
	return {"ok": true, "text": "\n".join(lines) + "\n"}
