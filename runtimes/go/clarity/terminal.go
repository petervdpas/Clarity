// Reading from the terminal without showing what is typed (a password on
// the command line, signin_cli.go); terminal_<system>.go turn the echo off.

package clarity

import (
	"io"
	"strings"
)

// readLine reads up to the end of a line, a byte at a time, so nothing
// after it is taken from the terminal.
func readLine(in io.Reader) (string, error) {
	var line strings.Builder
	one := make([]byte, 1)
	for {
		n, err := in.Read(one)
		if n == 1 {
			if one[0] == '\n' {
				return line.String(), nil
			}
			line.WriteByte(one[0])
		}
		if err == io.EOF {
			return line.String(), nil
		}
		if err != nil {
			return "", err
		}
	}
}
