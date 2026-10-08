extends SceneTree


func _initialize() -> void:
	var scene = load("res://main.tscn")
	if not scene is PackedScene:
		push_error("basic example scene did not load")
		quit(1)
		return
	var instance = scene.instantiate()
	root.add_child.call_deferred(instance)
	call_deferred("_inspect", instance)


func _inspect(instance: Node) -> void:
	var running := false
	for child in instance.get_children():
		if child is ReciteDialogueNode and not child.active_content_identity().is_empty():
			running = true
	if running:
		print("Godot packaged example started")
		quit(0)
	else:
		push_error("basic example did not start an imported dialogue session")
		quit(1)
