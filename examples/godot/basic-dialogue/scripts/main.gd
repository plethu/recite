extends Control

var recite: ReciteDialogueNode
var asset: ReciteDialogueResource
var transcript: RichTextLabel
var choices: VBoxContainer
var status: Label
var pending_effect_id := ""

func _ready() -> void:
	recite = ReciteDialogueNode.new()
	add_child(recite)
	recite.output.connect(_on_recite_output)
	recite.adapter_error.connect(_on_recite_error)

	var root := VBoxContainer.new()
	root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	root.add_theme_constant_override("separation", 8)
	add_child(root)

	transcript = RichTextLabel.new()
	transcript.fit_content = true
	transcript.size_flags_vertical = Control.SIZE_EXPAND_FILL
	root.add_child(transcript)

	choices = VBoxContainer.new()
	root.add_child(choices)

	status = Label.new()
	root.add_child(status)

	asset = ReciteDialogueResource.new()
	var loaded := asset.load_from_path("res://dialogue/basic.recitec")
	if not loaded.is_ok():
		_show_error(loaded.error())
		return

	var started := recite.start(asset, "start", "en-GB")
	if not started.is_ok():
		_show_error(started.error())

func _on_recite_output(output: ReciteOutput) -> void:
	var data := output.data()
	match data.get("kind", ""):
		"line":
			var line := data["line"]
			transcript.append_text(line["text"] + "\n")
		"prompt":
			var line = data["line"]
			if line != null:
				transcript.append_text(line["text"] + "\n")
			_render_choices(data["choices"])
		"effect":
			var effect := data["effect"]
			transcript.append_text("[color=gray]effect: %s[/color]\n" % effect["function"])
			if effect["mode"] == "blocking":
				pending_effect_id = effect["id"]
				status.text = "Waiting for effect acknowledgement"
				recite.acknowledge_effect(pending_effect_id, true, "")
		"end":
			for effect in data["deferred_effects"]:
				transcript.append_text("[color=gray]deferred: %s[/color]\n" % effect["function"])
			status.text = "Ended"

func _render_choices(items: Array) -> void:
	for child in choices.get_children():
		child.queue_free()
	for choice in items:
		var button := Button.new()
		button.text = choice["text"]
		button.disabled = not choice["availability"]["is_available"]
		button.pressed.connect(func() -> void:
			var result := recite.select_choice(choice["id"])
			if not result.is_ok():
				_show_error(result.error())
		)
		choices.add_child(button)

func _on_recite_error(error: ReciteAdapterError) -> void:
	_show_error(error.data())

func _show_error(error: Dictionary) -> void:
	status.text = "%s: %s" % [error.get("code", "adapter_error"), error.get("message", "")]
