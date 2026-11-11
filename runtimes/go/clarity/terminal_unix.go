//go:build linux || darwin || freebsd || netbsd || openbsd || dragonfly

package clarity

import (
	"os"

	"golang.org/x/sys/unix"
)

// readHidden reads a line from the terminal in, its echo off while it does.
func readHidden(in *os.File) (string, error) {
	fd := int(in.Fd())
	was, err := unix.IoctlGetTermios(fd, termiosGet)
	if err != nil {
		return "", &ValueError{What: "there is no terminal to type the password on"}
	}
	quiet := *was
	quiet.Lflag &^= unix.ECHO
	quiet.Lflag |= unix.ICANON | unix.ISIG
	quiet.Iflag |= unix.ICRNL
	if err := unix.IoctlSetTermios(fd, termiosSet, &quiet); err != nil {
		return "", &ValueError{What: "the terminal does not hide what is typed"}
	}
	defer unix.IoctlSetTermios(fd, termiosSet, was)
	return readLine(in)
}
