//go:build clarityformats

package clarity

// The data formats of code end to end on the Go runtime
// (conformance/formats-runtime.json): read through Clarity.Formats, decoded
// by encoding/json into records as the generator writes them, written back.
// Built with the tag clarityformats (formats_library.go): CGO_CFLAGS names
// runtimes/rust-formats-ffi/include and CGO_LDFLAGS the folder of its build.

import (
	"encoding/json"
	"os"
	"testing"

	"github.com/shopspring/decimal"
)

type formatsLine struct {
	ID       int64  `json:"id"`
	Product  string `json:"product"`
	Quantity int64  `json:"quantity"`
}

type formatsOrder struct {
	ID     int64           `json:"id"`
	Number string          `json:"number"`
	Total  decimal.Decimal `json:"total"`
	Placed *Date           `json:"placed"`
	Lines  []formatsLine   `json:"lines"`
}

func TestFormatsReadDecodeAndWriteAsEveryStackDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/formats-runtime.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Cases []struct {
			Name    string          `json:"name"`
			Kind    json.RawMessage `json:"kind"`
			Options json.RawMessage `json:"options"`
			Input   string          `json:"input"`
			Written *string         `json:"written"`
			Error   *string         `json:"error"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(text, &file); err != nil {
		t.Fatal(err)
	}
	reader, writer := FormatReader(libraryRead), FormatWriter(libraryWrite)
	for _, c := range file.Cases {
		kind := string(c.Kind)
		options := string(c.Options)
		var written string
		var err error
		var probe struct{ Kind string `json:"kind"` }
		_ = json.Unmarshal(c.Kind, &probe)
		if probe.Kind == "records" {
			var orders []formatsOrder
			orders, err = ReadFormat[[]formatsOrder](reader, "json", c.Input, options, kind)
			written = WriteFormat(writer, "json", orders, "", kind)
		} else {
			var order formatsOrder
			order, err = ReadFormat[formatsOrder](reader, "json", c.Input, options, kind)
			written = WriteFormat(writer, "json", order, "", kind)
		}
		switch {
		case c.Written != nil:
			if err != nil {
				t.Fatalf("%s: %v", c.Name, err)
			}
			if written != *c.Written {
				t.Errorf("%s:\n got %s\nwant %s", c.Name, written, *c.Written)
			}
		case c.Error != nil:
			if err == nil || err.Error() != *c.Error {
				t.Errorf("%s: got %v, want %s", c.Name, err, *c.Error)
			}
		}
	}
}
