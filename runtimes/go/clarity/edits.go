package clarity

// The edit bus (DESIGN 5.29): per open window, the record of the person's
// edits, kept here in the runtime, not in the screen. Each edit is a message
// with the part, its scope (its form or panel, or the part itself), what it
// changed (each name of the window's data, or #parts.<name> for a part's own
// text or tick, with its value before and after) and the database steps its
// handler made. The screen sends an edit when the person ends one, and Ctrl+Z
// or Ctrl+Y as an undo; the runtime takes the steps back or does them again
// and answers with the window's state as it was. Every runtime keeps the
// record alike (conformance/edit-bus.json).

import (
	"encoding/json"
	"reflect"
	"strings"
	"sync"
)

// Changed is one name an edit changed, with its value before and after;
// Had says whether the name was there at all.
type Changed struct {
	Name      string
	Before    any
	After     any
	HadBefore bool
	HadAfter  bool
}

// Edit is one edit of the person's.
type Edit struct {
	Part    string
	Scope   string
	Changes []Changed
	Steps   []int64
}

// maxEdits is how many edits a window keeps; the oldest go first.
const maxEdits = 500

// EditBus is the record of one window's edits.
type EditBus struct {
	mu       sync.Mutex
	baseline map[string]any
	going    *Edit
	done     []*Edit
	undone   []*Edit
}

// Reset is the window's data as a reply that is no edit left it: the edits
// start from here. While an edit goes on (a drag), it waits for its end.
func (b *EditBus) Reset(data map[string]any) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.going == nil {
		b.baseline = copied(data)
	}
}

// Drag is an edit that goes on: data is the window's data now, steps the
// database steps its handler made; it ends with Edit.
func (b *EditBus) Drag(part, scope string, data map[string]any, steps []int64) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.going == nil {
		b.going = &Edit{Part: part, Scope: scope}
	}
	b.going.Steps = append(b.going.Steps, steps...)
}

// Edit is the end of an edit of part: data is the window's data now, with
// what its handler changed, steps the database steps that handler made.
// One that changed nothing and made no step is no message; a new edit ends
// what could be done again in its scope.
func (b *EditBus) Edit(part, scope string, data map[string]any, steps []int64) {
	b.mu.Lock()
	defer b.mu.Unlock()
	edit := &Edit{Part: part, Scope: scope}
	if b.going != nil {
		edit.Steps = append(edit.Steps, b.going.Steps...)
		b.going = nil
	}
	edit.Steps = append(edit.Steps, steps...)
	edit.Changes = between(b.baseline, data)
	b.baseline = copied(data)
	if len(edit.Changes) == 0 && len(edit.Steps) == 0 {
		return
	}
	b.done = append(b.done, edit)
	if len(b.done) > maxEdits {
		b.done = b.done[len(b.done)-maxEdits:]
	}
	kept := b.undone[:0]
	for _, e := range b.undone {
		if e.Scope != scope {
			kept = append(kept, e)
		}
	}
	b.undone = kept
}

// Undo takes back (back) or does again the last edit of scope, on data, the
// window's data now: take takes its database steps back or does them again,
// in that order. It gives the window's data after it, and whether there was
// an edit; when take refuses, the edit stays where it was.
func (b *EditBus) Undo(scope string, data map[string]any, back bool, take func(steps []int64, back bool) error) (map[string]any, bool, error) {
	b.mu.Lock()
	defer b.mu.Unlock()
	from, to := &b.done, &b.undone
	if !back {
		from, to = &b.undone, &b.done
	}
	at := -1
	for i := len(*from) - 1; i >= 0; i-- {
		if (*from)[i].Scope == scope {
			at = i
			break
		}
	}
	if at < 0 {
		return nil, false, nil
	}
	edit := (*from)[at]
	steps := append([]int64(nil), edit.Steps...)
	if back {
		for i, j := 0, len(steps)-1; i < j; i, j = i+1, j-1 {
			steps[i], steps[j] = steps[j], steps[i]
		}
	}
	if len(steps) > 0 {
		if err := take(steps, back); err != nil {
			return nil, true, err
		}
	}
	*from = append((*from)[:at], (*from)[at+1:]...)
	*to = append(*to, edit)
	result := copied(data)
	changes := edit.Changes
	for i := range changes {
		c := changes[i]
		if back {
			c = changes[len(changes)-1-i]
		}
		value, had := c.After, c.HadAfter
		if back {
			value, had = c.Before, c.HadBefore
		}
		if had {
			result[c.Name] = copiedValue(value)
		} else {
			delete(result, c.Name)
		}
	}
	b.baseline = copied(result)
	return result, true, nil
}

