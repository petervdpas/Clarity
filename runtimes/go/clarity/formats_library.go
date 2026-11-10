//go:build clarityformats

package clarity

// Clarity.Formats through cgo, for the runtime's own test of the data
// formats (formats_test.go): built only with the tag clarityformats, so the
// runtime stays pure Go. A program has its own copy in its formats.go.

/*
#cgo LDFLAGS: -lclarity_formats
#include <stdlib.h>
#include "clarity_formats.h"

static int32_t format_read(const char* f, size_t fn, const char* i, size_t in, const char* o, size_t on, const char* k, size_t kn, uint8_t** out, size_t* len) { return clarity_formats_api()->read(f, fn, (const uint8_t*)i, in, o, on, k, kn, out, len); }
static int32_t format_write(const char* f, size_t fn, const char* v, size_t vn, const char* o, size_t on, const char* k, size_t kn, uint8_t** out, size_t* len) { return clarity_formats_api()->write(f, fn, v, vn, o, on, k, kn, out, len); }
static void format_free(uint8_t* bytes, size_t len) { clarity_formats_api()->free(bytes, len); }
static int32_t format_problem(char* into, size_t size) { return clarity_formats_api()->problem(into, size); }
*/
import "C"

import "unsafe"

func libraryCall(read bool, format, input, options, kind string) (string, error) {
	texts := []string{format, input, options, kind}
	cs := make([]*C.char, len(texts))
	for n, t := range texts {
		cs[n] = C.CString(t)
		defer C.free(unsafe.Pointer(cs[n]))
	}
	var out *C.uint8_t
	var length C.size_t
	var code C.int32_t
	if read {
		code = C.format_read(cs[0], C.size_t(len(format)), cs[1], C.size_t(len(input)), cs[2], C.size_t(len(options)), cs[3], C.size_t(len(kind)), &out, &length)
	} else {
		code = C.format_write(cs[0], C.size_t(len(format)), cs[1], C.size_t(len(input)), cs[2], C.size_t(len(options)), cs[3], C.size_t(len(kind)), &out, &length)
	}
	if code != C.CLARITY_FORMATS_OK {
		var why [1024]C.char
		C.format_problem(&why[0], C.size_t(len(why)))
		return "", FormatProblem(C.GoString(&why[0]))
	}
	made := C.GoStringN((*C.char)(unsafe.Pointer(out)), C.int(length))
	C.format_free(out, length)
	return made, nil
}

func libraryRead(format, input, options, kind string) (string, error) {
	return libraryCall(true, format, input, options, kind)
}

func libraryWrite(format, value, options, kind string) (string, error) {
	return libraryCall(false, format, value, options, kind)
}
