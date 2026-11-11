//go:build !clarityauth

package clarity

import "github.com/petervdpas/Clarity/runtimes/go/internal/authfake"

// fakeAuth stands in for Clarity.Auth without cgo; one for the whole run,
// as the library's tries and sessions are kept for the whole program.
var fakeAuth = authfake.New()

func testAuth() AuthCall { return fakeAuth.Call }
