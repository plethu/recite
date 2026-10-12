extends RefCounted


static func run(parent: Node) -> Array[String]:
	var failures: Array[String] = []
	var asset = ClassDB.instantiate("ReciteDialogueResource")
	if not asset.load_from_path("res://dialogue/basic.recitec").is_ok():
		failures.append("output isolation fixture should load")
		return failures
	var node = ClassDB.instantiate("ReciteDialogueNode")
	parent.add_child(node)
	var emitted: Array[Dictionary] = []
	var nested_results: Array = []
	node.output.connect(
		func(output):
			var data: Dictionary = output.data()
			emitted.append(data)
			if emitted.size() == 1:
				data["line"]["text"] = "changed by signal"
				var nested = node.select_choice("10000000000000000003")
				nested_results.append(nested)
				if nested.is_ok():
					nested.outputs()[0]["effect"]["args"][0]["value"] = "changed by caller"
			elif data["kind"] == "prompt":
				data["choices"][0]["text"] = "changed prompt choice"
	)
	var started = node.start(asset, "start", "")
	if not started.is_ok() or nested_results.is_empty() or not nested_results[0].is_ok():
		failures.append("nested output isolation operations should succeed")
		node.queue_free()
		return failures
	if started.outputs()[0]["line"]["text"] != "The signal clears.":
		failures.append("signal line mutation must not change returned output")
	if started.outputs()[1]["choices"][0]["text"] != "Inspect the relay.":
		failures.append("signal nested choice mutation must not change returned output")
	var kinds: Array = emitted.map(func(data): return data["kind"])
	if kinds != ["line", "prompt", "effect", "line", "effect"]:
		failures.append("nested output batch must follow the complete original batch")
	elif emitted[2]["effect"]["args"][0]["value"] != "relay":
		failures.append("returned nested argument mutation must not change queued signal output")
	node.queue_free()
	return failures
