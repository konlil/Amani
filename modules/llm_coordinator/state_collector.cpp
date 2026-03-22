#include "state_collector.h"

#include "core/io/json.h"
#include "scene/main/node.h"
#include "scene/main/scene_tree.h"
#include "scene/main/window.h"
#include "scene/3d/node_3d.h"

static Dictionary collect_node_info(Node *node) {
	Dictionary info;
	info["name"] = node->get_name();
	info["class"] = node->get_class();

	// Position for 3D nodes
	Node3D *node_3d = Object::cast_to<Node3D>(node);
	if (node_3d) {
		Vector3 pos = node_3d->get_position();
		Array position;
		position.push_back(pos.x);
		position.push_back(pos.y);
		position.push_back(pos.z);
		info["position"] = position;

		Vector3 rot = node_3d->get_rotation_degrees();
		Array rotation;
		rotation.push_back(rot.x);
		rotation.push_back(rot.y);
		rotation.push_back(rot.z);
		info["rotation_degrees"] = rotation;
	}

	// Children
	Array children;
	for (int i = 0; i < node->get_child_count(); i++) {
		children.push_back(collect_node_info(node->get_child(i)));
	}
	if (children.size() > 0) {
		info["children"] = children;
	}

	return info;
}

String StateCollector::collect_scene_state() {
	SceneTree *tree = SceneTree::get_singleton();
	if (!tree) {
		return "{\"error\": \"No scene tree\"}";
	}

	Node *root = tree->get_root();
	if (!root) {
		return "{\"error\": \"No root node\"}";
	}

	Dictionary state;
	state["scene_tree"] = collect_node_info(root);

	// Performance info
	Dictionary perf;
	perf["fps"] = Engine::get_singleton()->get_frames_per_second();
	state["performance"] = perf;

	return JSON::stringify(state, "\t");
}

String StateCollector::collect_entity_info(const String &entity_name) {
	SceneTree *tree = SceneTree::get_singleton();
	if (!tree || !tree->get_root()) {
		return "{\"error\": \"No scene tree\"}";
	}

	Node *node = tree->get_root()->find_child(entity_name, true, false);
	if (!node) {
		return "{\"error\": \"Entity not found: " + entity_name + "\"}";
	}

	Dictionary info = collect_node_info(node);
	return JSON::stringify(info, "\t");
}

void StateCollector::_bind_methods() {
	ClassDB::bind_method(D_METHOD("collect_scene_state"), &StateCollector::collect_scene_state);
	ClassDB::bind_method(D_METHOD("collect_entity_info", "entity_name"), &StateCollector::collect_entity_info);
}
