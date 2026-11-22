package clarity

// Whole numbers in code as every stack works them out (conformance/overflow.json):
// 64 bits, and a result that does not fit is a value error, never a wrapped number.

import "math"

func tooLarge() error {
	return saidValue("a number too large", KeyedText("numberTooLarge", "The number is too large"))
}

// WholeAdd is `a + b` of whole numbers in code.
func WholeAdd(a, b int64) (int64, error) {
	sum := a + b
	if (b > 0 && sum < a) || (b < 0 && sum > a) {
		return 0, tooLarge()
	}
	return sum, nil
}

// WholeSub is `a - b` of whole numbers in code.
func WholeSub(a, b int64) (int64, error) {
	difference := a - b
	if (b > 0 && difference > a) || (b < 0 && difference < a) {
		return 0, tooLarge()
	}
	return difference, nil
}

// WholeMul is `a * b` of whole numbers in code.
func WholeMul(a, b int64) (int64, error) {
	if a == 0 || b == 0 {
		return 0, nil
	}
	product := a * b
	if (a == -1 && b == math.MinInt64) || (b == -1 && a == math.MinInt64) || product/b != a {
		return 0, tooLarge()
	}
	return product, nil
}

// WholeNeg is `-a` of a whole number in code.
func WholeNeg(a int64) (int64, error) {
	if a == math.MinInt64 {
		return 0, tooLarge()
	}
	return -a, nil
}

// WholeDiv is `a div b` in code: the quotient of two whole numbers,
// rounded toward zero; dividing by zero is an error.
func WholeDiv(a, b int64) (int64, error) {
	if b == 0 {
		return 0, divisionByZero()
	}
	if b == -1 {
		return WholeNeg(a)
	}
	return a / b, nil
}

// WholeMod is `a mod b` in code: what is left of `a div b`, with the sign of a.
func WholeMod(a, b int64) (int64, error) {
	if b == 0 {
		return 0, divisionByZero()
	}
	if b == -1 {
		return 0, nil
	}
	return a % b, nil
}
