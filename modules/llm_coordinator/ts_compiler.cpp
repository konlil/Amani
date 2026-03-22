#include "ts_compiler.h"

#include "core/io/json.h"
#include "core/os/os.h"

TSCompiler::TSCompiler() {
	tsc_path = "npx tsc";
	eslint_path = "npx eslint";
}

void TSCompiler::set_project_dir(const String &p_dir) {
	project_dir = p_dir;
}

void TSCompiler::set_tsc_path(const String &p_path) {
	tsc_path = p_path;
}

void TSCompiler::set_eslint_path(const String &p_path) {
	eslint_path = p_path;
}

String TSCompiler::run_command(const String &command, int *exit_code) {
	List<String> args;
	args.push_back("-c");
	args.push_back(command);

	String output;
	int ec = OS::get_singleton()->execute("sh", args, &output);
	if (exit_code) {
		*exit_code = ec;
	}
	return output;
}

TSCompiler::CompileResult TSCompiler::parse_tsc_output(const String &output) {
	CompileResult result;
	result.success = true;

	Vector<String> lines = output.split("\n");
	for (int i = 0; i < lines.size(); i++) {
		String line = lines[i].strip_edges();
		if (line.is_empty()) {
			continue;
		}

		// tsc error format: file(line,col): error TSxxxx: message
		int paren_open = line.find("(");
		int paren_close = line.find(")");
		int colon_pos = line.find(": error ");

		if (paren_open >= 0 && paren_close > paren_open && colon_pos > paren_close) {
			CompileError err;
			err.file = line.substr(0, paren_open);

			String loc = line.substr(paren_open + 1, paren_close - paren_open - 1);
			Vector<String> loc_parts = loc.split(",");
			if (loc_parts.size() >= 2) {
				err.line = loc_parts[0].to_int();
				err.column = loc_parts[1].to_int();
			}

			err.message = line.substr(colon_pos + 2);

			// Check for "Did you mean" suggestions
			if (i + 1 < lines.size()) {
				String next_line = lines[i + 1].strip_edges();
				if (next_line.begins_with("Did you mean")) {
					err.suggestion = next_line;
				}
			}

			result.errors.push_back(err);
			result.success = false;
		}
	}

	return result;
}

Vector<TSCompiler::CompileError> TSCompiler::parse_eslint_output(const String &output) {
	Vector<CompileError> errors;

	// Parse ESLint JSON output
	JSON json;
	Error parse_err = json.parse(output);
	if (parse_err != OK) {
		return errors;
	}

	Variant data = json.get_data();
	if (data.get_type() != Variant::ARRAY) {
		return errors;
	}

	Array files = data;
	for (int i = 0; i < files.size(); i++) {
		Dictionary file_result = files[i];
		String file_path = file_result.get("filePath", "");
		Array messages = file_result.get("messages", Array());

		for (int j = 0; j < messages.size(); j++) {
			Dictionary msg = messages[j];
			CompileError err;
			err.file = file_path;
			err.line = msg.get("line", 0);
			err.column = msg.get("column", 0);
			err.message = msg.get("message", "");
			errors.push_back(err);
		}
	}

	return errors;
}

TSCompiler::CompileResult TSCompiler::compile() {
	String cmd = tsc_path + " --project " + project_dir + "/tsconfig.json --noEmit 2>&1";
	int ec = 0;
	String output = run_command(cmd, &ec);

	CompileResult result = parse_tsc_output(output);
	if (ec == 0) {
		result.success = true;
		result.errors.clear();
	}

	// If tsc passed, also compile to JS
	if (result.success) {
		String build_cmd = tsc_path + " --project " + project_dir + "/tsconfig.json 2>&1";
		run_command(build_cmd, &ec);
		result.output_dir = project_dir + "/build";
	}

	return result;
}

Vector<TSCompiler::CompileError> TSCompiler::lint() {
	String cmd = eslint_path + " --format json " + project_dir + "/src/**/*.ts 2>&1";
	int ec = 0;
	String output = run_command(cmd, &ec);
	return parse_eslint_output(output);
}

String TSCompiler::get_errors_json(const CompileResult &result) {
	Array errors_array;
	for (int i = 0; i < result.errors.size(); i++) {
		Dictionary err;
		err["stage"] = "compile";
		err["file"] = result.errors[i].file;
		err["line"] = result.errors[i].line;
		err["column"] = result.errors[i].column;
		err["message"] = result.errors[i].message;
		if (!result.errors[i].suggestion.is_empty()) {
			err["suggestion"] = result.errors[i].suggestion;
		}
		errors_array.push_back(err);
	}

	return JSON::stringify(errors_array, "\t");
}

void TSCompiler::_bind_methods() {
	ClassDB::bind_method(D_METHOD("set_project_dir", "dir"), &TSCompiler::set_project_dir);
	ClassDB::bind_method(D_METHOD("get_project_dir"), &TSCompiler::get_project_dir);
	ClassDB::bind_method(D_METHOD("set_tsc_path", "path"), &TSCompiler::set_tsc_path);
	ClassDB::bind_method(D_METHOD("set_eslint_path", "path"), &TSCompiler::set_eslint_path);
}
