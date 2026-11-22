package shell

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// How long a browser page may be quiet is the spec's (conformance/defaults.json).
func TestTheQuietTimeIsTheSpecs(t *testing.T) {
	text, err := os.ReadFile(filepath.Join("..", "..", "..", "conformance", "defaults.json"))
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		QuietMinutes int64 `json:"quietMinutes"`
	}
	if err := json.Unmarshal(text, &file); err != nil {
		t.Fatal(err)
	}
	if got := int64(Quiet / time.Minute); got != file.QuietMinutes {
		t.Fatalf("quiet: %d minutes, the spec says %d", got, file.QuietMinutes)
	}
}
