// Lists of values in code (list<integer>), as every runtime gives them
// (conformance/lists.json).

package clarity

import "fmt"

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

func outside(position int64, count int) error {
	return &ValueError{What: fmt.Sprintf("there is no item %d in a list of %d", position, count)}
}
