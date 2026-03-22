#include "quickjs_runtime.h"
#include "quickjs_binding_gen.h"

#include "core/io/logger.h"
#include "core/object/class_db.h"

// Helper: JS string to Godot String (used by runtime helpers)
static String qjs_to_string(JSContext *ctx, JSValueConst val) {
	const char *str = JS_ToCString(ctx, val);
	String result = str ? String::utf8(str) : String();
	JS_FreeCString(ctx, str);
	return result;
}

// Helper: Godot String to JS string
static JSValue qjs_from_string(JSContext *ctx, const String &str) {
	CharString utf8 = str.utf8();
	return JS_NewStringLen(ctx, utf8.get_data(), utf8.length());
}

// Helper: convert a JS value to a Variant (best-effort)
static Variant js_to_variant(JSContext *ctx, JSValueConst val) {
	if (JS_IsBool(val)) {
		return Variant((bool)JS_ToBool(ctx, val));
	}
	if (JS_IsNumber(val)) {
		double d;
		JS_ToFloat64(ctx, &d, val);
		return Variant(d);
	}
	if (JS_IsString(val)) {
		return Variant(qjs_to_string(ctx, val));
	}
	if (JS_IsNull(val) || JS_IsUndefined(val)) {
		return Variant();
	}
	// Check if it's a wrapped Godot object
	void *opaque = JS_GetOpaque(val, get_godot_obj_class_id());
	if (opaque) {
		return Variant(static_cast<Object *>(opaque));
	}
	// Check if it's a JS object that represents a Godot value type
	if (JS_IsObject(val)) {
		// Try Vector3: {x, y, z}
		JSValue jx = JS_GetPropertyStr(ctx, val, "x");
		JSValue jy = JS_GetPropertyStr(ctx, val, "y");
		JSValue jz = JS_GetPropertyStr(ctx, val, "z");
		JSValue jw = JS_GetPropertyStr(ctx, val, "w");

		if (!JS_IsUndefined(jx) && !JS_IsUndefined(jy) && !JS_IsUndefined(jz)) {
			double x, y, z;
			JS_ToFloat64(ctx, &x, jx);
			JS_ToFloat64(ctx, &y, jy);
			JS_ToFloat64(ctx, &z, jz);
			JS_FreeValue(ctx, jx);
			JS_FreeValue(ctx, jy);
			JS_FreeValue(ctx, jz);

			if (!JS_IsUndefined(jw)) {
				// Vector4 or Quaternion: {x, y, z, w}
				double w;
				JS_ToFloat64(ctx, &w, jw);
				JS_FreeValue(ctx, jw);
				return Variant(Quaternion(x, y, z, w));
			}
			JS_FreeValue(ctx, jw);
			return Variant(Vector3(x, y, z));
		}
		JS_FreeValue(ctx, jx);
		JS_FreeValue(ctx, jy);
		JS_FreeValue(ctx, jz);
		JS_FreeValue(ctx, jw);

		// Try Color: {r, g, b, a}
		JSValue jr = JS_GetPropertyStr(ctx, val, "r");
		JSValue jg = JS_GetPropertyStr(ctx, val, "g");
		JSValue jb = JS_GetPropertyStr(ctx, val, "b");
		if (!JS_IsUndefined(jr) && !JS_IsUndefined(jg) && !JS_IsUndefined(jb)) {
			double r, g, b, a = 1.0;
			JS_ToFloat64(ctx, &r, jr);
			JS_ToFloat64(ctx, &g, jg);
			JS_ToFloat64(ctx, &b, jb);
			JSValue ja = JS_GetPropertyStr(ctx, val, "a");
			if (!JS_IsUndefined(ja)) {
				JS_ToFloat64(ctx, &a, ja);
			}
			JS_FreeValue(ctx, jr);
			JS_FreeValue(ctx, jg);
			JS_FreeValue(ctx, jb);
			JS_FreeValue(ctx, ja);
			return Variant(Color(r, g, b, a));
		}
		JS_FreeValue(ctx, jr);
		JS_FreeValue(ctx, jg);
		JS_FreeValue(ctx, jb);

		// Try Vector2: {x, y} (re-check without z)
		jx = JS_GetPropertyStr(ctx, val, "x");
		jy = JS_GetPropertyStr(ctx, val, "y");
		if (!JS_IsUndefined(jx) && !JS_IsUndefined(jy)) {
			double x, y;
			JS_ToFloat64(ctx, &x, jx);
			JS_ToFloat64(ctx, &y, jy);
			JS_FreeValue(ctx, jx);
			JS_FreeValue(ctx, jy);
			return Variant(Vector2(x, y));
		}
		JS_FreeValue(ctx, jx);
		JS_FreeValue(ctx, jy);
	}
	return Variant();
}

