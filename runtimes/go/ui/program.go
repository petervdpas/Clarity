package ui

import (
	"io/fs"

	"github.com/petervdpas/Clarity/runtimes/go/shell"
)

// Engines are the engines beside the screen of a program (DESIGN 5.12),
// each instance by its number; package engine's Screens is one.
type Engines = shell.Engines

// Program answers the library's commands (clarity_ui.h) for a generated
// application; package shell's, which answers a browser's the same way.
type Program = shell.Program

// Desktop is what a generated program gives Clarity.UI to run it.
type Desktop struct {
	// Options are the service's, which answers the windows' commands.
	Options shell.Options
	// Files hold windows/, apps/, themes/ and resources/ at their root.
	Files fs.FS
	// Facts are what the program knows of its fields and types, as JSON
	// (the runtime's fields facts), which the parts read beside the values; nil for none.
	Facts []byte
	// Watches is whether an engine says how far a long call is: the library
	// then runs handlers off its own thread and shows the bar with Stop.
	Watches bool
	// Engines are the engines beside the screen; nil for none.
	Engines Engines
}

// NewProgram is the program of desktop, with its service.
func NewProgram(desktop Desktop) *Program {
	return shell.NewProgram(shell.ProgramOptions{Options: desktop.Options, Facts: desktop.Facts, Watches: desktop.Watches, Engines: desktop.Engines})
}
