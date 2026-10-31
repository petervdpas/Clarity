package clarity

import "testing"

// A path names a value in a window's data: a record's field, a plain value,
// a row of a list.
func TestAPathNamesAValueInTheWindowsData(t *testing.T) {
	data := map[string]any{"album": map[string]any{"tags": []any{"jazz"}}, "readings": 3.0, "albums": []any{map[string]any{"title": "Kind of Blue"}}}
	if got, ok := ValueAt(data, "album.tags"); !ok || len(got.([]any)) != 1 {
		t.Error("album.tags", got)
	}
	if got, ok := ValueAt(data, "albums.0.title"); !ok || got != "Kind of Blue" {
		t.Error("albums.0.title", got)
	}
	if _, ok := ValueAt(data, "album.nope.deeper"); ok {
		t.Error("a path that is not there")
	}
	must(t, SetValueAt(data, "readings", 4.0))
	must(t, SetValueAt(data, "albums.0.title", "Blue Train"))
	if data["readings"] != 4.0 || data["albums"].([]any)[0].(map[string]any)["title"] != "Blue Train" {
		t.Error(data)
	}
	if SetValueAt(data, "readings.x", 1) == nil || SetValueAt(data, "albums.5.title", 1) == nil {
		t.Error("set where there is no record")
	}
}
