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

// A provider's settings go to Clarity.Auth in the block, its secret read
// from the program's protected store, empty when it is not kept.
func TestSigninBlockCarriesProviderOptions(t *testing.T) {
	defer func() { Secrets = nil }()
	source := SigninSource{Program: "shop", Strategies: []SigninStrategy{
		{Name: "github", Columns: []string{"email"}, Options: [][2]string{{"client", "abc"}, {"secret", "setting:githubSecret"}, {"claim", "email"}}},
		{Name: "password", Columns: []string{"email", "password_hash"}},
	}}
	optionsOf := func(at int) map[string]string {
		return source.block()["strategies"].([]map[string]any)[at]["options"].(map[string]string)
	}
	if got := optionsOf(0)["secret"]; got != "" {
		t.Errorf("secret without a store %q, want empty", got)
	}
	Secrets = &countingStore{kept: map[string]string{"shop/githubSecret": "s3cret"}}
	got := optionsOf(0)
	if got["client"] != "abc" || got["secret"] != "s3cret" || got["claim"] != "email" {
		t.Errorf("options %v", got)
	}
	if got := optionsOf(1); len(got) != 0 {
		t.Errorf("password options %v, want none", got)
	}
}
