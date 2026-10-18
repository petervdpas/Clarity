// The values records hold beside Go's own: a calendar date and a date with a
// time of day, without a time zone, as Clarity's date and datetime. They
// read and write JSON as the renderer and the other stacks do: 2026-10-17
// and 2026-10-17T09:30:00.

package clarity

import (
	"encoding/json"
	"fmt"
	"time"
)

const (
	dateLayout     = "2006-01-02"
	dateTimeLayout = "2006-01-02T15:04:05"
)

// Date is a calendar date.
type Date struct {
	Year  int
	Month time.Month
	Day   int
}

// DateTime is a date with a time of day, to the second.
type DateTime struct {
	Date
	Hour, Minute, Second int
}

func ParseDate(text string) (Date, error) {
	t, err := time.Parse(dateLayout, text)
	if err != nil {
		return Date{}, err
	}
	return DateOf(t), nil
}

func ParseDateTime(text string) (DateTime, error) {
	for _, layout := range []string{dateTimeLayout, "2006-01-02 15:04:05", "2006-01-02T15:04", "2006-01-02 15:04"} {
		if t, err := time.Parse(layout, text); err == nil {
			return DateTimeOf(t), nil
		}
	}
	return DateTime{}, fmt.Errorf("%q is no date and time", text)
}

// DateOf is the calendar date of t.
func DateOf(t time.Time) Date { return Date{t.Year(), t.Month(), t.Day()} }

// DateTimeOf is t to the second, without its time zone.
func DateTimeOf(t time.Time) DateTime {
	return DateTime{DateOf(t), t.Hour(), t.Minute(), t.Second()}
}

func (d Date) String() string { return fmt.Sprintf("%04d-%02d-%02d", d.Year, d.Month, d.Day) }

func (d DateTime) String() string {
	return fmt.Sprintf("%sT%02d:%02d:%02d", d.Date, d.Hour, d.Minute, d.Second)
}

// Time is the date at midnight, in UTC, for drivers that take a time.
func (d Date) Time() time.Time { return time.Date(d.Year, d.Month, d.Day, 0, 0, 0, 0, time.UTC) }

func (d DateTime) Time() time.Time {
	return time.Date(d.Year, d.Month, d.Day, d.Hour, d.Minute, d.Second, 0, time.UTC)
}

// Compare is -1, 0 or 1 as d is before, the same as or after other.
func (d Date) Compare(other Date) int { return d.Time().Compare(other.Time()) }

func (d DateTime) Compare(other DateTime) int { return d.Time().Compare(other.Time()) }

func (d Date) MarshalJSON() ([]byte, error) { return json.Marshal(d.String()) }

func (d *Date) UnmarshalJSON(data []byte) error {
	var text string
	if err := json.Unmarshal(data, &text); err != nil {
		return err
	}
	parsed, err := ParseDate(text)
	*d = parsed
	return err
}

func (d DateTime) MarshalJSON() ([]byte, error) { return json.Marshal(d.String()) }

func (d *DateTime) UnmarshalJSON(data []byte) error {
	var text string
	if err := json.Unmarshal(data, &text); err != nil {
		return err
	}
	parsed, err := ParseDateTime(text)
	*d = parsed
	return err
}

// Today is the date where the application runs: Clarity's today().
func Today() *Date {
	d := DateOf(time.Now())
	return &d
}

// Now is the date and time where the application runs, to the second: Clarity's now().
func Now() *DateTime {
	d := DateTimeOf(time.Now())
	return &d
}

// CompareDates orders two dates of fields: no date comes before any date.
func CompareDates(a, b *Date) int {
	switch {
	case a == nil && b == nil:
		return 0
	case a == nil:
		return -1
	case b == nil:
		return 1
	}
	return a.Compare(*b)
}

// CompareDateTimes orders two dates and times of fields: none comes first.
func CompareDateTimes(a, b *DateTime) int {
	switch {
	case a == nil && b == nil:
		return 0
	case a == nil:
		return -1
	case b == nil:
		return 1
	}
	return a.Compare(*b)
}
