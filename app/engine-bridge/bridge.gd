extends Node

# LLM3dEngine Bridge — v1 simple mode
# Accepts JS file path via command line, executes it, takes screenshot, quits
# Usage: godot --path bridge_project -- --project-dir /path/to/game --run /path/to/main.js

var coordinator: LLMCoordinator
var runtime: QuickJSRuntime
var project_dir: String = ""
var js_file: String = ""
var mode: String = "run"  # "run" = execute+screenshot+quit, "watch" = stay alive and poll

func _ready():
	var args = OS.get_cmdline_user_args()
	for i in range(args.size()):
		if args[i] == "--project-dir" and i + 1 < args.size():
			project_dir = args[i + 1]
		elif args[i] == "--run" and i + 1 < args.size():
			js_file = args[i + 1]
		elif args[i] == "--watch":
			mode = "watch"

	if project_dir.is_empty():
		printerr("Error: --project-dir not specified")
		get_tree().quit(1)
		return

	# Initialize coordinator
	coordinator = LLMCoordinator.new()
	coordinator.set_project_dir(project_dir)

	# Initialize QuickJS runtime
	runtime = QuickJSRuntime.new()
	runtime.initialize()

	if not js_file.is_empty():
		_execute_and_screenshot()
	elif mode == "watch":
		# Poll for command file every 0.5s
		var timer = Timer.new()
		timer.wait_time = 0.5
		timer.timeout.connect(_poll_command_file)
		add_child(timer)
		timer.start()
		print('{"type":"ready","project_dir":"' + project_dir + '"}')

func _execute_and_screenshot():
	# Load and execute JS
	var fa = FileAccess.open(js_file, FileAccess.READ)
	if fa == null:
		printerr("Error: Cannot open JS file: " + js_file)
		get_tree().quit(1)
		return

	var js_code = fa.get_as_text()
	fa.close()

	var result = runtime.eval_string(js_code, js_file.get_file())
	print('{"type":"execute_result","result":"' + str(result).replace('"', '\\"') + '"}')

	# Wait 2 frames for scene to render, then screenshot
	await get_tree().process_frame
	await get_tree().process_frame

	var screenshot_path = coordinator.take_screenshot()
	print('{"type":"screenshot","path":"' + screenshot_path + '"}')

	if mode == "run":
		# Write a result file that Tauri can read
		var result_file = project_dir + "/last_run.json"
		var out = FileAccess.open(result_file, FileAccess.WRITE)
		if out:
			var result_data = {
				"success": true,
				"screenshot": screenshot_path,
				"js_file": js_file
			}
			out.store_string(JSON.stringify(result_data))
			out.close()

		runtime.finalize()
		get_tree().quit(0)

func _poll_command_file():
	var cmd_file = project_dir + "/command.json"
	if not FileAccess.file_exists(cmd_file):
		return

	var fa = FileAccess.open(cmd_file, FileAccess.READ)
	if fa == null:
		return

	var content = fa.get_as_text()
	fa.close()

	# Delete command file after reading
	DirAccess.remove_absolute(cmd_file)

	var json = JSON.new()
	var err = json.parse(content)
	if err != OK:
		printerr("Invalid command JSON: " + content)
		return

	var cmd = json.get_data()
	_handle_command(cmd)

func _handle_command(cmd: Dictionary):
	var action = cmd.get("action", "")

	match action:
		"compile_and_run":
			# Compile
			var compile_result = coordinator.compile_and_check()
			var cjson = JSON.new()
			cjson.parse(compile_result)
			var cdata = cjson.get_data()

			if not cdata.get("compile_success", false):
				_write_result({"type": "compile_and_run", "success": false, "compile": cdata})
				return

			# Find and execute JS
			var output_dir = cdata.get("output_dir", project_dir + "/build")
			var main_js = output_dir + "/main.js"
			var fa = FileAccess.open(main_js, FileAccess.READ)
			if fa == null:
				_write_result({"type": "compile_and_run", "success": false, "error": "Cannot open " + main_js})
				return

			var js_code = fa.get_as_text()
			fa.close()
			runtime.eval_string(js_code, "main.js")

			await get_tree().process_frame
			await get_tree().process_frame

			var screenshot_path = coordinator.take_screenshot()
			var state = coordinator.get_scene_state()
			var sjson = JSON.new()
			sjson.parse(state)

			_write_result({
				"type": "compile_and_run",
				"success": true,
				"screenshot": screenshot_path,
				"scene_state": sjson.get_data()
			})

		"screenshot":
			var path = coordinator.take_screenshot()
			_write_result({"type": "screenshot", "path": path})

		"quit":
			runtime.finalize()
			get_tree().quit(0)

func _write_result(data: Dictionary):
	var result_file = project_dir + "/result.json"
	var fa = FileAccess.open(result_file, FileAccess.WRITE)
	if fa:
		fa.store_string(JSON.stringify(data))
		fa.close()
	print(JSON.stringify(data))
