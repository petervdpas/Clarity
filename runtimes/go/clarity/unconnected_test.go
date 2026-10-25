// Why the databases did not open (DESIGN 5.27), in Clarity's own words that
// the project translates, as every runtime says it (conformance/unconnected.json).

package clarity

import (
	"encoding/json"
	"os"
	"reflect"
	"strings"
	"testing"
)

func unconnectedCases(t *testing.T) map[string]json.RawMessage {
	t.Helper()
	text, err := os.ReadFile("../../../conformance/unconnected.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases map[string]json.RawMessage
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	return cases
}

func sameText(t *testing.T, got Text, want json.RawMessage) {
	t.Helper()
	var wanted Text
	if err := json.Unmarshal(want, &wanted); err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(got, wanted) {
		t.Fatalf("got %#v, want %#v", got, wanted)
	}
}

func TestASecretConnectionNotKeptSaysSo(t *testing.T) {
	var notKept struct {
		Location string          `json:"location"`
		Reason   json.RawMessage `json:"reason"`
	}
	if err := json.Unmarshal(unconnectedCases(t)["notKept"], &notKept); err != nil {
		t.Fatal(err)
	}
	_, err := Connect(Source{"main", Sqlite, notKept.Location})
	sameText(t, UnconnectedReason(err), notKept.Reason)
	// Its message stays as it was, for logs.
	if !strings.Contains(err.Error(), "the secret setting neverKept is not kept yet") {
		t.Fatal(err)
	}
}

func TestAnyOtherFailureSaysTheRuntimesMessage(t *testing.T) {
	var failed struct {
		Storage string          `json:"storage"`
		Reason  json.RawMessage `json:"reason"`
	}
	if err := json.Unmarshal(unconnectedCases(t)["failed"], &failed); err != nil {
		t.Fatal(err)
	}
	sameText(t, UnconnectedReason(&StorageError{What: failed.Storage}), failed.Reason)
}
