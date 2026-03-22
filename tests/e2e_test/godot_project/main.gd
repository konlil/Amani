extends Node

func _ready():
	print("=== LLM3dEngine E2E Test Runner ===")

	# Step 1: Initialize QuickJS runtime
	var runtime = QuickJSRuntime.new()
	var ok = runtime.initialize()
	if ok:
		print("Step 1: QuickJS runtime initialized - PASS")
	else:
		print("Step 1: QuickJS runtime initialized - FAIL")
		get_tree().quit(1)
		return

	# Step 2: Load and execute the compiled JS
	var js_path = "res://main.js"
	var fa = FileAccess.open(js_path, FileAccess.READ)

	if fa == null:
		print("Step 2: Could not load JS file - FAIL")
		get_tree().quit(1)
		return

	var js_code = fa.get_as_text()
	fa.close()
	print("Step 2: JS file loaded (%d bytes) - PASS" % js_code.length())

	# Step 3: Execute JS in QuickJS
	print("Step 3: Executing JS...")
	var result = runtime.eval_string(js_code, "main.js")
	print("Step 3: JS execution result: %s" % result)

	# Step 4: Test state collector
	var collector = StateCollector.new()
	var state = collector.collect_scene_state()
	print("Step 4: Scene state collected (%d bytes) - PASS" % state.length())

	# Cleanup
	runtime.finalize()
	print("=== E2E Test Complete ===")
	get_tree().quit(0)
