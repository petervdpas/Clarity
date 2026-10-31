package clarity

// Reordering (DESIGN 5.30), decided here in the runtime, not in the screen
// (conformance/reorder.json): a row of a list, table, tree or tree table
// moved by a drag or by Alt and an arrow key. The result is the list's new
// order, each row's parent (in a tree) and each row's number at its level,
// which the part writes into the field of `reorder by`. A move that changes
// no parent or number is refused.

import (
	"encoding/json"
	"reflect"
	"slices"
)

// TreeRow is a row of a list or tree: its id and, in a tree, its parent's id or nil.
type TreeRow struct {
	ID     any
	Parent any
}

// Move is a drag onto a row (Where is before, after or into, Target the
// row's place) or a key on the moved row (up, down, left, right); places
// count from 0 in the list.
type Move struct {
	Row    int    `json:"row"`
	Where  string `json:"where,omitempty"`
	Target int    `json:"target"`
	Key    string `json:"key,omitempty"`
}

// Reordered is the list after a move: the old places in the new order, and
// each row's parent and number in that order.
type Reordered struct {
	Order   []int
	Parents []any
	Numbers []int64
}

// idKey is an id as JSON writes it, so 2 and 2.0 are one id.
func idKey(id any) string {
	text, err := json.Marshal(id)
	if err != nil {
		return ""
	}
	return string(text)
}

func sameID(a, b any) bool { return reflect.DeepEqual(asJSON(a), asJSON(b)) }

// parentsOf is the place of each row's parent, -1 for none: a parent that is
// not in the list is none, and a loop breaks at its row that comes first.
func parentsOf(rows []TreeRow) []int {
	at := map[string]int{}
	for i, r := range rows {
		if _, seen := at[idKey(r.ID)]; !seen {
			at[idKey(r.ID)] = i
		}
	}
	parent := make([]int, len(rows))
	for i, r := range rows {
		parent[i] = -1
		if r.Parent != nil {
			if p, ok := at[idKey(r.Parent)]; ok {
				parent[i] = p
			}
		}
	}
	for i := range rows {
		var seen []int
		p := i
		for p != -1 && !slices.Contains(seen, p) {
			seen = append(seen, p)
			p = parent[p]
		}
		if p != -1 {
			loop := seen[slices.Index(seen, p):]
			parent[slices.Min(loop)] = -1
		}
	}
	return parent
}

// numbered is each row's place at its level counted from 1, in the order of the list.
func numbered(levels []int, order []int) []int64 {
	counts := map[int]int64{}
	numbers := make([]int64, len(order))
	for i, place := range order {
		counts[levels[place]]++
		numbers[i] = counts[levels[place]]
	}
	return numbers
}

// Reorder is the list after move, false when the move changes nothing. In a
// list that is no tree, rows have no parent.
func Reorder(rows []TreeRow, move Move, tree bool) (Reordered, bool) {
	r := move.Row
	if r < 0 || r >= len(rows) {
		return Reordered{}, false
	}
	asTree := make([]TreeRow, len(rows))
	for i, row := range rows {
		asTree[i] = TreeRow{ID: row.ID}
		if tree {
			asTree[i].Parent = row.Parent
		}
	}
	levels := parentsOf(asTree)
	level := func(place int) []int {
		var at []int
		for i, l := range levels {
			if l == place {
				at = append(at, i)
			}
		}
		return at
	}
	under := func(place, row int) bool {
		for p := place; p != -1; p = levels[p] {
			if p == row {
				return true
			}
		}
		return false
	}
	// The last of a row and the rows right under it, leaving out the moved row.
	lastOf := func(place int) int {
		last := place
		for _, i := range level(place) {
			if i != r && i > last {
				last = i
			}
		}
		return last
	}
	parent := asTree[r].Parent
	var anchor int
	var after bool
	if move.Key != "" {
		siblings := level(levels[r])
		k := slices.Index(siblings, r)
		switch {
		case move.Key == "up" || move.Key == "down":
			next := k + 1
			if move.Key == "up" {
				next = k - 1
			}
			if next < 0 || next >= len(siblings) {
				return Reordered{}, false
			}
			anchor, after = siblings[next], move.Key == "down"
		case !tree:
			return Reordered{}, false
		case move.Key == "left":
			if levels[r] == -1 {
				return Reordered{}, false
			}
			anchor, after, parent = levels[r], true, asTree[levels[r]].Parent
		case move.Key == "right":
			if k <= 0 {
				return Reordered{}, false
			}
			before := siblings[k-1]
			anchor, after, parent = lastOf(before), true, asTree[before].ID
		default:
			return Reordered{}, false
		}
	} else {
		t := move.Target
		if t < 0 || t >= len(rows) || t == r || (tree && under(t, r)) {
			return Reordered{}, false
		}
		switch move.Where {
		case "into":
			if !tree {
				return Reordered{}, false
			}
			anchor, after, parent = lastOf(t), true, asTree[t].ID
		case "before", "after":
			anchor, after, parent = t, move.Where == "after", nil
			if tree {
				parent = asTree[t].Parent
			}
		default:
			return Reordered{}, false
		}
	}
	order := make([]int, 0, len(rows))
	for i := range asTree {
		if i != r {
			order = append(order, i)
		}
	}
	at := slices.Index(order, anchor)
	if after {
		at++
	}
	order = slices.Insert(order, at, r)
	moved := slices.Clone(asTree)
	moved[r].Parent = parent
	inOrder := make([]TreeRow, len(order))
	for i, place := range order {
		inOrder[i] = moved[place]
	}
	newLevels := parentsOf(inOrder)
	// Levels by old place, so both lists are numbered alike.
	byPlace := make([]int, len(rows))
	for i, place := range order {
		byPlace[place] = -1
		if newLevels[i] != -1 {
			byPlace[place] = order[newLevels[i]]
		}
	}
	numbers := numbered(byPlace, order)
	identity := make([]int, len(rows))
	for i := range identity {
		identity[i] = i
	}
	before := numbered(levels, identity)
	same := true
	for i, place := range order {
		if numbers[i] != before[place] || !sameID(moved[place].Parent, asTree[place].Parent) {
			same = false
			break
		}
	}
	if same {
		return Reordered{}, false
	}
	parents := make([]any, len(order))
	for i, place := range order {
		parents[i] = moved[place].Parent
	}
	return Reordered{Order: order, Parents: parents, Numbers: numbers}, true
}
