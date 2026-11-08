package clarity

import "testing"

// WholeDiv and WholeMod are `a div b` and `a mod b`: rounded toward zero, the
// sign of a, and a division by zero an error.
func TestWholeNumbersDivideTowardZero(t *testing.T) {
	cases := []struct{ a, b, div, mod int64 }{{7, 2, 3, 1}, {-7, 2, -3, -1}, {7, -2, -3, 1}, {-9223372036854775808, -1, -9223372036854775808, 0}}
	for _, c := range cases {
		d, err := WholeDiv(c.a, c.b)
		m, err2 := WholeMod(c.a, c.b)
		if err != nil || err2 != nil || d != c.div || m != c.mod {
			t.Fatalf("%d div/mod %d: %d %d (%v %v)", c.a, c.b, d, m, err, err2)
		}
	}
	if _, err := WholeDiv(1, 0); err == nil || err.Error() != "division by zero" {
		t.Fatalf("division by zero: %v", err)
	}
	if _, err := WholeMod(1, 0); err == nil {
		t.Fatal("mod by zero")
	}
}
