// Lists of values in code (list<integer>) and random, as every runtime gives
// them (conformance/lists.json).

package clarity

import (
	"fmt"
	"math/rand/v2"
	"sync"
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

func outside(position int64, count int) error {
	return &ValueError{What: fmt.Sprintf("there is no item %d in a list of %d", position, count)}
}

// Splitmix gives the numbers random(max) gives: splitmix64, the sequence
// every runtime gives alike from the same seed.
type Splitmix struct{ state uint64 }

// NewSplitmix starts the sequence from seed.
func NewSplitmix(seed uint64) *Splitmix { return &Splitmix{state: seed} }

// Next is a whole number from 1 to max.
func (s *Splitmix) Next(max int64) (int64, error) {
	if max < 1 {
		return 0, &ValueError{What: fmt.Sprintf("random takes a highest number of 1 or more, not %d", max)}
	}
	s.state += 0x9E3779B97F4A7C15
	z := s.state
	z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
	z = (z ^ (z >> 27)) * 0x94D049BB133111EB
	z ^= z >> 31
	return 1 + int64(z%uint64(max)), nil
}

var (
	shared     = NewSplitmix(rand.Uint64())
	sharedLock sync.Mutex
)

// Random is random(max) in an engine written in Clarity, from a sequence
// seeded once per run of the application.
func Random(max int64) (int64, error) {
	sharedLock.Lock()
	defer sharedLock.Unlock()
	return shared.Next(max)
}