// between is what changed from before to after, by name.
func between(before, after map[string]any) []Changed {
	var changes []Changed
	seen := map[string]bool{}
	names := make([]string, 0, len(before)+len(after))
	for name := range before {
		names = append(names, name)
	}
	for name := range after {
		names = append(names, name)
	}
	for _, name := range names {
		if seen[name] {
			continue
		}
		seen[name] = true
		was, hadBefore := before[name]
		now, hadAfter := after[name]
		if hadBefore == hadAfter && reflect.DeepEqual(asJSON(was), asJSON(now)) {
			continue
		}
		changes = append(changes, Changed{Name: name, Before: copiedValue(was), After: copiedValue(now), HadBefore: hadBefore, HadAfter: hadAfter})
	}
	return changes
}

// asJSON is a value as JSON has it, so a number compares alike whatever Go type holds it.
func asJSON(value any) any {
	text, err := json.Marshal(value)
	if err != nil {
		return value
	}
	var out any
	_ = json.Unmarshal(text, &out)
	return out
}

func copiedValue(value any) any { return asJSON(value) }

func copied(data map[string]any) map[string]any {
	out := make(map[string]any, len(data))
	for name, value := range data {
		out[name] = copiedValue(value)
	}
	return out
}

// buses is the edit bus of each open window, by its number.
var buses sync.Map

// EditsOf is the edit bus of the window numbered instance.
func EditsOf(instance uint64) *EditBus {
	bus, _ := buses.LoadOrStore(instance, &EditBus{})
	return bus.(*EditBus)
}

// EditsClosed forgets the edits of a window that closed.
func EditsClosed(instance uint64) { buses.Delete(instance) }

// partState names a part's own text or tick among the data an edit compares.
const partState = "#parts."

// EditData is what an edit of a window compares: its data, and each part's
// own text and tick as #parts.<name>.
func EditData(state WindowState) map[string]any {
	data := make(map[string]any, len(state.Data)+len(state.Parts))
	for name, value := range state.Data {
		data[name] = value
	}
	for name, part := range state.Parts {
		own := map[string]any{}
		if part.Text != nil {
			own["text"] = *part.Text
		}
		if part.Checked != nil {
			own["checked"] = *part.Checked
		}
		if len(own) > 0 {
			data[partState+name] = own
		}
	}
	return data
}

// WithEditData is state with data as an edit put it back: the window's data,
// and each part's own text and tick.
func WithEditData(state WindowState, data map[string]any) WindowState {
	state.Data = map[string]any{}
	parts := make(map[string]PartState, len(state.Parts))
	for name, part := range state.Parts {
		part.Text, part.Checked = nil, nil
		parts[name] = part
	}
	for name, value := range data {
		part, own := strings.CutPrefix(name, partState)
		if !own {
			state.Data[name] = value
			continue
		}
		kept := parts[part]
		if fields, ok := value.(map[string]any); ok {
			if text, ok := fields["text"].(string); ok {
				kept.Text = &text
			}
			if checked, ok := fields["checked"].(bool); ok {
				kept.Checked = &checked
			}
		}
		parts[part] = kept
	}
	state.Parts = parts
	return state
}
