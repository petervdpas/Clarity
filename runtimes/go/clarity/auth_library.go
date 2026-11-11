//go:build clarityauth

package clarity

// Clarity.Auth through cgo, for the runtime's own test of sign-in
// (signin_test.go): built only with the tag clarityauth, so the runtime
// stays pure Go. A program has its own copy and hands it to Db.AddSignin.

/*
#cgo LDFLAGS: -lclarity_auth
#include <stdlib.h>
#include "clarity_auth.h"

static int32_t auth_call(const char* o, size_t on, const char* a, size_t an, uint8_t** out, size_t* len) { return clarity_auth_api()->call(o, on, a, an, out, len); }
static void auth_free(uint8_t* bytes, size_t len) { clarity_auth_api()->free(bytes, len); }
static int32_t auth_problem(char* into, size_t size) { return clarity_auth_api()->problem(into, size); }
*/
import "C"

import (
	"errors"
	"unsafe"
)

func libraryAuthCall(op, args string) (string, error) {
	o, a := C.CString(op), C.CString(args)
	defer C.free(unsafe.Pointer(o))
	defer C.free(unsafe.Pointer(a))
	var out *C.uint8_t
	var length C.size_t
	if C.auth_call(o, C.size_t(len(op)), a, C.size_t(len(args)), &out, &length) != C.CLARITY_AUTH_OK {
		var why [1024]C.char
		C.auth_problem(&why[0], C.size_t(len(why)))
		return "", errors.New(C.GoString(&why[0]))
	}
	made := C.GoStringN((*C.char)(unsafe.Pointer(out)), C.int(length))
	C.auth_free(out, length)
	return made, nil
}
