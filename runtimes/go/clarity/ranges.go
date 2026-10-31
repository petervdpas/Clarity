package clarity

// The slider and the spin box of U1c, decided here in the runtime
// (conformance/ranges.json): how a key, a drag or a typed number moves a
// value within its range. Numbers come as text and stay exact.

import (
	"fmt"
	"strings"

	"github.com/shopspring/decimal"
)

// NumberRange is a slider's or a spin box's range: nil Min or Max is no end; an
// empty Step is 1.
type NumberRange struct {
	Min, Max *string
	Step     string
}

// keySteps are the steps a key moves, for the keys that move by steps.
var keySteps = map[string]int64{"up": 1, "down": -1, "pageUp": 10, "pageDown": -10}

func exactNumber(text string) (decimal.Decimal, error) {
	n, err := decimal.NewFromString(strings.TrimSpace(text))
	if err != nil {
		return decimal.Decimal{}, &InvalidError{fmt.Sprintf("%q is no number", text)}
	}
	return n, nil
}

// ends are a range's lowest and highest value, nil for none, and its step.
func (r NumberRange) ends() (low, high *decimal.Decimal, step decimal.Decimal, err error) {
	if r.Min != nil {
		n, err := exactNumber(*r.Min)
		if err != nil {
			return nil, nil, step, err
		}
		low = &n
	}
	if r.Max != nil {
		n, err := exactNumber(*r.Max)
		if err != nil {
			return nil, nil, step, err
		}
		high = &n
	}
	step = decimal.NewFromInt(1)
	if r.Step != "" {
		if step, err = exactNumber(r.Step); err != nil {
			return nil, nil, step, err
		}
	}
	return low, high, step, nil
}

func within(n decimal.Decimal, low, high *decimal.Decimal) decimal.Decimal {
	if high != nil && n.GreaterThan(*high) {
		n = *high
	}
	if low != nil && n.LessThan(*low) {
		n = *low
	}
	return n
}

// RangeKeyed is the value after key: a step up or down, ten, or an end
// (home, end), within the range. No value counts as the lowest, or 0.
func RangeKeyed(value *string, r NumberRange, key string) (string, error) {
	low, high, step, err := r.ends()
	if err != nil {
		return "", err
	}
	now := decimal.Zero
	if low != nil {
		now = *low
	}
	if value != nil && strings.TrimSpace(*value) != "" {
		if now, err = exactNumber(*value); err != nil {
			return "", err
		}
	}
	next := now
	if steps, ok := keySteps[key]; ok {
		next = now.Add(step.Mul(decimal.NewFromInt(steps)))
	} else if key == "home" {
		if low != nil {
			next = *low
		}
	} else if key == "end" {
		if high != nil {
			next = *high
		}
	} else {
		return "", &InvalidError{fmt.Sprintf("%q is no key of a range", key)}
	}
	return within(next, low, high).String(), nil
}

// RangeDragged is the value a slider takes where it is dragged to: the
// nearest step from the lowest value (half a step up), at most the highest
// step in the range.
func RangeDragged(where string, r NumberRange) (string, error) {
	low, high, step, err := r.ends()
	if err != nil {
		return "", err
	}
	if !step.IsPositive() {
		return "", &InvalidError{fmt.Sprintf("a slider's step is more than 0, not %s", step)}
	}
	at, err := exactNumber(where)
	if err != nil {
		return "", err
	}
	from := decimal.Zero
	if low != nil {
		from = *low
	}
	steps := decimal.Zero
	if offset := at.Sub(from); offset.IsPositive() {
		q, rest := offset.QuoRem(step, 0)
		if rest.Mul(decimal.NewFromInt(2)).GreaterThanOrEqual(step) {
			q = q.Add(decimal.NewFromInt(1))
		}
		steps = q
	}
	if high != nil {
		most, _ := high.Sub(from).QuoRem(step, 0)
		if steps.GreaterThan(most) {
			steps = most
		}
		if steps.IsNegative() {
			steps = decimal.Zero
		}
	}
	return from.Add(steps.Mul(step)).String(), nil
}

// RangeTyped is a typed number, kept within the range.
func RangeTyped(value string, r NumberRange) (string, error) {
	r.Step = ""
	low, high, _, err := r.ends()
	if err != nil {
		return "", err
	}
	n, err := exactNumber(value)
	if err != nil {
		return "", err
	}
	return within(n, low, high).String(), nil
}
