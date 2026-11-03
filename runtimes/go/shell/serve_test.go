// The web target (DESIGN 7.3): where the server listens. What it serves
// and answers is in program_test.go.

package shell

import "testing"

func TestTheServerListensWhereItIsTold(t *testing.T) {
	t.Setenv("CLARITY_ADDRESS", "")
	if got := Address([]string{"--serve"}, "127.0.0.1:9090"); got != "127.0.0.1:9090" {
		t.Fatal(got)
	}
	if got := Address([]string{"--serve", "--address", "0.0.0.0:9000"}, "127.0.0.1:9090"); got != "0.0.0.0:9000" {
		t.Fatal(got)
	}
	t.Setenv("CLARITY_ADDRESS", ":7000")
	if got := Address(nil, "127.0.0.1:9090"); got != ":7000" {
		t.Fatal(got)
	}
}
