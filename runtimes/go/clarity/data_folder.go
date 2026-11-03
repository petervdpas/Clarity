// The folder a program's SQLite databases open in when the environment
// variable CLARITY_DATA names one: each under its own file name, so a run in
// the IDE or by hand (a live check on a copy) chooses its data without
// editing the generated program. The same as the Rust and C# runtimes.

package clarity

import (
	"path/filepath"
	"strings"
)

// DataVariable is the environment variable that names the folder.
const DataVariable = "CLARITY_DATA"

// DataFolderSqlite is the file of an SQLite database at location, with env
// reading the environment: in the folder CLARITY_DATA names, under its own
// file name, or where the project put it when the variable is unset or
// empty. A location from the environment (env:) or a secret setting
// (setting:) is as it says, and so is SQLite's own :memory:.
func DataFolderSqlite(location string, env func(string) string) string {
	if strings.HasPrefix(location, "env:") || strings.HasPrefix(location, "setting:") || location == ":memory:" {
		return location
	}
	folder := env(DataVariable)
	name := filepath.Base(location)
	if folder == "" || location == "" || name == "." || name == ".." || name == string(filepath.Separator) {
		return location
	}
	return filepath.Join(folder, name)
}
