// What goes wrong in a generated application, as errors code can tell
// apart: a contract that does not hold, a record that is not there or is
// still referred to, a value that does not fit, the database itself, and a
// refusal in the application's own words.

package clarity

import "fmt"

// ContractError is a require or ensure of a function or action that did not hold.
type ContractError struct{ What string }

func (e *ContractError) Error() string { return "condition failed: " + e.What }

// NotFoundError is a record that should exist and does not; ID is nil for
// a reference that is not set.
type NotFoundError struct {
	Table string
	ID    *int64
}

func (e *NotFoundError) Error() string {
	if e.ID == nil {
		return fmt.Sprintf("no %s is set", e.Table)
	}
	return fmt.Sprintf("there is no %s with id %d", e.Table, *e.ID)
}

// InUseError is a record another record still refers to, so it cannot go.
type InUseError struct {
	Table string
	ID    int64
}

func (e *InUseError) Error() string {
	return fmt.Sprintf("the %s with id %d is still referred to", e.Table, e.ID)
}

// InvalidError is a value that does not fit its field.
type InvalidError struct{ What string }

func (e *InvalidError) Error() string { return "invalid value: " + e.What }

// ValueError is a value in code that does not fit where it is used: a
// position outside its list, a highest number below 1 (conformance/lists.json).
type ValueError struct{ What string }

func (e *ValueError) Error() string { return e.What }

// StorageError is the database failing.
type StorageError struct {
	What string
	Err  error
}

func (e *StorageError) Error() string { return "storage: " + e.What }

func (e *StorageError) Unwrap() error { return e.Err }

// RefusedError is a refusal: a data event or code that will not go on, in
// the application's own words, which the screen shows.
type RefusedError struct{ Text Text }

func (e *RefusedError) Error() string { return e.Text.Show() }

// Refuse is Clarity's refuse.
func Refuse(text Text) error { return &RefusedError{text} }

// Require checks a require line of a function or action.
func Require(holds bool, what string) error {
	if holds {
		return nil
	}
	return &ContractError{what}
}

// Ensure checks an ensure line of a function or action.
func Ensure(holds bool, what string) error { return Require(holds, what) }

// referenceFailure is a statement that breaks a reference between records.
type referenceFailure struct{ err error }

func (e *referenceFailure) Error() string { return e.err.Error() }
