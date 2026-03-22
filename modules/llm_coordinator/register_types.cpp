#include "register_types.h"

#include "coordinator.h"
#include "core/object/class_db.h"
#include "screenshot.h"
#include "state_collector.h"
#include "ts_compiler.h"

void initialize_llm_coordinator_module(ModuleInitializationLevel p_level) {
	if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
		return;
	}
	ClassDB::register_class<TSCompiler>();
	ClassDB::register_class<StateCollector>();
	ClassDB::register_class<Screenshot>();
	ClassDB::register_class<LLMCoordinator>();
}

void uninitialize_llm_coordinator_module(ModuleInitializationLevel p_level) {
	if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
		return;
	}
}
