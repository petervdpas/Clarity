// The settings each person keeps for themselves (DESIGN 5.17): one JSON
// file per application in the folder for application settings of the
// person's system (conformance/settings.json, `file`). The screen decides
// which kept values still fit; this only reads and keeps the file.

package clarity

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
)

// SettingsFileIn is where program keeps its settings on system goos
// ("windows", "macos" or another), with env giving the environment; false
// when it names no folder for application settings.
func SettingsFileIn(goos string, env func(string) string, program string) (string, bool) {
	var folder string
	switch goos {
	case "windows":
		folder = env("APPDATA")
	case "macos", "darwin":
		if home := env("HOME"); home != "" {
			folder = home + "/Library/Application Support"
		}
	default:
		folder = env("XDG_CONFIG_HOME")
		if home := env("HOME"); folder == "" && home != "" {
			folder = home + "/.config"
		}
	}
	if folder == "" {
		return "", false
	}
	return folder + "/" + program + "/settings.json", true
}

// SettingsFile is where program keeps its settings on this machine.
func SettingsFile(program string) (string, bool) {
	file, ok := SettingsFileIn(runtime.GOOS, os.Getenv, program)
	return filepath.FromSlash(file), ok
}

// ReadSettings gives the settings program kept; none when it kept none, or
// they cannot be read.
func ReadSettings(program string) map[string]any {
	kept := map[string]any{}
	if file, ok := SettingsFile(program); ok {
		if text, err := os.ReadFile(file); err == nil {
			_ = json.Unmarshal(text, &kept)
		}
	}
	return kept
}

// KeepSettings keeps values as the settings of program, written whole and
// then put in place, so a crash leaves the old file or the new one.
func KeepSettings(program string, values map[string]any) error {
	file, ok := SettingsFile(program)
	if !ok {
		return fmt.Errorf("this system names no folder for application settings")
	}
	text, err := json.MarshalIndent(values, "", "  ")
	if err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(file), 0o755); err != nil {
		return fmt.Errorf("cannot keep the settings in %s: %w", file, err)
	}
	written := file + ".new"
	if err := os.WriteFile(written, text, 0o644); err != nil {
		return fmt.Errorf("cannot keep the settings in %s: %w", file, err)
	}
	return os.Rename(written, file)
}
