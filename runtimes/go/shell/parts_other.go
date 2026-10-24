//go:build (!linux || !cgo) && !windows

package shell

import "unsafe"

func partParent(unsafe.Pointer) uintptr { return 0 }
