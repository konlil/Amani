#ifndef QUICKJS_RUNTIME_H
#define QUICKJS_RUNTIME_H

#include "core/object/ref_counted.h"
#include "core/string/ustring.h"
#include "scene/main/node.h"

extern "C" {
#include "quickjs.h"
}

class QuickJSRuntime : public RefCounted {
	GDCLASS(QuickJSRuntime, RefCounted);

	JSRuntime *rt = nullptr;
	JSContext *ctx = nullptr;

	bool initialized = false;
	Node *scene_root = nullptr;

	// Stored JS callback for _process(delta)
	JSValue process_callback = JS_UNDEFINED;

protected:
	static void _bind_methods();

public:
	QuickJSRuntime();
	~QuickJSRuntime();

	bool initialize();
	void finalize();

	String eval_string(const String &p_code, const String &p_filename = "<eval>");
	bool is_initialized() const { return initialized; }

	void set_scene_root(Node *p_root);
	void tick_process(float p_delta);
};

#endif // QUICKJS_RUNTIME_H
