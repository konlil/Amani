#ifndef COORDINATOR_H
#define COORDINATOR_H

#include "core/object/ref_counted.h"
#include "core/string/ustring.h"
#include "screenshot.h"
#include "state_collector.h"
#include "ts_compiler.h"

class LLMCoordinator : public RefCounted {
	GDCLASS(LLMCoordinator, RefCounted);

	Ref<TSCompiler> compiler;
	Ref<StateCollector> state_collector;
	Ref<Screenshot> screenshot;

	String project_dir;
	int screenshot_counter = 0;

protected:
	static void _bind_methods();

public:
	LLMCoordinator();

	void set_project_dir(const String &p_dir);
	String get_project_dir() const { return project_dir; }

	// Full compile-check pipeline: tsc -> eslint -> return structured feedback
	String compile_and_check();

	// Collect current scene state as JSON
	String get_scene_state();

	// Take a screenshot, return file path
	String take_screenshot(int width = 1280, int height = 720);

	// Full feedback bundle: compile errors + scene state + screenshot path
	String get_feedback();

	Ref<TSCompiler> get_compiler() { return compiler; }
	Ref<StateCollector> get_state_collector() { return state_collector; }
	Ref<Screenshot> get_screenshot_tool() { return screenshot; }
};

#endif // COORDINATOR_H
