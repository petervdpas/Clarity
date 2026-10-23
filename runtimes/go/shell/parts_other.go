//go:build !linux || !cgo

package shell

import "unsafe"

func partParent(unsafe.Pointer) uintptr { return 0 }
