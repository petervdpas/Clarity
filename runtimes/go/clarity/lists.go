// Lists of values in code (list<integer>), as every runtime gives them
// (conformance/lists.json).

package clarity

import (
	"fmt"
	"strings"
)

// At is xs[i] in code: item i of a list, counted from 1.
func At[T any](list []T, position int64) (T, error) {
	if position < 1 || position > int64(len(list)) {
		var zero T
		return zero, outside(position, len(list))
	}
	return list[position-1], nil
}

// SetAt is xs[i] = value in code: sets item i of a list, counted from 1.
func SetAt[T any](list []T, position int64, value T) error {
	if position < 1 || position > int64(len(list)) {
		return outside(position, len(list))
	}
	list[position-1] = value
	return nil
}

// WholeDiv is `a div b` in code: the quotient of two whole numbers,
// rounded toward zero; dividing by zero is an error.
func WholeDiv(a, b int64) (int64, error) {
	if b == 0 {
		return 0, &ValueError{What: "division by zero"}
	}
	if b == -1 {
		return -a, nil
	}
	return a / b, nil
}

// WholeMod is `a mod b` in code: what is left of `a div b`, with the sign of a.
func WholeMod(a, b int64) (int64, error) {
	if b == 0 {
		return 0, &ValueError{What: "division by zero"}
	}
	if b == -1 {
		return 0, nil
	}
	return a % b, nil
}

func outside(position int64, count int) error {
	return &ValueError{What: fmt.Sprintf("there is no item %d in a list of %d", position, count)}
}

// Range is `from to to` in code: the whole numbers from from to to, both
// counted; none when to is below from (conformance/lists.json, `ranges`).
func Range(from, to int64) []int64 {
	out := []int64{}
	for i := from; i <= to; i++ {
		out = append(out, i)
	}
	return out
}

// Join is `join(texts, separator)` in code: the texts in order with the
// separator between each two (conformance/lists.json, `joins`).
func Join(texts []string, separator string) string {
	return strings.Join(texts, separator)
}
