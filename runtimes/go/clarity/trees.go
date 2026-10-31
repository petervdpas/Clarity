package clarity

// Trees of U1c, decided here in the runtime (conformance/trees.json): the
// rows of a list arranged by their parent, a row under its parent when that
// is in the list and at the top otherwise, each level in the order of the
// list, a closed row hiding what is under it.

import "encoding/json"

// TreeShown is a row a tree shows: its place in the list, its depth, and
// whether rows are under it; JSON writes it as [place, depth, children].
type TreeShown struct {
	Place, Depth int
	Children     bool
}

func (s TreeShown) MarshalJSON() ([]byte, error) {
	return json.Marshal([]any{s.Place, s.Depth, s.Children})
}

// ShownTree is the rows shown, top to bottom, with open the ids of the rows shown open.
func ShownTree(rows []TreeRow, open []any) []TreeShown {
	opened := map[string]bool{}
	for _, id := range open {
		opened[idKey(id)] = true
	}
	parent := parentsOf(rows)
	under := make([][]int, len(rows))
	var tops []int
	for i, p := range parent {
		if p == -1 {
			tops = append(tops, i)
		} else {
			under[p] = append(under[p], i)
		}
	}
	shown := []TreeShown{}
	var walk func(place, depth int)
	walk = func(place, depth int) {
		shown = append(shown, TreeShown{place, depth, len(under[place]) > 0})
		if opened[idKey(rows[place].ID)] {
			for _, child := range under[place] {
				walk(child, depth+1)
			}
		}
	}
	for _, top := range tops {
		walk(top, 0)
	}
	return shown
}

// Above is the ids of the rows above the row at place at, top first: what a
// tree opens when that row is chosen.
func Above(rows []TreeRow, at int) []any {
	ids := []any{}
	if at < 0 || at >= len(rows) {
		return ids
	}
	parent := parentsOf(rows)
	for p := parent[at]; p != -1; p = parent[p] {
		ids = append([]any{rows[p].ID}, ids...)
	}
	return ids
}
