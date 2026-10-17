@tool
extends RefCounted

const TIMEOUT_USEC := 120000000
const READ_BYTES := 32768
const READS_PER_POLL := 4
const MAX_STDERR_BYTES := 65536

var result: Dictionary = {}
var _prepared: Dictionary
var _child: Dictionary = {}
var _finish: Callable
var _started := Time.get_ticks_usec()
var _stdout: Array[PackedByteArray] = []
var _stderr := PackedByteArray()
var _stderr_truncated := false
var _closed := [false, false]
var _stopped := false


func _init(prepared: Dictionary, executable: String, finish: Callable) -> void:
	_prepared = prepared
	_finish = finish
	if not prepared.ok:
		result = prepared
		return
	_child = OS.execute_with_pipe(executable, prepared.arguments, false)
	if _child.is_empty():
		result = {"ok": false, "category": "io", "error": "cannot start schema exporter"}
		_cleanup()


func poll() -> bool:
	if not result.is_empty():
		return true
	var exited := not OS.is_process_running(_child.pid)
	_stopped = exited
	_drain(_child.stdio, 0, exited)
	_drain(_child.stderr, 1, exited)
	if Time.get_ticks_usec() - _started >= TIMEOUT_USEC:
		cancel("schema export timed out")
		result.category = "timeout"
		return true
	if not exited or not _closed.all(func(value): return value):
		return false
	var bytes := PackedByteArray()
	for chunk in _stdout:
		bytes.append_array(chunk)
	result = _finish.call(
		_prepared, OS.get_process_exit_code(_child.pid), bytes.get_string_from_utf8()
	)
	if not result.ok and not _stderr.is_empty():
		result["stderr"] = _stderr_text()
		result["stderr_truncated"] = _stderr_truncated or _stderr.size() > MAX_STDERR_BYTES
		result.error += "\n" + result.stderr
		if result.stderr_truncated:
			result.error += "\n[stderr truncated]"
	_cleanup()
	return true


func _stderr_text() -> String:
	var end := mini(MAX_STDERR_BYTES, _stderr.size())
	# Keep up to four extra bytes so truncation can stop before a UTF-8 codepoint.
	while end > 0 and end < _stderr.size() and (_stderr[end] & 0xC0) == 0x80:
		end -= 1
	return _stderr.slice(0, end).get_string_from_utf8()


func _drain(pipe: FileAccess, index: int, exited: bool) -> void:
	if _closed[index]:
		return
	for _read in READS_PER_POLL:
		var bytes := pipe.get_buffer(READ_BYTES)
		if index == 0 and not bytes.is_empty():
			_stdout.append(bytes)
		elif index == 1:
			var remaining := MAX_STDERR_BYTES + 4 - _stderr.size()
			_stderr.append_array(bytes.slice(0, remaining))
			_stderr_truncated = _stderr_truncated or bytes.size() > remaining
		var error := pipe.get_error()
		if error == ERR_FILE_EOF:
			_closed[index] = true
			return
		if bytes.is_empty():
			# Unix pipes report ERR_FILE_CANT_READ for both EAGAIN and EOF.
			# Only an empty read after confirmed child exit means drained.
			_closed[index] = exited
			return


func cancel(reason := "schema export cancelled") -> void:
	if not result.is_empty():
		return
	_stopped = not OS.is_process_running(_child.pid)
	if not _stopped:
		# OS.kill reaps the child on Unix. Querying its PID afterwards can
		# report ECHILD; remember successful termination instead.
		_stopped = OS.kill(_child.pid) == OK
	result = {"ok": false, "category": "cancelled", "error": reason}
	_cleanup()


func _cleanup() -> void:
	if not _child.is_empty():
		_child.stdio.close()
		_child.stderr.close()
		if not _stopped:
			result["retained_files"] = [_prepared.source, _prepared.staged]
			result["retained_pid"] = _child.pid
			result.error += (
				"; child %d could not be stopped; private files retained: %s"
				% [_child.pid, result.retained_files]
			)
			return
	var retained: Array[String] = []
	for path in [_prepared.get("source", ""), _prepared.get("staged", "")]:
		if not path.is_empty() and FileAccess.file_exists(path):
			if DirAccess.remove_absolute(path) != OK:
				retained.append(path)
	if not retained.is_empty():
		result["retained_files"] = retained
		push_warning("Cannot remove Recite schema export staging: %s" % retained)
	_stdout.clear()
	_stderr.clear()


func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE and result.is_empty() and not _child.is_empty():
		cancel()
