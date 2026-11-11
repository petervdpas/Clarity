//go:build darwin || freebsd || netbsd || openbsd || dragonfly

package clarity

import "golang.org/x/sys/unix"

// The requests that read and set a terminal's modes on macOS and the BSDs.
const (
	termiosGet = unix.TIOCGETA
	termiosSet = unix.TIOCSETA
)
