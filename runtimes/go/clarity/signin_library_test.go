//go:build clarityauth

package clarity

// testAuth is Clarity.Auth itself, with the tag clarityauth.
func testAuth() AuthCall { return libraryAuthCall }
