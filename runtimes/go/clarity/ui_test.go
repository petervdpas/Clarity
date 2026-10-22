// The binding between a window on the screen and its handlers: the state
// the screen sends, and the reply it gets, in the same JSON as from the
// Rust runtime.

package clarity

import (
	"encoding/json"
	"fmt"
	"strings"
	"testing"
)

// An order as a window holds it.
type order struct {
	ID       int64
	Number   string
	Customer *int64
	Total    int64
	Placed   *DateTime
	Paid     bool
	Code     string
}

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
	equal(t, string(wire), `{"state":{"window":"Card","instance":0,"data":{"order":{"Code":"","Customer":null,"ID":7,"Number":"ORD-8","Paid":false,"Placed":null,"Total":0},"orders":[{"Code":"","Customer":null,"ID":1,"Number":"A","Paid":false,"Placed":null,"Total":0},{"Code":"","Customer":null,"ID":2,"Number":"B","Paid":false,"Placed":null,"Total":0}]},"parts":{"mine":{"checked":true},"name":{"error":"Name is required","readonly":false},"name_filter":{"text":""},"save":{"enabled":false}},"selected":{"grid":1}},"messages":[{"level":"info","text":{"text":"Saved {n}","values":{"n":{"kind":"text","value":"ORD-7"}}}}],"open":[{"window":"Order","with":{"order":{"ID":7,"Number":"ORD-7","Customer":null,"Total":0,"Placed":null,"Paid":false,"Code":""}}}],"close":true}`)

	// The data goes through a map, so its keys are sorted, as from the Rust runtime.
	// A new window's state has its number, and no parts and no selected rows yet.
	opened := NewWindowState("Card", map[string]any{})
	fresh, _ := json.Marshal(opened)
	equal(t, string(fresh), fmt.Sprintf(`{"window":"Card","instance":%d,"data":{},"parts":{},"selected":{}}`, opened.Instance))
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

func TestRowsReadAgainHaveNoSelectedRow(t *testing.T) {
	ui := NewUi(WindowState{Window: "Orders", Selected: map[string]int{"grid": 1, "other": 0}})
	rows := []string{"a", "b"}
	ui.Deselect("grid")
	if _, err := Selected(ui, "grid", rows); err == nil {
		t.Fatal("a table read again kept its selected row")
	}
	reply, err := ui.Reply("Orders", map[string]any{})
	must(t, err)
	if _, ok := reply.State.Selected["grid"]; ok {
		t.Fatal("the reply kept the selected row")
	}
	equal(t, reply.State.Selected["other"], 0)
}

// Every window that opens has a number of its own, which its handlers'
// replies keep, so what belongs to one open window (an engine's state) is
// found again and dropped when it closes.
func TestEveryOpenWindowHasItsOwnNumber(t *testing.T) {
	first, second := NewWindowState("Orders", nil), NewWindowState("Orders", nil)
	if first.Instance == 0 || second.Instance == 0 || first.Instance == second.Instance {
		t.Fatalf("%d %d", first.Instance, second.Instance)
	}
	ui := NewUi(first)
	if ui.Instance() != first.Instance {
		t.Fatalf("ui %d", ui.Instance())
	}
	reply, err := ui.Reply("Orders", map[string]any{})
	if err != nil || reply.State.Instance != first.Instance {
		t.Fatalf("reply %d %v", reply.State.Instance, err)
	}
	// The number travels with the state to the screen and back.
	text, _ := json.Marshal(first)
	var back WindowState
	if err := json.Unmarshal(text, &back); err != nil || back.Instance != first.Instance {
		t.Fatalf("%s %v", text, err)
	}
}

func TestAClosedWindowIsReportedToWhatListens(t *testing.T) {
	var closed []uint64
	OnWindowClosed(func(instance uint64) { closed = append(closed, instance) })
	WindowClosed(7)
	if len(closed) != 1 || closed[0] != 7 {
		t.Fatal(closed)
	}
}

// A text value a handler sets travels as the text as written with its
// slots, so the screen shows it in the person's language and format.
func TestAReplyCarriesTheTextsShownInTextValues(t *testing.T) {
	ui := NewUi(WindowState{Window: "Game"})
	ui.SetShown("rules", NewText("Your move, {name}").With("name", SlotText("Ada")))
	ui.SetShown("hint", PlainText("Pick a square"))
	reply, err := ui.Reply("Game", nil)
	if err != nil {
		t.Fatal(err)
	}
	var got map[string]any
	text, _ := json.Marshal(reply)
	_ = json.Unmarshal(text, &got)
	shown := got["shown"].(map[string]any)
	if shown["hint"] != "Pick a square" {
		t.Fatalf("hint: %v", shown["hint"])
	}
	rules := shown["rules"].(map[string]any)
	if rules["text"] != "Your move, {name}" || rules["values"].(map[string]any)["name"].(map[string]any)["value"] != "Ada" {
		t.Fatalf("rules: %v", rules)
	}
	quiet, _ := NewUi(WindowState{Window: "Game"}).Reply("Game", nil)
	text, _ = json.Marshal(quiet)
	var none map[string]any
	if json.Unmarshal(text, &none); none["shown"] != nil {
		t.Fatalf("a reply without shown texts carries none: %s", text)
	}
}

func TestAQuestionTravelsInTheReplyAsFromTheRustRuntime(t *testing.T) {
	ui := NewUi(NewWindowState("Albums", nil))
	plain, err := ui.Reply("Albums", nil)
	must(t, err)
	text, _ := json.Marshal(plain)
	if strings.Contains(string(text), `"asks"`) {
		t.Fatal("no asks, no field")
	}
	yes := KeyText("delete")
	ui.Ask(KeyText("deleteAlbum").With("title", SlotText("Blue")), Confirm(&yes, nil), "deleteConfirmed")
	friend := "Friend"
	ui.Ask(KeyText("lendTo"), ValueForm("record", &friend, []any{3, 5}, nil), "lend", map[string]any{"id": 7, "title": "Blue"})
	reply, err := ui.Reply("Albums", nil)
	must(t, err)
	got, _ := json.Marshal(reply.Asks)
	want := `[{"question":{"key":"deleteAlbum","values":{"title":{"kind":"text","value":"Blue"}}},"form":{"kind":"confirm","yes":{"key":"delete"},"no":null},"handler":"deleteConfirmed","values":[]},` +
		`{"question":{"key":"lendTo"},"form":{"kind":"value","type":"record","of":"Friend","only":[3,5],"default":null},"handler":"lend","values":[{"id":7,"title":"Blue"}]}]`
	equal(t, string(got), want)
}

func TestAnAnswerBecomesTheValueItsHandlerTakes(t *testing.T) {
	n, err := Answer[int64](json.RawMessage(`3`))
	must(t, err)
	equal(t, n, int64(3))
	name, err := Answer[string](json.RawMessage(`"Ada"`))
	must(t, err)
	equal(t, name, "Ada")
	if _, err := Answer[int64](json.RawMessage(`"three"`)); err == nil {
		t.Fatal("text was taken as a number")
	}
}

func TestTheChoicesFromAListAreItsRecordsIDs(t *testing.T) {
	got := IDs([]album{{ID: 3}, {ID: 5}})
	equal(t, fmt.Sprint(got), "[3 5]")
}
