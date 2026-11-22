package clarity

// Decimals in code as every stack works them out, as Rust and C# keep them: up
// to 28 places with a whole number of units that fits in 96 bits. A sum,
// difference or product with more places than fit is rounded to the places
// that do, a tie to the even digit; one that does not fit with no places is a
// value error (conformance/overflow.json). A quotient is worked out a digit at
// a time, its last digit rounded; an exact one keeps only the places it needs,
// and dividing by zero is a value error (conformance/decimal-division.json).

import (
	"math/big"

	"github.com/shopspring/decimal"
)

// The most places a quotient keeps, and the largest number of units it holds (2^96 - 1).
const divisionPlaces = 28

var (
	mostUnits = new(big.Int).Sub(new(big.Int).Lsh(big.NewInt(1), 96), big.NewInt(1))
	bigTen    = big.NewInt(10)
)

// DecimalAdd is `a + b` of decimals in code.
func DecimalAdd(a, b decimal.Decimal) (decimal.Decimal, error) { return fitted(a.Add(b)) }

// DecimalSub is `a - b` of decimals in code.
func DecimalSub(a, b decimal.Decimal) (decimal.Decimal, error) { return fitted(a.Sub(b)) }

// DecimalMul is `a * b` of decimals in code.
func DecimalMul(a, b decimal.Decimal) (decimal.Decimal, error) { return fitted(a.Mul(b)) }

// fitted is an exact result as a decimal keeps it: rounded to at most 28
// places, and to fewer while its units do not fit in 96 bits.
func fitted(d decimal.Decimal) (decimal.Decimal, error) {
	units := new(big.Int).Abs(d.Coefficient())
	places := -int(d.Exponent())
	if places < 0 {
		units.Mul(units, new(big.Int).Exp(bigTen, big.NewInt(int64(-places)), nil))
		places = 0
	}
	if places > divisionPlaces {
		units = roundedOff(units, places-divisionPlaces)
		places = divisionPlaces
	}
	for units.Cmp(mostUnits) > 0 && places > 0 {
		units = roundedOff(units, 1)
		places--
	}
	if units.Cmp(mostUnits) > 0 {
		return decimal.Zero, tooLarge()
	}
	if d.Sign() < 0 {
		units.Neg(units)
	}
	return decimal.NewFromBigInt(units, -int32(places)), nil
}

// roundedOff is units without its last digits, rounded to the nearest, a tie to the even one.
func roundedOff(units *big.Int, digits int) *big.Int {
	unit := new(big.Int).Exp(bigTen, big.NewInt(int64(digits)), nil)
	kept, rest := new(big.Int).QuoRem(units, unit, new(big.Int))
	switch rest.Lsh(rest, 1).Cmp(unit) {
	case 1:
		kept.Add(kept, big.NewInt(1))
	case 0:
		if kept.Bit(0) == 1 {
			kept.Add(kept, big.NewInt(1))
		}
	}
	return kept
}

// DecimalDiv is `a / b` of decimals in code.
func DecimalDiv(a, b decimal.Decimal) (decimal.Decimal, error) {
	if b.IsZero() {
		return decimal.Zero, divisionByZero()
	}
	negative := a.Sign()*b.Sign() < 0
	top := new(big.Int).Abs(a.Coefficient())
	bottom := new(big.Int).Abs(b.Coefficient())
	// a / b = (top / bottom) * 10^shift; the quotient's places are its digits after the point less shift.
	shift := int(a.Exponent()) - int(b.Exponent())
	quotient, rest := new(big.Int).QuoRem(top, bottom, new(big.Int))
	if quotient.Cmp(mostUnits) > 0 && shift >= 0 {
		return decimal.Zero, tooLarge()
	}
	digits := 0
	grows := func() bool {
		next := new(big.Int).Mul(quotient, bigTen)
		return next.Add(next, big.NewInt(9)).Cmp(mostUnits) <= 0
	}
	for rest.Sign() != 0 && digits-shift < divisionPlaces && grows() {
		rest.Mul(rest, bigTen)
		digit := new(big.Int)
		digit.QuoRem(rest, bottom, rest)
		quotient.Mul(quotient, bigTen).Add(quotient, digit)
		digits++
	}
	exact := rest.Sign() == 0
	if !exact {
		// The last digit rounded to the nearest, a tie to the even one.
		rest.Mul(rest, bigTen)
		digit, left := new(big.Int).QuoRem(rest, bottom, new(big.Int))
		d := digit.Int64()
		if d > 5 || (d == 5 && (left.Sign() != 0 || quotient.Bit(0) == 1)) {
			quotient.Add(quotient, big.NewInt(1))
		}
		if quotient.Cmp(mostUnits) > 0 {
			quotient.Quo(quotient, bigTen)
			digits--
		}
	}
	places := digits - shift
	if places < 0 {
		quotient.Mul(quotient, new(big.Int).Exp(bigTen, big.NewInt(int64(-places)), nil))
		places = 0
	}
	if exact {
		// An exact quotient keeps only the places it needs: 6.0 / 3 is 2.
		zero := new(big.Int)
		for places > 0 {
			q, r := new(big.Int).QuoRem(quotient, bigTen, new(big.Int))
			if r.Cmp(zero) != 0 {
				break
			}
			quotient = q
			places--
		}
	}
	if negative {
		quotient.Neg(quotient)
	}
	return decimal.NewFromBigInt(quotient, -int32(places)), nil
}
