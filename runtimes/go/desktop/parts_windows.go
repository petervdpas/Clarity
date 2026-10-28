//go:build windows

package desktop

import "unsafe"

// partParent is the window of a desktop window on Windows: its HWND, which
// the part host makes its child windows in, beside the webview's.
func partParent(native unsafe.Pointer) uintptr {
	return uintptr(native)
}
