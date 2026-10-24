//go:build (!linux || !cgo) && !windows

package parts

import "errors"

func openPlatform() (windows, error) {
	return nil, errors.New("custom parts drawn by a native library run on Linux and Windows")
}
