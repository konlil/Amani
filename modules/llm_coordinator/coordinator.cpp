#include "coordinator.h"

#include "core/io/json.h"

LLMCoordinator::LLMCoordinator() {
	compiler.instantiate();
	state_collector.instantiate();
	screenshot.instantiate();
}

void LLMCoordinator::set_project_dir(const String &p_dir) {
	project_dir = p_dir;
	compiler->set_project_dir(p_dir);
}

String LLMCoordinator::compile_and_check() {
	Dictionary result;

	// Step 1: tsc compile
	TSCompiler::CompileResult compile_result = compiler->compile();
	result["compile_success"] = compile_result.success;

	if (!compile_result.success) {
		result["stage"] = "compile";
		Array errors;
		for (int i = 0; i < compile_result.errors.size(); i++) {
			Dictionary err;
			err["file"] = compile_result.errors[i].file;
			err["line"] = compile_result.errors[i].line;
			err["column"] = compile_result.errors[i].column;
			err["message"] = compile_result.errors[i].message;
			if (!compile_result.errors[i].suggestion.is_empty()) {
				err["suggestion"] = compile_result.errors[i].suggestion;
			}
			errors.push_back(err);
		}
		result["errors"] = errors;
		return JSON::stringify(result, "\t");
	}

	// Step 2: ESLint check
	Vector<TSCompiler::CompileError> lint_errors = compiler->lint();
	if (lint_errors.size() > 0) {
		result["stage"] = "lint";
		result["lint_success"] = false;
		Array errors;
		for (int i = 0; i < lint_errors.size(); i++) {
			Dictionary err;
			err["file"] = lint_errors[i].file;
			err["line"] = lint_errors[i].line;
			err["column"] = lint_errors[i].column;
			err["message"] = lint_errors[i].message;
			errors.push_back(err);
		}
		result["errors"] = errors;
	} else {
		result["lint_success"] = true;
	}

	result["stage"] = "complete";
	result["output_dir"] = compile_result.output_dir;
	return JSON::stringify(result, "\t");
}

String LLMCoordinator::get_scene_state() {
	return state_collector->collect_scene_state();
}

String LLMCoordinator::take_screenshot(int width, int height) {
	screenshot_counter++;
	String path = project_dir + "/screenshots/frame_" + String::num_int64(screenshot_counter) + ".png";
	return screenshot->capture(path, width, height);
}

String LLMCoordinator::get_feedback() {
	Dictionary feedback;

	// Scene state
	String state_json = state_collector->collect_scene_state();
	JSON json;
	json.parse(state_json);
	feedback["scene_state"] = json.get_data();

	// Screenshot
	String screenshot_path = take_screenshot();
	feedback["screenshot"] = screenshot_path;

	return JSON::stringify(feedback, "\t");
}

void LLMCoordinator::_bind_methods() {
	ClassDB::bind_method(D_METHOD("set_project_dir", "dir"), &LLMCoordinator::set_project_dir);
	ClassDB::bind_method(D_METHOD("get_project_dir"), &LLMCoordinator::get_project_dir);
	ClassDB::bind_method(D_METHOD("compile_and_check"), &LLMCoordinator::compile_and_check);
	ClassDB::bind_method(D_METHOD("get_scene_state"), &LLMCoordinator::get_scene_state);
	ClassDB::bind_method(D_METHOD("take_screenshot", "width", "height"), &LLMCoordinator::take_screenshot, DEFVAL(1280), DEFVAL(720));
	ClassDB::bind_method(D_METHOD("get_feedback"), &LLMCoordinator::get_feedback);
}
