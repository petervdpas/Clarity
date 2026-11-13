package clarity

import (
	"encoding/json"
	"strings"
	"testing"
)

// A toast travels as a message with the look its declaration gives: its
// name, level, lifespan, icon and corner; a plain message carries none.
func TestAToastTravelsWithItsLook(t *testing.T) {
	ui := NewUi(WindowState{Window: "Card"})
	ui.Message(Info, PlainText("Plain"))
	ui.Toast(Toast{Name: "saved", Level: "success", Lifespan: 3000, Icon: "check", Corner: "bottomRight"}, PlainText("Saved Ada"))
	reply, err := ui.Reply("Card", map[string]any{})
	must(t, err)
	wire, err := json.Marshal(reply.Messages)
	must(t, err)
	equal(t, string(wire), `[{"level":"info","text":"Plain"},{"level":"info","text":"Saved Ada","toast":{"name":"saved","level":"success","lifespan":3000,"icon":"check","corner":"bottomRight"}}]`)
	if strings.Contains(string(wire), `"toast":null`) {
		t.Fatal("a plain message carries no toast")
	}
}
