#include "register_types.h"

#include "core/object/class_db.h"

void initialize_quickjs_scripting_module(ModuleInitializationLevel p_level) {
	if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
		return;
	}
	// TODO: Register QuickJS runtime classes here
}

void uninitialize_quickjs_scripting_module(ModuleInitializationLevel p_level) {
	if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
		return;
	}
	// TODO: Cleanup QuickJS runtime here
}
