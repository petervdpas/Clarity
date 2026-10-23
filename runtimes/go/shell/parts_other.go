//go:build !linux || !cgo

package shell

import (
	"errors"
	"unsafe"
)

func startParts(string, func(uint64, string, string, string)) error {
	return errors.New("custom parts drawn by a native library run on Linux for now")
}

func placePart(uint64, string, string, uintptr, int, int, int, int, bool) {}
func setPart(uint64, string, string, string)                              {}
func hearPart(uint64, string, string, string)                             {}
func closeParts(uint64)                                                   {}
func stopParts()                                                          {}
func partParent(unsafe.Pointer) uintptr                                   { return 0 }
