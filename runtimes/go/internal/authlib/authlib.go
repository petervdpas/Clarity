//go:build clarityauth

// Package authlib is Clarity.Auth through cgo, for the runtime's own tests
// of sign-in (clarity and shell): built only with the tag clarityauth, so
// the runtime stays pure Go. A program has its own copy and hands it to
// Db.AddSignin. Every sign-in test runs against the library itself, never a
// stand-in, so what it decides is decided once (runtimes/rust-auth).
package authlib

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
	"encoding/json"
	"errors"
	"unsafe"
)

// Call is one operation of Clarity.Auth by name, its arguments and its result as JSON.
func Call(op, args string) (string, error) {
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

// Hash is text kept by Clarity.Auth's hash `method` (argon2id, sha256), as a stored secret.
func Hash(method, text string) string {
	args, _ := json.Marshal(map[string]string{"method": method, "text": text})
	out, err := Call("hash", string(args))
	if err != nil {
		panic(err)
	}
	var hashed string
	if err := json.Unmarshal([]byte(out), &hashed); err != nil {
		panic(err)
	}
	return hashed
}