// Helper: convert a Variant to a JS value (best-effort)
static JSValue variant_to_js(JSContext *ctx, const Variant &v) {
	switch (v.get_type()) {
		case Variant::NIL:
			return JS_NULL;
		case Variant::BOOL:
			return JS_NewBool(ctx, (bool)v);
		case Variant::INT:
			return JS_NewInt64(ctx, (int64_t)v);
		case Variant::FLOAT:
			return JS_NewFloat64(ctx, (double)v);
		case Variant::STRING:
		case Variant::STRING_NAME:
		case Variant::NODE_PATH:
			return qjs_from_string(ctx, String(v));
		case Variant::VECTOR2:
		case Variant::VECTOR2I: {
			Vector2 vec = v;
			JSValue obj = JS_NewObject(ctx);
			JS_SetPropertyStr(ctx, obj, "x", JS_NewFloat64(ctx, vec.x));
			JS_SetPropertyStr(ctx, obj, "y", JS_NewFloat64(ctx, vec.y));
			return obj;
		}
		case Variant::VECTOR3:
		case Variant::VECTOR3I: {
			Vector3 vec = v;
			JSValue obj = JS_NewObject(ctx);
			JS_SetPropertyStr(ctx, obj, "x", JS_NewFloat64(ctx, vec.x));
			JS_SetPropertyStr(ctx, obj, "y", JS_NewFloat64(ctx, vec.y));
			JS_SetPropertyStr(ctx, obj, "z", JS_NewFloat64(ctx, vec.z));
			return obj;
		}
		case Variant::COLOR: {
			Color c = v;
			JSValue obj = JS_NewObject(ctx);
			JS_SetPropertyStr(ctx, obj, "r", JS_NewFloat64(ctx, c.r));
			JS_SetPropertyStr(ctx, obj, "g", JS_NewFloat64(ctx, c.g));
			JS_SetPropertyStr(ctx, obj, "b", JS_NewFloat64(ctx, c.b));
			JS_SetPropertyStr(ctx, obj, "a", JS_NewFloat64(ctx, c.a));
			return obj;
		}
		case Variant::QUATERNION: {
			Quaternion q = v;
			JSValue obj = JS_NewObject(ctx);
			JS_SetPropertyStr(ctx, obj, "x", JS_NewFloat64(ctx, q.x));
			JS_SetPropertyStr(ctx, obj, "y", JS_NewFloat64(ctx, q.y));
			JS_SetPropertyStr(ctx, obj, "z", JS_NewFloat64(ctx, q.z));
			JS_SetPropertyStr(ctx, obj, "w", JS_NewFloat64(ctx, q.w));
			return obj;
		}
		case Variant::OBJECT: {
			Object *obj = v.operator Object *();
			return wrap_godot_object(ctx, obj);
		}
		default:
			// For unsupported types, return string representation
			return qjs_from_string(ctx, v.stringify());
	}
}

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

