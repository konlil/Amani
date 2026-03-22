#include "screenshot.h"

#include "core/io/image.h"
#include "scene/main/scene_tree.h"
#include "scene/main/viewport.h"
#include "scene/main/window.h"
#include "servers/rendering_server.h"

String Screenshot::capture(const String &output_path, int width, int height) {
	SceneTree *tree = SceneTree::get_singleton();
	if (!tree) {
		return "error: no scene tree";
	}

	Viewport *viewport = tree->get_root();
	if (!viewport) {
		return "error: no root viewport";
	}

	Ref<Image> image = viewport->get_texture()->get_image();
	if (image.is_null()) {
		return "error: failed to get viewport image";
	}

	// Resize if needed
	if (image->get_width() != width || image->get_height() != height) {
		image->resize(width, height, Image::INTERPOLATE_BILINEAR);
	}

	// Save as PNG
	Error err = image->save_png(output_path);
	if (err != OK) {
		return "error: failed to save screenshot to " + output_path;
	}

	return output_path;
}

void Screenshot::_bind_methods() {
	ClassDB::bind_method(D_METHOD("capture", "output_path", "width", "height"), &Screenshot::capture, DEFVAL(1280), DEFVAL(720));
}
