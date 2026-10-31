package clarity

import "testing"

// countingStore is a protected store that counts what it is asked to keep.
type countingStore struct {
	kept map[string]string
	sets int
}

func (s *countingStore) Get(program, name string) string { return s.kept[program+"/"+name] }

func (s *countingStore) Set(program, name, value string) error {
	s.sets++
	s.kept[program+"/"+name] = value
	return nil
}

// Keeping the settings writes a secret only when it changed: another
// setting changing never needs the protected store, nor fails where it
// cannot be reached (DESIGN 5.27).
func TestASecretGoesToTheStoreOnlyWhenItChanged(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", dir)
	t.Setenv("APPDATA", dir)
	store := &countingStore{kept: map[string]string{"app/token": "abc"}}
	Secrets = store
	defer func() { Secrets = nil }()
	values := ReadSettingsWith("app", []string{"token"})
	values["showToolbar"] = false
	if err := KeepSettingsWith("app", values, []string{"token"}); err != nil {
		t.Fatal(err)
	}
	if store.sets != 0 {
		t.Fatalf("an unchanged secret was written %d times", store.sets)
	}
	values["token"] = "xyz"
	if err := KeepSettingsWith("app", values, []string{"token"}); err != nil {
		t.Fatal(err)
	}
	if store.sets != 1 || store.kept["app/token"] != "xyz" {
		t.Fatalf("the changed secret: %d writes, kept %q", store.sets, store.kept["app/token"])
	}
	// A secret never kept is empty: keeping it empty writes nothing.
	values["other"] = ""
	if err := KeepSettingsWith("app", values, []string{"token", "other"}); err != nil {
		t.Fatal(err)
	}
	if store.sets != 1 {
		t.Fatalf("an empty secret never kept was written")
	}
}