// Engine.createResource(className) — create any Godot resource by class name
// e.g. Engine.createResource("BoxMesh"), Engine.createResource("StandardMaterial3D")
static JSValue js_engine_create_resource(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv) {
	if (argc < 1) {
		return JS_ThrowTypeError(ctx, "Engine.createResource requires a class name");
	}
	String class_name = qjs_to_string(ctx, argv[0]);
	Object *obj = ClassDB::instantiate(class_name);
	if (!obj) {
		return JS_ThrowReferenceError(ctx, "Cannot instantiate class: %s", class_name.utf8().get_data());
	}
	return wrap_godot_object(ctx, obj);
}

// Engine.setProperty(obj, propName, value) — set any property via Variant
static JSValue js_engine_set_property(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv) {
	if (argc < 3) {
		return JS_ThrowTypeError(ctx, "Engine.setProperty requires (obj, name, value)");
	}
	Object *obj = static_cast<Object *>(JS_GetOpaque(argv[0], get_godot_obj_class_id()));
	if (!obj) {
		return JS_ThrowTypeError(ctx, "First argument is not a Godot object");
	}
	String prop_name = qjs_to_string(ctx, argv[1]);
	Variant value = js_to_variant(ctx, argv[2]);
	obj->set(prop_name, value);
	return JS_UNDEFINED;
}

// Engine.getProperty(obj, propName) — get any property via Variant
static JSValue js_engine_get_property(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv) {
	if (argc < 2) {
		return JS_ThrowTypeError(ctx, "Engine.getProperty requires (obj, name)");
	}
	Object *obj = static_cast<Object *>(JS_GetOpaque(argv[0], get_godot_obj_class_id()));
	if (!obj) {
		return JS_ThrowTypeError(ctx, "First argument is not a Godot object");
	}
	String prop_name = qjs_to_string(ctx, argv[1]);
	Variant value = obj->get(prop_name);
	return variant_to_js(ctx, value);
}

// Engine.callMethod(obj, methodName, ...args) — call any method via Variant
static JSValue js_engine_call_method(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv) {
	if (argc < 2) {
		return JS_ThrowTypeError(ctx, "Engine.callMethod requires (obj, methodName, ...args)");
	}
	Object *obj = static_cast<Object *>(JS_GetOpaque(argv[0], get_godot_obj_class_id()));
	if (!obj) {
		return JS_ThrowTypeError(ctx, "First argument is not a Godot object");
	}
	String method_name = qjs_to_string(ctx, argv[1]);

	int method_argc = argc - 2;
	Variant *vargs = method_argc > 0 ? memnew_arr(Variant, method_argc) : nullptr;
	const Variant **argptrs = method_argc > 0 ? memnew_arr(const Variant *, method_argc) : nullptr;
	for (int i = 0; i < method_argc; i++) {
		vargs[i] = js_to_variant(ctx, argv[i + 2]);
		argptrs[i] = &vargs[i];
	}

	Callable::CallError ce;
	Variant ret = obj->callp(method_name, argptrs, method_argc, ce);

	if (argptrs) {
		memdelete_arr(argptrs);
	}
	if (vargs) {
		memdelete_arr(vargs);
	}

	if (ce.error != Callable::CallError::CALL_OK) {
		return JS_ThrowTypeError(ctx, "Method call failed: %s", method_name.utf8().get_data());
	}
	return variant_to_js(ctx, ret);
}

// Engine.onProcess(callback) — stores a JS callback for per-frame ticking
static JSValue js_engine_on_process(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv) {
	if (argc < 1 || !JS_IsFunction(ctx, argv[0])) {
		return JS_ThrowTypeError(ctx, "Engine.onProcess requires a function argument");
	}
	// Store callback as a property on the Engine object itself
	JSValue global = JS_GetGlobalObject(ctx);
	JSValue engine = JS_GetPropertyStr(ctx, global, "Engine");
	JS_SetPropertyStr(ctx, engine, "_processCallback", JS_DupValue(ctx, argv[0]));
	JS_FreeValue(ctx, engine);
	JS_FreeValue(ctx, global);
	return JS_UNDEFINED;
}

