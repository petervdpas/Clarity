//go:build clarityauth

package clarity

import "github.com/petervdpas/Clarity/runtimes/go/internal/authlib"

// testAuth is Clarity.Auth itself, with the tag clarityauth.
func testAuth() AuthCall { return authlib.Call }
