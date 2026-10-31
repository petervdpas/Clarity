package clarity

// Paths into a window's data, as the screen names a value it acts on: names
// joined by dots, `album.tags` the field tags of the record album,
// `readings` a plain value, and a number a row of a list (`albums.0.title`).

import (
	"fmt"
	"strconv"
	"strings"
)

// ValueAt is the value at path in data; false when there is none.
func ValueAt(data map[string]any, path string) (any, bool) {
	var at any = data
	for _, name := range strings.Split(path, ".") {
		switch here := at.(type) {
		case map[string]any:
			value, ok := here[name]
			if !ok {
				return nil, false
			}
			at = value
		case []any:
			i, err := strconv.Atoi(name)
			if err != nil || i < 0 || i >= len(here) {
				return nil, false
			}
			at = here[i]
		default:
			return nil, false
		}
	}
	return at, true
}

// SetValueAt puts value at path in data; the records and lists on the way must be there.
func SetValueAt(data map[string]any, path string, value any) error {
	names := strings.Split(path, ".")
	var at any = data
	for i, name := range names {
		last := i == len(names)-1
		switch here := at.(type) {
		case map[string]any:
			if last {
				here[name] = value
				return nil
			}
			at = here[name]
		case []any:
			n, err := strconv.Atoi(name)
			if err != nil || n < 0 || n >= len(here) {
				return &InvalidError{fmt.Sprintf("the window has no %s", path)}
			}
			if last {
				here[n] = value
				return nil
			}
			at = here[n]
		default:
			return &InvalidError{fmt.Sprintf("the window has no %s", path)}
		}
	}
	return &InvalidError{fmt.Sprintf("the window has no %s", path)}
}
