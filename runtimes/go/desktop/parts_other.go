//go:build (!linux || !cgo) && !windows

package desktop

import "unsafe"

func partParent(unsafe.Pointer) uintptr { return 0 }
