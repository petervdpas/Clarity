package clarity

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// Clarity's defaults are the spec's (conformance/defaults.json); the copies
// this runtime carries say the same.
type defaultsFile struct {
	Signin struct {
		SessionMinutes int64 `json:"sessionMinutes"`
		Tries          int64 `json:"tries"`
		TriesMinutes   int64 `json:"triesMinutes"`
	} `json:"signin"`
	UndoSteps     int64 `json:"undoSteps"`
	HTTPSeconds   int64 `json:"httpSeconds"`
	OwnersPerRead int64 `json:"ownersPerRead"`
	EditBusKeeps  int64 `json:"editBusKeeps"`
	QuietMinutes  int64 `json:"quietMinutes"`
}

func readDefaults(t *testing.T) defaultsFile {
	text, err := os.ReadFile(filepath.Join("..", "..", "..", "conformance", "defaults.json"))
	if err != nil {
		t.Fatal(err)
	}
	var file defaultsFile
	if err := json.Unmarshal(text, &file); err != nil {
		t.Fatal(err)
	}
	return file
}

func TestTheDefaultsAreTheSpecs(t *testing.T) {
	file := readDefaults(t)
	for _, c := range []struct {
		name      string
		got, want int64
	}{
		{"sign-in session", SigninSessionSeconds, file.Signin.SessionMinutes * 60},
		{"sign-in tries", SigninTries, file.Signin.Tries},
		{"sign-in wait", SigninTriesSeconds, file.Signin.TriesMinutes * 60},
		{"undo steps", UndoSteps, file.UndoSteps},
		{"http", int64(HTTPTimeout / time.Second), file.HTTPSeconds},
		{"owners per read", OwnersPerRead, file.OwnersPerRead},
		{"edit bus", maxEdits, file.EditBusKeeps},
	} {
		if c.got != c.want {
			t.Errorf("%s: %d, the spec says %d", c.name, c.got, c.want)
		}
	}
}
