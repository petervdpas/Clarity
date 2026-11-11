//go:build linux

package clarity

import "golang.org/x/sys/unix"

// The requests that read and set a terminal's modes on Linux.
const (
	termiosGet = unix.TCGETS
	termiosSet = unix.TCSETS
)
