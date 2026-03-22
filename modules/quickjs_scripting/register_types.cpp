#include "register_types.h"

#include "core/object/class_db.h"
#include "quickjs_runtime.h"

void initialize_quickjs_scripting_module(ModuleInitializationLevel p_level) {
	if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
		return;
	}
	ClassDB::register_class<QuickJSRuntime>();
}

void uninitialize_quickjs_scripting_module(ModuleInitializationLevel p_level) {
	if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
		return;
	}
}
