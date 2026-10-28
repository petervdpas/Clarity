//go:build linux && cgo

package desktop

/*
#cgo pkg-config: gtk+-3.0
#include "parts_linux.h"
*/
import "C"

import "unsafe"

// partParent is the X11 window of the webview in a desktop window (its GtkWindow).
func partParent(native unsafe.Pointer) uintptr {
	return uintptr(C.clarity_go_parts_parent(native))
}
