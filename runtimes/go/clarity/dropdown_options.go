package clarity

// The dropdown's options of U1c, decided here in the runtime
// (conformance/dropdown-options.json). A typed dropdown offers the values
// its field already has: each once, without empty text, those containing
// the typed text (ignoring case), sorted ignoring case and then as written.
// A search dropdown offers the records whose text or shown fields hold every
// typed word, ignoring case, in the order they came.

import (
	"fmt"
	"slices"
	"strings"
)

// Suggestions are what a typed dropdown offers of values for typed.
func Suggestions(values []string, typed string) []string {
	wanted := strings.ToLower(strings.TrimSpace(typed))
	offered := []string{}
	seen := map[string]bool{}
	for _, v := range values {
		if v == "" || seen[v] {
			continue
		}
		seen[v] = true
		if strings.Contains(strings.ToLower(v), wanted) {
			offered = append(offered, v)
		}
	}
	slices.SortStableFunc(offered, func(a, b string) int {
		if c := strings.Compare(strings.ToLower(a), strings.ToLower(b)); c != 0 {
			return c
		}
		return strings.Compare(a, b)
	})
	return offered
}

// Searched are the records a search dropdown offers of options for typed,
// searching their text and the fields of show (every field when show is empty).
func Searched(options []RefOption, typed string, show []string) []RefOption {
	words := strings.Fields(strings.ToLower(typed))
	if len(words) == 0 && options != nil {
		return options
	}
	offered := []RefOption{}
	for _, o := range options {
		texts := []string{strings.ToLower(o.Text)}
		add := func(v any) {
			if v != nil {
				texts = append(texts, strings.ToLower(fmt.Sprint(v)))
			}
		}
		if len(show) == 0 {
			for _, v := range o.Fields {
				add(v)
			}
		} else {
			for _, f := range show {
				add(o.Fields[f])
			}
		}
		holds := true
		for _, w := range words {
			if !slices.ContainsFunc(texts, func(t string) bool { return strings.Contains(t, w) }) {
				holds = false
				break
			}
		}
		if holds {
			offered = append(offered, o)
		}
	}
	return offered
}
