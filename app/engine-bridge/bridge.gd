extends Node

var coordinator: LLMCoordinator
var runtime: QuickJSRuntime
var project_dir: String = ""
var js_file: String = ""
var mode: String = "run"
var ipc_host: String = "127.0.0.1"
var ipc_port: int = 0
var session_id: String = ""
var ipc_peer: StreamPeerTCP
var ipc_buffer: String = ""
var ipc_connected: bool = false
var ready_sent: bool = false
var ipc_retry_count: int = 0
const IPC_MAX_RETRIES := 5
const IPC_RETRY_INTERVAL_SEC := 0.5
var ipc_retry_deadline_msec: int = 0
var overlay_hidden: bool = false
var overlay_x: int = -10000
var overlay_y: int = -10000
var overlay_w: int = 1280
var overlay_h: int = 720
var log_path: String = ""

func _log(message: String):
	if log_path.is_empty():
		return
	var file = FileAccess.open(log_path, FileAccess.READ_WRITE)
	if file == null:
		file = FileAccess.open(log_path, FileAccess.WRITE)
	if file == null:
		return
	file.seek_end()
	file.store_line(message)
	file.close()

func _ready():
	var args = OS.get_cmdline_user_args()
	for i in range(args.size()):
		if args[i] == "--project-dir" and i + 1 < args.size():
			project_dir = args[i + 1]
		elif args[i] == "--run" and i + 1 < args.size():
			js_file = args[i + 1]
		elif args[i] == "--watch":
			mode = "watch"
		elif args[i] == "--ipc-host" and i + 1 < args.size():
			ipc_host = args[i + 1]
		elif args[i] == "--ipc-port" and i + 1 < args.size():
			ipc_port = int(args[i + 1])
		elif args[i] == "--session-id" and i + 1 < args.size():
			session_id = args[i + 1]

	if project_dir.is_empty():
		printerr("Error: --project-dir not specified")
		get_tree().quit(1)
		return

	log_path = project_dir.path_join("overlay_bridge.log")
	var clear = FileAccess.open(log_path, FileAccess.WRITE)
	if clear:
		clear.store_line("bridge log start")
		clear.close()
	_log("ready project_dir=%s mode=%s ipc=%s:%d session=%s" % [project_dir, mode, ipc_host, ipc_port, session_id])

	coordinator = LLMCoordinator.new()
	coordinator.set_project_dir(project_dir)

	runtime = QuickJSRuntime.new()
	runtime.initialize()
	runtime.set_scene_root(get_tree().root)

	if not js_file.is_empty():
		_execute_and_screenshot()
	elif mode == "watch":
		DisplayServer.window_set_flag(DisplayServer.WINDOW_FLAG_BORDERLESS, true)
		_connect_ipc()

func _process(delta):
	if runtime and runtime.is_initialized():
		runtime.tick_process(delta)

	if mode == "watch":
		_poll_ipc()

func _connect_ipc():
	if ipc_port <= 0:
		printerr("Error: --ipc-port not specified")
		get_tree().quit(1)
		return

	ipc_peer = StreamPeerTCP.new()
	var err = ipc_peer.connect_to_host(ipc_host, ipc_port)
	_log("connect_to_host err=%s host=%s port=%d" % [str(err), ipc_host, ipc_port])
	if err != OK:
		printerr("Error: cannot connect to IPC host: " + str(err))
		get_tree().quit(1)

func _poll_ipc():
	if ipc_peer == null:
		return

	ipc_peer.poll()
	var status = ipc_peer.get_status()
	_log("poll status=%d connected=%s retries=%d" % [status, str(ipc_connected), ipc_retry_count])

	if status == StreamPeerTCP.STATUS_CONNECTED:
		if not ipc_connected:
			ipc_connected = true
			ipc_retry_count = 0
			_send_ready_messages()

		var available = ipc_peer.get_available_bytes()
		if available > 0:
			_log("recv available=%d" % available)
			ipc_buffer += ipc_peer.get_utf8_string(available)
			var newline_pos = ipc_buffer.find("\n")
			while newline_pos != -1:
				var line = ipc_buffer.substr(0, newline_pos).strip_edges()
				ipc_buffer = ipc_buffer.substr(newline_pos + 1)
				if not line.is_empty():
					_handle_ipc_line(line)
				newline_pos = ipc_buffer.find("\n")
	elif status == StreamPeerTCP.STATUS_ERROR or status == StreamPeerTCP.STATUS_NONE:
		if Time.get_ticks_msec() < ipc_retry_deadline_msec:
			return
		if ipc_retry_count >= IPC_MAX_RETRIES:
			printerr("IPC connection dropped")
			get_tree().quit(1)
			return
		ipc_retry_count += 1
		ipc_retry_deadline_msec = Time.get_ticks_msec() + int(IPC_RETRY_INTERVAL_SEC * 1000.0)
		ipc_connected = false
		ready_sent = false
		_log("ipc reconnect scheduled retry=%d" % ipc_retry_count)
		_connect_ipc()

