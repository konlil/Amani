#ifndef SCREENSHOT_H
#define SCREENSHOT_H

#include "core/object/ref_counted.h"
#include "core/string/ustring.h"

class Screenshot : public RefCounted {
	GDCLASS(Screenshot, RefCounted);

protected:
	static void _bind_methods();

public:
	String capture(const String &output_path, int width = 1280, int height = 720);
};

#endif // SCREENSHOT_H
