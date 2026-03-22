#ifndef TS_COMPILER_H
#define TS_COMPILER_H

#include "core/object/ref_counted.h"
#include "core/string/ustring.h"

class TSCompiler : public RefCounted {
	GDCLASS(TSCompiler, RefCounted);

public:
	struct CompileError {
		String file;
		int line = 0;
		int column = 0;
		String message;
		String suggestion;
	};

	struct CompileResult {
		bool success = false;
		String output_dir;
		Vector<CompileError> errors;
	};

private:
	String project_dir;
	String tsc_path;
	String eslint_path;

	CompileResult parse_tsc_output(const String &output);
	Vector<CompileError> parse_eslint_output(const String &output);
	String run_command(const String &command, int *exit_code = nullptr);

protected:
	static void _bind_methods();

public:
	TSCompiler();

	void set_project_dir(const String &p_dir);
	String get_project_dir() const { return project_dir; }

	void set_tsc_path(const String &p_path);
	void set_eslint_path(const String &p_path);

	CompileResult compile();
	Vector<CompileError> lint();
	String get_errors_json(const CompileResult &result);
};

#endif // TS_COMPILER_H
