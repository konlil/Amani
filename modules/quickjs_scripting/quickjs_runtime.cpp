#include "quickjs_runtime.h"

#include "core/io/logger.h"

static JSValue js_print(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv) {
	String output;
	for (int i = 0; i < argc; i++) {
		if (i > 0) {
			output += " ";
		}
		const char *str = JS_ToCString(ctx, argv[i]);
		if (str) {
			output += String::utf8(str);
			JS_FreeCString(ctx, str);
		}
	}
	print_line("[QuickJS] " + output);
	return JS_UNDEFINED;
}

QuickJSRuntime::QuickJSRuntime() {
}

QuickJSRuntime::~QuickJSRuntime() {
	finalize();
}

bool QuickJSRuntime::initialize() {
	if (initialized) {
		return true;
	}

	rt = JS_NewRuntime();
	if (!rt) {
		ERR_PRINT("QuickJS: Failed to create runtime");
		return false;
	}

	ctx = JS_NewContext(rt);
	if (!ctx) {
		ERR_PRINT("QuickJS: Failed to create context");
		JS_FreeRuntime(rt);
		rt = nullptr;
		return false;
	}

	// Register console.log
	JSValue global = JS_GetGlobalObject(ctx);

	JSValue console = JS_NewObject(ctx);
	JS_SetPropertyStr(ctx, console, "log",
			JS_NewCFunction(ctx, js_print, "log", 1));
	JS_SetPropertyStr(ctx, global, "console", console);

	// Also register top-level print
	JS_SetPropertyStr(ctx, global, "print",
			JS_NewCFunction(ctx, js_print, "print", 1));

	JS_FreeValue(ctx, global);

	initialized = true;
	print_line("QuickJS runtime initialized.");
	return true;
}

void QuickJSRuntime::finalize() {
	if (!initialized) {
		return;
	}

	if (ctx) {
		JS_FreeContext(ctx);
		ctx = nullptr;
	}
	if (rt) {
		JS_FreeRuntime(rt);
		rt = nullptr;
	}

	initialized = false;
	print_line("QuickJS runtime finalized.");
}

String QuickJSRuntime::eval_string(const String &p_code, const String &p_filename) {
	if (!initialized) {
		ERR_PRINT("QuickJS: Runtime not initialized");
		return "error: runtime not initialized";
	}

	CharString code_utf8 = p_code.utf8();
	CharString filename_utf8 = p_filename.utf8();

	JSValue result = JS_Eval(ctx, code_utf8.get_data(), code_utf8.length(),
			filename_utf8.get_data(), JS_EVAL_TYPE_GLOBAL);

	String output;
	if (JS_IsException(result)) {
		JSValue exception = JS_GetException(ctx);
		const char *str = JS_ToCString(ctx, exception);
		if (str) {
			output = "error: " + String::utf8(str);
			ERR_PRINT("QuickJS exception: " + String::utf8(str));
			JS_FreeCString(ctx, str);
		}

		// Print stack trace if available
		JSValue stack = JS_GetPropertyStr(ctx, exception, "stack");
		if (!JS_IsUndefined(stack)) {
			const char *stack_str = JS_ToCString(ctx, stack);
			if (stack_str) {
				ERR_PRINT("Stack: " + String::utf8(stack_str));
				JS_FreeCString(ctx, stack_str);
			}
		}
		JS_FreeValue(ctx, stack);
		JS_FreeValue(ctx, exception);
	} else {
		const char *str = JS_ToCString(ctx, result);
		if (str) {
			output = String::utf8(str);
			JS_FreeCString(ctx, str);
		}
	}

	JS_FreeValue(ctx, result);
	return output;
}

void QuickJSRuntime::_bind_methods() {
	ClassDB::bind_method(D_METHOD("initialize"), &QuickJSRuntime::initialize);
	ClassDB::bind_method(D_METHOD("finalize"), &QuickJSRuntime::finalize);
	ClassDB::bind_method(D_METHOD("eval_string", "code", "filename"), &QuickJSRuntime::eval_string, DEFVAL("<eval>"));
	ClassDB::bind_method(D_METHOD("is_initialized"), &QuickJSRuntime::is_initialized);
}
