extends RefCounted

const Exporter = preload("res://addons/recite/schema_export.gd")


static func run(parent: Node) -> Array[String]:
	var failures: Array[String] = []
	var root := ProjectSettings.globalize_path("user://polled-export")
	DirAccess.make_dir_recursive_absolute(root)
	var executable := root.path_join("gated.py")
	var release := root.path_join("release")
	var fail_child := root.path_join("fail")
	var cli := OS.get_environment("RECITE_CLI")
	var script := FileAccess.open(executable, FileAccess.WRITE)
	script.store_string(
		(
			"#!/usr/bin/env python3\nimport os,sys,time\n"
			+ "sys.stderr.write('E' + 'é' * 200000 + '\\n'); sys.stderr.flush()\n"
			+ "deadline=time.monotonic()+10\n"
			+ "while not os.path.exists(%s):\n" % JSON.stringify(release)
			+ " if time.monotonic()>deadline: sys.exit(52)\n time.sleep(.001)\n"
			+ "if os.path.exists(%s): sys.exit(53)\n" % JSON.stringify(fail_child)
			+ "os.execv(%s, [%s]+sys.argv[1:])\n" % [JSON.stringify(cli), JSON.stringify(cli)]
		)
	)
	script.close()
	OS.execute("chmod", ["+x", executable])
	var resource := ReciteSchemaDeclarations.new()
	resource.producer_id = "polled-schema"
	resource.types = {"mood": {"kind": "enum", "values": ["calm", "tense"]}}
	for index in 1000:
		resource.conditions["gate_%04d" % index] = {"params": [], "returns": "bool"}
	var output := root.path_join("schema.json")
	var saved := "previous manifest"
	var saved_resource := root.path_join("declarations.tres")
	ResourceSaver.save(resource, saved_resource)
	resource.take_over_path(saved_resource)
	for mode in [
		"success", "stale", "saved_stale", "saved_deleted", "failed", "cancelled", "timeout"
	]:
		if not FileAccess.file_exists(saved_resource):
			ResourceSaver.save(resource, saved_resource)
		var file := FileAccess.open(output, FileAccess.WRITE)
		file.store_string(saved)
		file.close()
		if mode == "failed":
			var marker := FileAccess.open(fail_child, FileAccess.WRITE)
			marker.close()
		var job := Exporter.begin_export(resource, output, executable)
		var private_files := [job._prepared.source, job._prepared.staged]
		# A gated child cannot finish. Native scene frames must keep advancing
		# while both pipes are drained, including stderr larger than a pipe buffer.
		for _frame in 30:
			if job.poll():
				failures.append("schema export completed before its child was released")
				break
			if job._stderr.size() > job.MAX_STDERR_BYTES + 4:
				failures.append("schema export retained unbounded stderr")
			await parent.get_tree().process_frame
		if FileAccess.get_file_as_string(output) != saved:
			failures.append("running schema export changed the published file")
		if mode == "cancelled":
			job.cancel()
		elif mode == "timeout":
			job._started = Time.get_ticks_usec() - job.TIMEOUT_USEC - 1
			job.poll()
		else:
			if mode == "saved_stale":
				var changed := ReciteSchemaDeclarations.new()
				changed.producer_id = "externally-changed"
				ResourceSaver.save(changed, saved_resource)
			if mode == "saved_deleted":
				DirAccess.remove_absolute(saved_resource)
			if mode == "stale":
				resource.types = {"mood": {"kind": "enum", "values": ["changed"]}}
			file = FileAccess.open(release, FileAccess.WRITE)
			file.close()
		var deadline := Time.get_ticks_msec() + 10000
		while not job.poll() and Time.get_ticks_msec() < deadline:
			await parent.get_tree().process_frame
		if not job.poll():
			failures.append("schema export job did not terminate")
			job.cancel()
		elif mode == "success":
			var manifest = JSON.parse_string(FileAccess.get_file_as_string(output))
			if (
				not job.result.ok
				or not manifest is Dictionary
				or manifest.get("conditions", {}).size() != 1000
			):
				failures.append(
					"released schema export did not publish a manifest: %s" % job.result
				)
		elif mode == "failed":
			if (
				job.result.ok
				or not job.result.get("stderr_truncated", false)
				or not job.result.get("stderr", "").begins_with("Eé")
				or job.result.get("stderr", "").to_utf8_buffer().size() > job.MAX_STDERR_BYTES
			):
				failures.append("failed export lost its bounded stderr: %s" % job.result)
		elif (
			job.result.ok
			or (
				job.result.get("category")
				!= ("stale" if mode in ["saved_stale", "saved_deleted"] else mode)
			)
		):
			failures.append("schema export lost its %s result: %s" % [mode, job.result])
		for path in private_files:
			if FileAccess.file_exists(path):
				failures.append("schema export retained a private file: %s" % path)
		if mode != "success" and FileAccess.get_file_as_string(output) != saved:
			failures.append("cancelled/stale export replaced the previous manifest")
		if FileAccess.file_exists(fail_child):
			DirAccess.remove_absolute(fail_child)
		if FileAccess.file_exists(release):
			DirAccess.remove_absolute(release)
	DirAccess.remove_absolute(saved_resource)
	DirAccess.remove_absolute(executable)
	DirAccess.remove_absolute(output)
	return failures
