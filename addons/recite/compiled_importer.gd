@tool
extends EditorImportPlugin

func _get_importer_name() -> String:
	return "recite.compiled_dialogue"

func _get_visible_name() -> String:
	return "Recite compiled dialogue"

func _get_recognized_extensions() -> PackedStringArray:
	return PackedStringArray(["recitec"])

func _get_save_extension() -> String:
	return "res"

func _get_resource_type() -> String:
	return "ReciteDialogueResource"

func _get_preset_count() -> int:
	return 1

func _get_preset_name(_preset_index: int) -> String:
	return "Default"

func _get_import_options(_path: String, _preset_index: int) -> Array[Dictionary]:
	return []

func _import(source_file: String, save_path: String, _options: Dictionary,
		_platform_variants: Array[String], _gen_files: Array[String]) -> Error:
	var file := FileAccess.open(source_file, FileAccess.READ)
	if file == null:
		push_error("Recite import: cannot read %s" % source_file)
		return _retain_previous(save_path, source_file, PackedByteArray(), true)
	var bytes := file.get_buffer(file.get_length())
	var resource := ReciteDialogueResource.new()
	var result := resource.load_from_bytes(bytes)
	if not result.is_ok():
		push_error("Recite import %s: %s" % [source_file, result.error().get("message", "invalid compiled dialogue")])
		return _retain_previous(save_path, source_file, bytes, false)
	var saved := ResourceSaver.save(resource, save_path + ".res")
	if saved != OK:
		return saved
	return _store_validated_backup(save_path, bytes)

func _retain_previous(save_path: String, source_file: String,
		rejected_bytes: PackedByteArray, unreadable: bool) -> Error:
	var previous_bytes := FileAccess.get_file_as_bytes(save_path + ".lastgood")
	if previous_bytes.is_empty():
		return ERR_FILE_CORRUPT
	var resource := ReciteDialogueResource.new()
	if not resource.load_from_bytes(previous_bytes).is_ok():
		return ERR_FILE_CORRUPT
	# Loading the rejected candidate sets a structured error without replacing
	# the valid bytes. This error is stored in the imported Resource.
	if unreadable:
		resource.load_from_path(source_file)
	else:
		resource.load_from_bytes(rejected_bytes)
	return ResourceSaver.save(resource, save_path + ".res")

func _store_validated_backup(save_path: String, bytes: PackedByteArray) -> Error:
	var staged := save_path + ".lastgood.tmp"
	var file := FileAccess.open(staged, FileAccess.WRITE)
	if file == null:
		return FileAccess.get_open_error()
	file.store_buffer(bytes)
	file.flush()
	var write_error := file.get_error()
	file.close()
	if write_error != OK:
		DirAccess.remove_absolute(staged)
		return write_error
	var published := DirAccess.rename_absolute(staged, save_path + ".lastgood")
	if published != OK:
		DirAccess.remove_absolute(staged)
	return published
