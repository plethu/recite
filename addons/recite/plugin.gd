@tool
extends EditorPlugin

const CompiledImporter = preload("res://addons/recite/compiled_importer.gd")
const SchemaExporter = preload("res://addons/recite/schema_export.gd")

var _importer: EditorImportPlugin


func _enter_tree() -> void:
	_importer = CompiledImporter.new()
	add_import_plugin(_importer)
	add_tool_menu_item("Export Recite Schema", _export_schema)


func _exit_tree() -> void:
	remove_tool_menu_item("Export Recite Schema")
	remove_import_plugin(_importer)
	_importer = null


func _export_schema() -> void:
	var source_path := str(ProjectSettings.get_setting("recite/schema_resource", ""))
	var output_path := str(ProjectSettings.get_setting("recite/schema_manifest", ""))
	var cli_path := str(ProjectSettings.get_setting("recite/cli_path", "recite"))
	var resource := ResourceLoader.load(source_path) if not source_path.is_empty() else null
	if resource == null:
		push_error(
			"Recite: set recite/schema_resource to a saved ReciteSchemaDeclarations Resource"
		)
		return
	var result: Dictionary = SchemaExporter.export_resource(resource, output_path, cli_path)
	if not result.ok:
		push_error("Recite schema export: %s" % result.error)
	else:
		print("Recite schema exported: %s" % output_path)
