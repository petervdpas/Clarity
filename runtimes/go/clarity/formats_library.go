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
static uint32_t format_interface(void) { return clarity_formats_api()->interface; }
static int32_t format_read_table(const char* f, size_t fn, const uint8_t* i, size_t in, const char* o, size_t on, const char* k, size_t kn, uint8_t** out, size_t* len) { return clarity_formats_api()->read_table(f, fn, i, in, o, on, k, kn, out, len); }
static int32_t format_write_table(const char* f, size_t fn, const uint8_t* g, size_t gn, const char* t, size_t tn, const char* o, size_t on, const char* k, size_t kn, uint8_t** out, size_t* len) { return clarity_formats_api()->write_table(f, fn, g, gn, t, tn, o, on, k, kn, out, len); }
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
	made, err := libraryMade(code, out, length)
	return string(made), err
}

// libraryMade is what a call made, given back to the library, or why it failed.
func libraryMade(code C.int32_t, out *C.uint8_t, length C.size_t) ([]byte, error) {
	if code != C.CLARITY_FORMATS_OK {
		var why [1024]C.char
		C.format_problem(&why[0], C.size_t(len(why)))
		return nil, FormatProblem(C.GoString(&why[0]))
	}
	made := C.GoBytes(unsafe.Pointer(out), C.int(length))
	C.format_free(out, length)
	return made, nil
}

// libraryTables tells whether the library has the table calls (interface 2).
func libraryTables() error {
	if C.format_interface() < 2 {
		return FormatProblem("Clarity.Formats is too old for a file data source")
	}
	return nil
}

func libraryReadTable(format string, input []byte, options, fields string) (string, error) {
	if err := libraryTables(); err != nil {
		return "", err
	}
	f, o, k := C.CString(format), C.CString(options), C.CString(fields)
	defer C.free(unsafe.Pointer(f))
	defer C.free(unsafe.Pointer(o))
	defer C.free(unsafe.Pointer(k))
	i := C.CBytes(input)
	defer C.free(i)
	var out *C.uint8_t
	var length C.size_t
	code := C.format_read_table(f, C.size_t(len(format)), (*C.uint8_t)(i), C.size_t(len(input)), o, C.size_t(len(options)), k, C.size_t(len(fields)), &out, &length)
	made, err := libraryMade(code, out, length)
	return string(made), err
}

func libraryWriteTable(format string, original []byte, table, options, fields string) ([]byte, error) {
	if err := libraryTables(); err != nil {
		return nil, err
	}
	f, t, o, k := C.CString(format), C.CString(table), C.CString(options), C.CString(fields)
	defer C.free(unsafe.Pointer(f))
	defer C.free(unsafe.Pointer(t))
	defer C.free(unsafe.Pointer(o))
	defer C.free(unsafe.Pointer(k))
	g := C.CBytes(original)
	defer C.free(g)
	var out *C.uint8_t
	var length C.size_t
	code := C.format_write_table(f, C.size_t(len(format)), (*C.uint8_t)(g), C.size_t(len(original)), t, C.size_t(len(table)), o, C.size_t(len(options)), k, C.size_t(len(fields)), &out, &length)
	return libraryMade(code, out, length)
}

func libraryRead(format, input, options, kind string) (string, error) {
	return libraryCall(true, format, input, options, kind)
}

func libraryWrite(format, value, options, kind string) (string, error) {
	return libraryCall(false, format, value, options, kind)
}
