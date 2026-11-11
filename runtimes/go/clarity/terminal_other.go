//go:build !(linux || darwin || freebsd || netbsd || openbsd || dragonfly || windows)

package clarity

import "os"

// readHidden refuses on a system whose terminal Clarity cannot hide typing on.
func readHidden(in *os.File) (string, error) {
	return "", &ValueError{What: "this system's terminal cannot hide what is typed"}
}
