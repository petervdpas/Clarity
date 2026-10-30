// round(value, places), sin and cos in code (conformance/rounding.json and drawing.json).

package clarity

import (
	"math"

	"github.com/shopspring/decimal"
)

// Round gives value with at most places decimals, rounded half away from
// zero; a value with fewer stays as it is, and places below 0 count as 0.
func Round(value decimal.Decimal, places int64) decimal.Decimal {
	places = max(0, min(places, 28))
	if -int64(value.Exponent()) <= places {
		return value
	}
	return value.Round(int32(places))
}

// Snapped is snapped(value, step): the multiple of step nearest to value, a
// half rounding up; a step of 0 gives the value (conformance/drawing.json, grids).
func Snapped(value, step int64) int64 {
	if step <= 0 {
		return value
	}
	n := 2*value + step
	q := n / (2 * step)
	if n%(2*step) != 0 && n < 0 {
		q--
	}
	return q * step
}

// Sin is sin(degrees) in code, rounded to 10 decimals (conformance/drawing.json).
func Sin(degrees decimal.Decimal) decimal.Decimal {
	return angle(degrees, math.Sin)
}

// Cos is cos(degrees) in code, rounded to 10 decimals (conformance/drawing.json).
func Cos(degrees decimal.Decimal) decimal.Decimal {
	return angle(degrees, math.Cos)
}

func angle(degrees decimal.Decimal, of func(float64) float64) decimal.Decimal {
	radians := degrees.InexactFloat64() * math.Pi / 180
	// Rounded, so every runtime gives the same.
	return Round(decimal.NewFromFloat(of(radians)), 10)
}

// Units is a value of code as a number a canvas draws with.
func Units(value decimal.Decimal) float64 {
	return value.InexactFloat64()
}
