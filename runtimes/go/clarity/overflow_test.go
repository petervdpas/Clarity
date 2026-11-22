package clarity

import (
	"encoding/json"
	"errors"
	"os"
	"strconv"
	"testing"

	"github.com/shopspring/decimal"
)

// Whole numbers and decimals that do not fit, as every stack treats them
// (conformance/overflow.json): a value error, never a wrapped number.
func TestNumbersThatDoNotFitAreAValueError(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/overflow.json")
	must(t, err)
	var file struct {
		Cases []struct {
			Kind, Op, A, B, Result string
			Error                  bool
		} `json:"cases"`
	}
	must(t, json.Unmarshal(text, &file))
	for _, c := range file.Cases {
		var got string
		var err error
		if c.Kind == "whole" {
			a, _ := strconv.ParseInt(c.A, 10, 64)
			b, _ := strconv.ParseInt(c.B, 10, 64)
			var n int64
			switch c.Op {
			case "+":
				n, err = WholeAdd(a, b)
			case "-":
				n, err = WholeSub(a, b)
			case "*":
				n, err = WholeMul(a, b)
			case "div":
				n, err = WholeDiv(a, b)
			case "mod":
				n, err = WholeMod(a, b)
			case "neg":
				n, err = WholeNeg(a)
			}
			got = strconv.FormatInt(n, 10)
		} else {
			a := decimal.RequireFromString(c.A)
			var d decimal.Decimal
			switch c.Op {
			case "+":
				d, err = DecimalAdd(a, decimal.RequireFromString(c.B))
			case "-":
				d, err = DecimalSub(a, decimal.RequireFromString(c.B))
			case "*":
				d, err = DecimalMul(a, decimal.RequireFromString(c.B))
			case "neg":
				d = a.Neg()
			}
			got = DecimalText(d)
		}
		if c.Error {
			var value *ValueError
			if !errors.As(err, &value) {
				t.Errorf("%s %s %s %s: a value error, not %s %v", c.Kind, c.A, c.Op, c.B, got, err)
			}
			continue
		}
		if err != nil || got != c.Result {
			t.Errorf("%s %s %s %s: %s %v, want %s", c.Kind, c.A, c.Op, c.B, got, err, c.Result)
		}
	}
}
