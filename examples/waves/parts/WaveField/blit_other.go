//go:build !windows && (!linux || !cgo)

package wavefield

// Where the part has no window of its own, the picture goes nowhere; the
// water still runs, so the part's values and tells are the same everywhere.

func openScreen(Window) screen { return nothing{} }
