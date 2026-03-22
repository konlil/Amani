#ifndef STATE_COLLECTOR_H
#define STATE_COLLECTOR_H

#include "core/object/ref_counted.h"
#include "core/string/ustring.h"

class StateCollector : public RefCounted {
	GDCLASS(StateCollector, RefCounted);

protected:
	static void _bind_methods();

public:
	String collect_scene_state();
	String collect_entity_info(const String &entity_name);
};

#endif // STATE_COLLECTOR_H
