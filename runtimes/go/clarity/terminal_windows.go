//go:build windows

package clarity

import (
	"os"

	"golang.org/x/sys/windows"
)

// readHidden reads a line from the console in, its echo off while it does.
func readHidden(in *os.File) (string, error) {
	handle := windows.Handle(in.Fd())
	var was uint32
	if err := windows.GetConsoleMode(handle, &was); err != nil {
		return "", &ValueError{What: "there is no terminal to type the password on"}
	}
	quiet := (was &^ windows.ENABLE_ECHO_INPUT) | windows.ENABLE_PROCESSED_INPUT | windows.ENABLE_LINE_INPUT
	if err := windows.SetConsoleMode(handle, quiet); err != nil {
		return "", &ValueError{What: "the terminal does not hide what is typed"}
	}
	defer windows.SetConsoleMode(handle, was)
	return readLine(in)
}