// Engine.getSceneRoot() — returns the wrapped scene root node
static JSValue js_engine_get_scene_root(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv) {
	JSValue global = JS_GetGlobalObject(ctx);
	JSValue root = JS_GetPropertyStr(ctx, global, "_sceneRoot");
	JS_FreeValue(ctx, global);
	return root;
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

	// Register generated class bindings + Engine namespace
	quickjs_register_generated_bindings(ctx, global);

	// Add Engine helper functions to the Engine namespace
	JSValue engine_ns = JS_GetPropertyStr(ctx, global, "Engine");
	JS_SetPropertyStr(ctx, engine_ns, "onProcess",
			JS_NewCFunction(ctx, js_engine_on_process, "onProcess", 1));
	JS_SetPropertyStr(ctx, engine_ns, "getSceneRoot",
			JS_NewCFunction(ctx, js_engine_get_scene_root, "getSceneRoot", 0));
	JS_SetPropertyStr(ctx, engine_ns, "createResource",
			JS_NewCFunction(ctx, js_engine_create_resource, "createResource", 1));
	JS_SetPropertyStr(ctx, engine_ns, "setProperty",
			JS_NewCFunction(ctx, js_engine_set_property, "setProperty", 3));
	JS_SetPropertyStr(ctx, engine_ns, "getProperty",
			JS_NewCFunction(ctx, js_engine_get_property, "getProperty", 2));
	JS_SetPropertyStr(ctx, engine_ns, "callMethod",
			JS_NewCFunction(ctx, js_engine_call_method, "callMethod", 3));
	JS_FreeValue(ctx, engine_ns);

	JS_FreeValue(ctx, global);

	initialized = true;
	print_line("QuickJS runtime initialized.");
	return true;
}

void QuickJSRuntime::finalize() {
	if (!initialized) {
		return;
	}

	scene_root = nullptr;

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

void QuickJSRuntime::set_scene_root(Node *p_root) {
	scene_root = p_root;
	if (!ctx || !p_root) {
		return;
	}

	// Store wrapped scene root as global._sceneRoot
	JSValue global = JS_GetGlobalObject(ctx);
	JSValue wrapped = wrap_godot_object(ctx, p_root);
	JS_SetPropertyStr(ctx, global, "_sceneRoot", wrapped);
	JS_FreeValue(ctx, global);

	print_line("QuickJS: Scene root set.");
}

void QuickJSRuntime::tick_process(float p_delta) {
	if (!ctx) {
		return;
	}

	JSValue global = JS_GetGlobalObject(ctx);
	JSValue engine = JS_GetPropertyStr(ctx, global, "Engine");
	JSValue callback = JS_GetPropertyStr(ctx, engine, "_processCallback");

	if (JS_IsFunction(ctx, callback)) {
		JSValue delta_val = JS_NewFloat64(ctx, p_delta);
		JSValue ret = JS_Call(ctx, callback, JS_UNDEFINED, 1, &delta_val);
		if (JS_IsException(ret)) {
			JSValue exception = JS_GetException(ctx);
			const char *str = JS_ToCString(ctx, exception);
			if (str) {
				ERR_PRINT("QuickJS _process error: " + String::utf8(str));
				JS_FreeCString(ctx, str);
			}
			JS_FreeValue(ctx, exception);
		}
		JS_FreeValue(ctx, ret);
		JS_FreeValue(ctx, delta_val);
	}

	JS_FreeValue(ctx, callback);
	JS_FreeValue(ctx, engine);
	JS_FreeValue(ctx, global);
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
	ClassDB::bind_method(D_METHOD("set_scene_root", "root"), &QuickJSRuntime::set_scene_root);
	ClassDB::bind_method(D_METHOD("tick_process", "delta"), &QuickJSRuntime::tick_process);
}