func _apply_overlay_frame():
	if overlay_hidden:
		_log("apply_overlay hidden -> offscreen")
		DisplayServer.window_set_position(Vector2i(-20000, -20000))
		return

	_log("apply_overlay frame x=%d y=%d w=%d h=%d" % [overlay_x, overlay_y, overlay_w, overlay_h])
	DisplayServer.window_set_position(Vector2i(overlay_x, overlay_y))
	DisplayServer.window_set_size(Vector2i(overlay_w, overlay_h))

func _send_ready_messages():
	if ready_sent:
		return

	ready_sent = true
	_log("send ready messages")
	_send_ipc({
		"type": "hello",
		"project_dir": project_dir
	})
	_send_ipc({
		"type": "ready",
		"project_dir": project_dir
	})

	var handle = DisplayServer.window_get_native_handle(DisplayServer.WINDOW_HANDLE)
	_send_ipc({
		"type": "window_ready",
		"handle": handle
	})

func _handle_ipc_line(line: String):
	var json = JSON.new()
	var err = json.parse(line)
	if err != OK:
		printerr("Invalid IPC JSON: " + line)
		return

	var msg = json.get_data()
	if typeof(msg) != TYPE_DICTIONARY:
		return

	if msg.get("session_id", "") != session_id:
		_log("ignore message with foreign session")
		return

	_log("handle message action=%s type=%s" % [str(msg.get("action", "")), str(msg.get("type", ""))])
	_handle_command(msg)

func _handle_command(cmd: Dictionary):
	var action = cmd.get("action", "")

	match action:
		"compile_and_run":
			await _compile_and_run(cmd)
		"quit":
			_log("command quit")
			runtime.finalize()
			get_tree().quit(0)
		"show_window":
			overlay_hidden = false
			_log("command show_window")
			_apply_overlay_frame()
		"hide_window":
			overlay_hidden = true
			_log("command hide_window")
			_apply_overlay_frame()
		"set_frame":
			overlay_x = int(cmd.get("x", 0))
			overlay_y = int(cmd.get("y", 0))
			overlay_w = int(cmd.get("width", 0))
			overlay_h = int(cmd.get("height", 0))
			_log("command set_frame x=%d y=%d w=%d h=%d" % [overlay_x, overlay_y, overlay_w, overlay_h])
			_apply_overlay_frame()

func _compile_and_run(cmd: Dictionary):
	var compile_result = coordinator.compile_and_check()
	_log("command compile_and_run")
	var cjson = JSON.new()
	cjson.parse(compile_result)
	var cdata = cjson.get_data()

	if not cdata.get("compile_success", false):
		_send_ipc({
			"type": "compile_and_run",
			"request_id": cmd.get("request_id", ""),
			"success": false,
			"compile": cdata
		})
		return

	var output_dir = cdata.get("output_dir", project_dir + "/build")
	var main_js = output_dir + "/scripts/main.js"
	if not FileAccess.file_exists(main_js):
		main_js = output_dir + "/main.js"
	var fa = FileAccess.open(main_js, FileAccess.READ)
	if fa == null:
		_send_ipc({
			"type": "compile_and_run",
			"request_id": cmd.get("request_id", ""),
			"success": false,
			"error": "Cannot open " + main_js
		})
		return

	var root = get_tree().root
	for child in root.get_children():
		if child != self:
			root.remove_child(child)
			child.queue_free()

	runtime.finalize()
	runtime.initialize()
	runtime.set_scene_root(root)

	var js_code = fa.get_as_text()
	fa.close()
	runtime.eval_string(js_code, "main.js")

	for i in range(5):
		runtime.tick_process(get_process_delta_time())
		await get_tree().process_frame

	var state = coordinator.get_scene_state()
	var sjson = JSON.new()
	sjson.parse(state)

	_send_ipc({
		"type": "compile_and_run",
		"request_id": cmd.get("request_id", ""),
		"success": true,
		"scene_state": sjson.get_data()
	})

func _send_ipc(data: Dictionary):
	if ipc_peer == null:
		return

	data["session_id"] = session_id
	var payload = JSON.stringify(data) + "\n"
	var bytes = payload.to_utf8_buffer()
	var err = ipc_peer.put_data(bytes)
	_log("send_ipc type=%s action=%s err=%s" % [str(data.get("type", "")), str(data.get("action", "")), str(err)])
	if err != OK:
		printerr("IPC send failed: " + str(err))

func _execute_and_screenshot():
	var fa = FileAccess.open(js_file, FileAccess.READ)
	if fa == null:
		printerr("Error: Cannot open JS file: " + js_file)
		get_tree().quit(1)
		return

	var js_code = fa.get_as_text()
	fa.close()

	var result = runtime.eval_string(js_code, js_file.get_file())
	print('{"type":"execute_result","result":"' + str(result).replace('"', '\\"') + '"}')

	for i in range(5):
		runtime.tick_process(get_process_delta_time())
		await get_tree().process_frame

	var screenshot_path = coordinator.take_screenshot()
	print('{"type":"screenshot","path":"' + screenshot_path + '"}')

	if mode == "run":
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
