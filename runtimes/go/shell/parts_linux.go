//go:build linux && cgo

package shell

/*
#cgo pkg-config: gtk+-3.0
#cgo LDFLAGS: -ldl
#include <stdlib.h>
#include "parts_linux.h"
*/
import "C"

import (
	"errors"
	"unsafe"
)

// told is where a part's tells go; set by startParts.
var told func(window uint64, part, tell, values string)

//export clarityGoTold
func clarityGoTold(window C.uint64_t, part, tell, values *C.char) {
	if told != nil {
		told(uint64(window), C.GoString(part), C.GoString(tell), C.GoString(values))
	}
}

func startParts(path string, on func(window uint64, part, tell, values string)) error {
	told = on
	at := C.CString(path)
	defer C.free(unsafe.Pointer(at))
	if problem := C.clarity_go_parts_start(at); problem != nil {
		return errors.New(C.GoString(problem))
	}
	return nil
}

func placePart(window uint64, part, kind string, parent uintptr, x, y, width, height int, shown bool) {
	p, k := C.CString(part), C.CString(kind)
	defer C.free(unsafe.Pointer(p))
	defer C.free(unsafe.Pointer(k))
	C.clarity_go_parts_place(C.uint64_t(window), p, k, C.uintptr_t(parent), C.int(x), C.int(y), C.int(width), C.int(height), C.bool(shown))
}

func setPart(window uint64, part, name, value string) {
	p, n, v := C.CString(part), C.CString(name), C.CString(value)
	defer C.free(unsafe.Pointer(p))
	defer C.free(unsafe.Pointer(n))
	defer C.free(unsafe.Pointer(v))
	C.clarity_go_parts_set(C.uint64_t(window), p, n, v)
}

func hearPart(window uint64, part, hear, values string) {
	p, h, v := C.CString(part), C.CString(hear), C.CString(values)
	defer C.free(unsafe.Pointer(p))
	defer C.free(unsafe.Pointer(h))
	defer C.free(unsafe.Pointer(v))
	C.clarity_go_parts_hear(C.uint64_t(window), p, h, v)
}

func closeParts(window uint64) { C.clarity_go_parts_close(C.uint64_t(window)) }

func stopParts() { C.clarity_go_parts_stop() }

// partParent is the X11 window of the webview in a desktop window (its GtkWindow).
func partParent(native unsafe.Pointer) uintptr {
	return uintptr(C.clarity_go_parts_parent(native))
}
