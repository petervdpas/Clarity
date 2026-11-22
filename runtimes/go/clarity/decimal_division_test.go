package clarity

import (
	"encoding/json"
	"errors"
	"os"
	"testing"

	"github.com/shopspring/decimal"
)

// Dividing decimals as every stack divides them (conformance/decimal-division.json):
// generated Go divides through DecimalDiv, a value error for dividing by zero.
func TestDecimalsDivideAsEveryStackDividesThem(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/decimal-division.json")
	must(t, err)
	var file struct {
		Cases []struct {
			A, B, Quotient string
			Error          bool
		} `json:"cases"`
	}
	must(t, json.Unmarshal(text, &file))
	for _, c := range file.Cases {
		q, err := DecimalDiv(decimal.RequireFromString(c.A), decimal.RequireFromString(c.B))
		if c.Error {
			var value *ValueError
			if !errors.As(err, &value) {
				t.Errorf("%s / %s: a value error, not %v", c.A, c.B, err)
			}
			continue
		}
		if err != nil || DecimalText(q) != c.Quotient {
			t.Errorf("%s / %s: %s %v, want %s", c.A, c.B, DecimalText(q), err, c.Quotient)
		}
	}
}
