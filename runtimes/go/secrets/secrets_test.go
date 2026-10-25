package secrets

// Secret settings in the system's protected store (DESIGN 5.27): kept, read
// back and forgotten, apart from settings.json. Needs a store, such as the
// Secret Service of a desktop session; without one it says so and passes.

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
)

const program = "clarity-runtime-test"

func storeWorks(t *testing.T) bool {
	store := System{}
	works := store.Set(program, "probe", "1") == nil && store.Get(program, "probe") == "1"
	_ = store.Set(program, "probe", "")
	if !works {
		t.Log("no protected store here; the secret tests check nothing")
	}
	return works
}

func TestASecretIsKeptReadAndForgotten(t *testing.T) {
	if !storeWorks(t) {
		return
	}
	store := System{}
	if err := store.Set(program, "token", "s3cr3t"); err != nil {
		t.Fatal(err)
	}
	if got := store.Get(program, "token"); got != "s3cr3t" {
		t.Fatalf("got %q", got)
	}
	if err := store.Set(program, "token", ""); err != nil {
		t.Fatal(err)
	}
	if got := store.Get(program, "token"); got != "" {
		t.Fatalf("still %q", got)
	}
	if err := store.Set(program, "token", ""); err != nil {
		t.Fatal("forgetting what was never kept is no error:", err)
	}
}

func TestSettingsKeepTheirSecretsOutOfTheFile(t *testing.T) {
	if !storeWorks(t) {
		return
	}
	home := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", home)
	clarity.Secrets = System{}
	defer func() { clarity.Secrets = nil }()
	name := "clarity-runtime-test-settings"
	values := map[string]any{"pageSize": float64(20), "apiToken": "t0k3n"}
	if err := clarity.KeepSettingsWith(name, values, []string{"apiToken"}); err != nil {
		t.Fatal(err)
	}
	text, err := os.ReadFile(filepath.Join(home, name, "settings.json"))
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(string(text), "t0k3n") || strings.Contains(string(text), "apiToken") {
		t.Fatalf("the file holds the secret: %s", text)
	}
	read := clarity.ReadSettingsWith(name, []string{"apiToken"})
	if read["apiToken"] != "t0k3n" || read["pageSize"] != float64(20) {
		t.Fatalf("read %v", read)
	}
	_ = System{}.Set(name, "apiToken", "")
}

// A database connects with a secret setting (DESIGN 5.27): its connection,
// password and all, kept in the protected store, not in the program.
func TestADatabaseConnectsWithASecretSetting(t *testing.T) {
	if !storeWorks(t) {
		return
	}
	clarity.Secrets = System{}
	defer func() { clarity.Secrets = nil }()
	file := filepath.Join(t.TempDir(), "secret-connection.db")
	if err := (System{}).Set(program, "shopDb", file); err != nil {
		t.Fatal(err)
	}
	db, err := clarity.Connect(clarity.Source{Name: "main", Dialect: clarity.Sqlite, At: "setting:" + program + "/shopDb"})
	_ = (System{}).Set(program, "shopDb", "")
	if err != nil {
		t.Fatal(err)
	}
	db.Close()
	_, err = clarity.Connect(clarity.Source{Name: "main", Dialect: clarity.Sqlite, At: "setting:" + program + "/shopDb"})
	if err == nil || !strings.Contains(err.Error(), "the secret setting shopDb is not kept yet") {
		t.Fatalf("got %v", err)
	}
}
