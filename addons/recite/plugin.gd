@tool
extends EditorPlugin

const CompiledImporter = preload("res://addons/recite/compiled_importer.gd")
const SchemaExporter = preload("res://addons/recite/schema_export.gd")
const SchemaExportJob = preload("res://addons/recite/schema_export_job.gd")

var _importer: EditorImportPlugin
var _schema_job: SchemaExportJob
var _schema_output := ""
var _schema_source := ""
var _schema_cli := ""


func _enter_tree() -> void:
	_importer = CompiledImporter.new()
	add_import_plugin(_importer)
	add_tool_menu_item("Export Recite Schema", _export_schema)
	set_process(false)


func _exit_tree() -> void:
	if _schema_job != null:
		_schema_job.cancel()
		_schema_job = null
	remove_tool_menu_item("Export Recite Schema")
	remove_import_plugin(_importer)
	_importer = null


func _export_schema() -> void:
	if _schema_job != null:
		push_warning("A Recite schema export is already running")
		return
	var source_path := str(ProjectSettings.get_setting("recite/schema_resource", ""))
	var output_path := str(ProjectSettings.get_setting("recite/schema_manifest", ""))
	var cli_path := str(ProjectSettings.get_setting("recite/cli_path", "recite"))
	var resource := ResourceLoader.load(source_path) if not source_path.is_empty() else null
	if resource == null:
		push_error(
			"Recite: set recite/schema_resource to a saved ReciteSchemaDeclarations Resource"
		)
		return
	_schema_output = output_path
	_schema_source = source_path
	_schema_cli = cli_path
	_schema_job = SchemaExporter.begin_export(resource, output_path, cli_path)
	set_process(true)


func _process(_delta: float) -> void:
	if _schema_job == null:
		return
	if (
		str(ProjectSettings.get_setting("recite/schema_manifest", "")) != _schema_output
		or str(ProjectSettings.get_setting("recite/schema_resource", "")) != _schema_source
		or str(ProjectSettings.get_setting("recite/cli_path", "recite")) != _schema_cli
	):
		_schema_job.cancel("schema export configuration changed; export again")
	if not _schema_job.poll():
		return
	var result: Dictionary = _schema_job.result
	_schema_job = null
	set_process(false)
	if not result.ok:
		push_error("Recite schema export: %s" % result.error)
	else:
		print("Recite schema exported: %s" % _schema_output)
