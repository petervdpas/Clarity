package clarity

import (
	"errors"
	"testing"
)

// What a window says when code fails at a value or a record: Clarity's own
// words by a literal key (translated where the project has them), while the
// error's message stays the English of the shared cases.

func TestAnItemOutsideItsListIsSaidByItsKey(t *testing.T) {
	_, err := At([]int64{1}, 10)
	if err == nil || err.Error() != "there is no item 10 in a list of 1" {
		t.Fatalf("message %v", err)
	}
	said, ok := Said(err)
	if !ok || said.Key != "itemOutsideList" || said.Show() != "There is no item 10 in a list of 1" {
		t.Fatalf("said %#v %q", said, said.Show())
	}
}

func TestARecordNotThereOrInUseIsSaidByItsKey(t *testing.T) {
	id := int64(3)
	missing, ok := Said(&NotFoundError{Table: "Customer", ID: &id})
	if !ok || missing.Key != "recordNotThere" || missing.Show() != "There is no Customer with id 3" {
		t.Fatalf("missing %#v %q", missing, missing.Show())
	}
	used, ok := Said(&InUseError{Table: "Customer", ID: 3})
	if !ok || used.Key != "recordStillUsed" || used.Show() != "This Customer is still used by other records" {
		t.Fatalf("used %#v %q", used, used.Show())
	}
	if (&NotFoundError{Table: "Customer", ID: &id}).Error() != "there is no Customer with id 3" {
		t.Fatal("the message stays English")
	}
}

func TestDivisionByZeroAndANumberTooLargeAreSaidByTheirKeys(t *testing.T) {
	_, err := WholeDiv(1, 0)
	said, ok := Said(err)
	if !ok || said.Key != "divisionByZero" || said.Show() != "Division by zero" || err.Error() != "division by zero" {
		t.Fatalf("division %v %#v", err, said)
	}
	_, err = WholeAdd(9223372036854775807, 1)
	said, ok = Said(err)
	if !ok || said.Key != "numberTooLarge" || said.Show() != "The number is too large" || err.Error() != "a number too large" {
		t.Fatalf("too large %v %#v", err, said)
	}
}

func TestARefusalIsSaidInItsOwnWordsAndOtherErrorsAreNot(t *testing.T) {
	said, ok := Said(Refuse(KeyedText("tooLate", "Too late")))
	if !ok || said.Key != "tooLate" {
		t.Fatalf("refused %#v", said)
	}
	if _, ok := Said(errors.New("the disk is full")); ok {
		t.Fatal("an error of its own has no words of Clarity")
	}
}
