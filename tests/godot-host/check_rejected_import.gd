extends SceneTree

func _initialize() -> void:
	var last_good = ResourceLoader.load("user://recite-asset.tres", "", ResourceLoader.CACHE_MODE_IGNORE)
	var imported = ResourceLoader.load("res://dialogue/basic.recitec", "", ResourceLoader.CACHE_MODE_IGNORE)
	if last_good is ReciteDialogueResource and imported is ReciteDialogueResource \
			and imported.content_identity() == last_good.content_identity() \
			and imported.last_error().get("code") == "asset_load_or_decode_error" \
			and imported.revision_info()["import_error"]["code"] == "asset_load_or_decode_error":
		print("Godot rejected import retained last-good")
		quit(0)
	else:
		push_error("rejected importer candidate removed or changed the previous valid revision")
		quit(1)
