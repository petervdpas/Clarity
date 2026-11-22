package clarity

import (
	"database/sql"
	"path/filepath"
	"testing"
	"time"
)

// Another program holding the database a moment (Clarity's data view,
// clarity-migrate, a second copy of the app) makes a write wait for it, up to
// five seconds, instead of failing with SQLITE_BUSY at once.
func TestAWriteWaitsWhileAnotherProgramHoldsTheDatabase(t *testing.T) {
	path := filepath.Join(t.TempDir(), "shop.db")
	db, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	if err := db.Batch("", "CREATE TABLE item (id INTEGER PRIMARY KEY, name TEXT)"); err != nil {
		t.Fatal(err)
	}
	other, err := sql.Open("sqlite", "file:"+path)
	if err != nil {
		t.Fatal(err)
	}
	defer other.Close()
	other.SetMaxOpenConns(1)
	if _, err := other.Exec("BEGIN IMMEDIATE"); err != nil {
		t.Fatal(err)
	}
	go func() {
		time.Sleep(300 * time.Millisecond)
		other.Exec("COMMIT")
	}()
	if err := db.Batch("", "INSERT INTO item (name) VALUES ('waited')"); err != nil {
		t.Fatalf("the write did not wait for the other program: %v", err)
	}
}
