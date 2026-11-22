// What goes wrong in a generated application, as errors code can tell
// apart: a contract that does not hold, a record that is not there or is
// still referred to, a value that does not fit, the database itself, and a
// refusal in the application's own words.

package clarity

import (
	"errors"
	"fmt"
)

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
// Text is what a window says of it in Clarity's own words, when it has them.
type ValueError struct {
	What string
	Text *Text
}

// saidValue is a ValueError with Clarity's own words for it.
func saidValue(what string, text Text) error { return &ValueError{What: what, Text: &text} }

// divisionByZero is a / 0, a div 0 or a mod 0 (conformance/decimal-division.json).
func divisionByZero() error {
	return saidValue("division by zero", KeyedText("divisionByZero", "Division by zero"))
}

// Said is what a window shows when code fails with err: a refusal its own
// text; a value or a record that is not there, or still used, in Clarity's
// own words by a literal key (translated where the project has them). False
// for any other error, which a window shows by its message. The message
// (Error) stays the English of the shared cases.
func Said(err error) (Text, bool) {
	var refused *RefusedError
	var value *ValueError
	var missing *NotFoundError
	var used *InUseError
	switch {
	case errors.As(err, &refused):
		return refused.Text, true
	case errors.As(err, &value) && value.Text != nil:
		return *value.Text, true
	case errors.As(err, &missing) && missing.ID != nil:
		return KeyedText("recordNotThere", "There is no {table} with id {id}").With("table", SlotText(missing.Table)).With("id", SlotInteger(*missing.ID, "")), true
	case errors.As(err, &used):
		return KeyedText("recordStillUsed", "This {table} is still used by other records").With("table", SlotText(used.Table)), true
	}
	return Text{}, false
}

func (e *ValueError) Error() string { return e.What }

// StepsError is a database whose migration steps are not the ones the app was
// made with (docs/ledger/migrations.md): the app stops with Line, in English.
type StepsError struct{ Line string }

func (e *StepsError) Error() string { return e.Line }

// StepsLine is the line an app stops with when err is a StepsError: a database
// whose migration steps are not the app's (docs/ledger/migrations.md).
func StepsLine(err error) (string, bool) {
	var steps *StepsError
	if errors.As(err, &steps) {
		return steps.Line, true
	}
	return "", false
}

// StorageError is the database failing.
type StorageError struct {
	What string
	Err  error
}

func (e *StorageError) Error() string { return "storage: " + e.What }

func (e *StorageError) Unwrap() error { return e.Err }

// NotKeptError is a database that connects with a secret setting (DESIGN
// 5.27) the person has not kept yet.
type NotKeptError struct{ Setting string }

func (e *NotKeptError) Error() string {
	return "storage: the secret setting " + e.Setting + " is not kept yet: the settings name the connection"
}

// UnconnectedReason is why the databases did not open, as the person reads
// it (manual/data.settings.md, conformance/unconnected.json): in Clarity's own words,
// which the project translates, with this runtime's message in the slot.
func UnconnectedReason(err error) Text {
	var notKept *NotKeptError
	if errors.As(err, &notKept) {
		return KeyText("unconnectedNotKept")
	}
	problem := err.Error()
	var storage *StorageError
	if errors.As(err, &storage) {
		problem = storage.What
	}
	return KeyText("unconnectedFailed").With("problem", SlotText(problem))
}

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
