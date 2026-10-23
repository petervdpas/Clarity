//go:build !linux || !cgo

package parts

import "errors"

func openPlatform() (windows, error) {
	return nil, errors.New("custom parts run on Linux for now")
}
