extends Node

# LLM3dEngine Bridge
# Accepts JSON commands via stdin, responds via stdout
# Protocol: one JSON object per line (newline-delimited JSON)

var coordinator: LLMCoordinator
var runtime: QuickJSRuntime
var project_dir: String = ""
var running: bool = true

func _ready():
	# Parse command-line args for project dir
	var args = OS.get_cmdline_args()
	for i in range(args.size()):
		if args[i] == "--project-dir" and i + 1 < args.size():
			project_dir = args[i + 1]

	if project_dir.is_empty():
		_send_response({"error": "No --project-dir specified"})
		get_tree().quit(1)
		return

	# Initialize coordinator
	coordinator = LLMCoordinator.new()
	coordinator.set_project_dir(project_dir)

	# Initialize QuickJS runtime
	runtime = QuickJSRuntime.new()
	runtime.initialize()

	_send_response({"type": "ready", "project_dir": project_dir})
	print("LLM3dEngine bridge ready, project: " + project_dir, " (stderr)")

func _process(_delta):
	if not running:
		return

	# Read from stdin (non-blocking)
	var line = _read_stdin_line()
	if line.is_empty():
		return

	var json = JSON.new()
	var err = json.parse(line)
	if err != OK:
		_send_response({"error": "Invalid JSON", "input": line})
		return

	var cmd = json.get_data()
	_handle_command(cmd)

func _handle_command(cmd: Dictionary):
	var action = cmd.get("action", "")

	match action:
		"compile":
			var result_json = coordinator.compile_and_check()
			var json = JSON.new()
			json.parse(result_json)
			_send_response({"type": "compile_result", "data": json.get_data()})

		"execute":
			var js_file = cmd.get("file", "")
			if js_file.is_empty():
				# Default: execute all JS in build dir
				js_file = project_dir + "/build/main.js"

			var fa = FileAccess.open(js_file, FileAccess.READ)
			if fa == null:
				_send_response({"type": "execute_result", "success": false, "error": "Cannot open: " + js_file})
				return

			var js_code = fa.get_as_text()
			fa.close()
			var result = runtime.eval_string(js_code, js_file.get_file())
			_send_response({"type": "execute_result", "success": true, "result": result})

		"screenshot":
			var width = cmd.get("width", 1280)
			var height = cmd.get("height", 720)
			var path = coordinator.take_screenshot(width, height)
			_send_response({"type": "screenshot", "path": path})

		"state":
			var state_json = coordinator.get_scene_state()
			var json = JSON.new()
			json.parse(state_json)
			_send_response({"type": "scene_state", "data": json.get_data()})

		"feedback":
			var feedback_json = coordinator.get_feedback()
			var json = JSON.new()
			json.parse(feedback_json)
			_send_response({"type": "feedback", "data": json.get_data()})

		"compile_and_run":
			# Full pipeline: compile -> execute -> screenshot
			var compile_json = coordinator.compile_and_check()
			var cjson = JSON.new()
			cjson.parse(compile_json)
			var compile_data = cjson.get_data()

			if not compile_data.get("compile_success", false):
				_send_response({"type": "compile_and_run", "stage": "compile", "success": false, "data": compile_data})
				return

			# Execute
			var js_file = compile_data.get("output_dir", project_dir + "/build") + "/main.js"
			var fa = FileAccess.open(js_file, FileAccess.READ)
			if fa == null:
				_send_response({"type": "compile_and_run", "stage": "execute", "success": false, "error": "Cannot open: " + js_file})
				return

			var js_code = fa.get_as_text()
			fa.close()
			runtime.eval_string(js_code, "main.js")

			# Wait a frame for scene to update, then screenshot
			await get_tree().process_frame
			var screenshot_path = coordinator.take_screenshot()
			var state_json = coordinator.get_scene_state()
			var sjson = JSON.new()
			sjson.parse(state_json)

			_send_response({
				"type": "compile_and_run",
				"stage": "complete",
				"success": true,
				"compile": compile_data,
				"screenshot": screenshot_path,
				"scene_state": sjson.get_data()
			})

		"quit":
			running = false
			runtime.finalize()
			_send_response({"type": "quit"})
			get_tree().quit(0)

		_:
			_send_response({"error": "Unknown action: " + action})

func _send_response(data: Dictionary):
	# Send JSON response on stdout, one line
	print(JSON.stringify(data))

func _read_stdin_line() -> String:
	# Use OS.read_string_from_stdin if available, otherwise poll
	if OS.has_method("read_string_from_stdin"):
		return OS.read_string_from_stdin().strip_edges()
	return ""
