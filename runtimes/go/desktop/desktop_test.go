package desktop

import "testing"

func TestADesktopWindowOpensAtItsWindowsSizeOrTheStandardOne(t *testing.T) {
	if w, h := size(320, 460); w != 320 || h != 460 {
		t.Fatalf("got %dx%d", w, h)
	}
	if w, h := size(0, 0); w != 1100 || h != 760 {
		t.Fatalf("got %dx%d for no size", w, h)
	}
}
