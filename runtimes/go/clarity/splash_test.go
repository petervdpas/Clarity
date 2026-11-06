package clarity

import "testing"

func TestTheLastSplashStatusSaidIsTheOneShown(t *testing.T) {
	SetSplashStatus(PlainText("Loading"))
	SetSplashStatus(KeyText("openingDatabase"))
	if got := SplashStatus(); got == nil || got.Key != "openingDatabase" {
		t.Fatalf("got %v", got)
	}
}
