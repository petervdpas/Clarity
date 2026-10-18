// The binding between a window on the screen and its handlers: the state
// the screen sends, and the reply it gets, in the same JSON as from the
// Rust runtime.

package clarity

import (
	"encoding/json"
	"testing"
)

type cardWindow struct {
	Order  *order  `json:"order"`
	Orders []order `json:"orders"`
}

func TestAHandlerRepliesAsTheRustRuntimeDoes(t *testing.T) {
	var state WindowState
	sent := `{"window":"Card","data":{"order":{"ID":7,"Number":"ORD-7"},"orders":[{"ID":1,"Number":"A"},{"ID":2,"Number":"B"}]},"parts":{"name_filter":{"text":"Ada"},"mine":{"checked":true}},"selected":{"grid":1}}`
	must(t, json.Unmarshal([]byte(sent), &state))
	var w cardWindow
	must(t, state.Decode(&w))
	equal(t, w.Order.Number, "ORD-7")

	ui := NewUi(state)
	equal(t, ui.Text("name_filter"), "Ada")
	equal(t, ui.Text("other"), "")
	equal(t, ui.Checked("mine"), true)
	equal(t, ui.Visible("save"), true)
	equal(t, ui.Readonly("name", true), true)
	equal(t, ui.Caption("save", "Save"), "Save")
	picked, err := Selected(ui, "grid", w.Orders)
	must(t, err)
	equal(t, picked.Number, "B")
	if _, err := Selected(ui, "other", w.Orders); err == nil {
		t.Fatal("a table without a selected row gave one")
	}

	ui.SetEnabled("save", false)
	ui.SetError("name", PlainText("Name is required"))
	ui.SetReadonly("name", false)
	ui.SetText("name_filter", "")
	ui.Message(Info, NewText("Saved {n}").With("n", SlotText("ORD-7")))
	ui.Open("Order", map[string]any{"order": w.Order})
	ui.Close()
	w.Order.Number = "ORD-8"
	reply, err := ui.Reply("Card", &w)
	must(t, err)
	wire, err := json.Marshal(reply)
	must(t, err)
	equal(t, string(wire), `{"state":{"window":"Card","data":{"order":{"Code":"","Customer":null,"ID":7,"Number":"ORD-8","Paid":false,"Placed":null,"Total":0},"orders":[{"Code":"","Customer":null,"ID":1,"Number":"A","Paid":false,"Placed":null,"Total":0},{"Code":"","Customer":null,"ID":2,"Number":"B","Paid":false,"Placed":null,"Total":0}]},"parts":{"mine":{"checked":true},"name":{"error":"Name is required","readonly":false},"name_filter":{"text":""},"save":{"enabled":false}},"selected":{"grid":1}},"messages":[{"level":"info","text":{"text":"Saved {n}","values":{"n":{"kind":"text","value":"ORD-7"}}}}],"open":[{"window":"Order","with":{"order":{"ID":7,"Number":"ORD-7","Customer":null,"Total":0,"Placed":null,"Paid":false,"Code":""}}}],"close":true}`)

	// The data goes through a map, so its keys are sorted, as from the Rust runtime.
	// A new window's state has no parts and no selected rows yet.
	fresh, _ := json.Marshal(NewWindowState("Card", map[string]any{}))
	equal(t, string(fresh), `{"window":"Card","data":{},"parts":{},"selected":{}}`)
}

// A window opens with its loaded data and with filled in by name; data
// that does not fit the window is refused.
func TestAWindowOpensWithItsDataAndWith(t *testing.T) {
	w := &cardWindow{Order: &order{Number: "new"}}
	state, err := Opened("Card", w, map[string]any{"order": map[string]any{"ID": 7, "Number": "ORD-7"}})
	must(t, err)
	var back cardWindow
	must(t, state.Decode(&back))
	equal(t, back.Order.Number, "ORD-7")
	if _, err := Opened("Card", &cardWindow{}, map[string]any{"order": "not an order"}); err == nil {
		t.Fatal("an order that is text was taken")
	}
}
