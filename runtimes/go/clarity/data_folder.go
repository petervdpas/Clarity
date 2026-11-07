// Where a program's SQLite databases open (conformance/data-folder.json): a
// database the project keeps beside it is written data:<program>/<file>, and
// opens in the folder CLARITY_DATA names (Clarity's Run names the project's
// data folder), else in the person's folder for application data under the
// program's name. CLARITY_DATA moves any SQLite file there under its own
// file name. The same in every runtime.

package clarity

import (
	"path/filepath"
	"runtime"
	"strings"
)

// DataVariable is the environment variable that names the folder.
const DataVariable = "CLARITY_DATA"

// DataFolderSqlite is the file of an SQLite database at location, with env
// reading the environment, on this system.
func DataFolderSqlite(location string, env func(string) string) string {
	return DataFolderSqliteIn(runtime.GOOS, location, env)
}

// DataFolderSqliteIn is DataFolderSqlite on system goos (windows, darwin or
// another). A location from the environment (env:) or a secret setting
// (setting:) is as it says, and so is SQLite's own :memory:.
func DataFolderSqliteIn(goos, location string, env func(string) string) string {
	if strings.HasPrefix(location, "env:") || strings.HasPrefix(location, "setting:") || location == ":memory:" {
		return location
	}
	kept, isKept := strings.CutPrefix(location, "data:")
	name := location
	if isKept {
		name = kept
		if at := strings.LastIndex(kept, "/"); at >= 0 {
			name = kept[at+1:]
		}
	}
	if folder := env(DataVariable); folder != "" {
		base := filepath.Base(name)
		if name != "" && base != "." && base != ".." && base != string(filepath.Separator) {
			return filepath.Join(folder, base)
		}
	}
	if !isKept {
		return location
	}
	// The person's folder for application data, under the program's name.
	var folder string
	switch goos {
	case "windows":
		folder = env("APPDATA")
	case "darwin", "macos":
		if home := env("HOME"); home != "" {
			folder = home + "/Library/Application Support"
		}
	default:
		folder = env("XDG_DATA_HOME")
		if folder == "" {
			if home := env("HOME"); home != "" {
				folder = home + "/.local/share"
			}
		}
	}
	if folder == "" {
		return name
	}
	return folder + "/" + kept
}
