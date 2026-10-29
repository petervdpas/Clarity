// Pictures in records (X2), against the cases every runtime shares
// (conformance/pictures.json), and kept in SQLite or in a file store.

package clarity

import (
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

type pictureCases struct {
	Kinds []struct {
		Extension, Bytes, Type string
	}
	NotPictures []string
	FileNames   []struct{ Bytes, Name string }
	Wire        []struct {
		Value, Bytes string
		JSON         json.RawMessage `json:"json"`
	}
	Presence map[string]string
}

func loadPictureCases(t *testing.T) pictureCases {
	text, err := os.ReadFile("../../../conformance/pictures.json")
	must(t, err)
	var cases pictureCases
	must(t, json.Unmarshal(text, &cases))
	return cases
}

func unhex(t *testing.T, text string) []byte {
	b, err := hex.DecodeString(text)
	must(t, err)
	return b
}

func TestAPictureIsToldByItsFirstBytes(t *testing.T) {
	cases := loadPictureCases(t)
	for _, kind := range cases.Kinds {
		found, ok := KindOf(unhex(t, kind.Bytes))
		if !ok {
			t.Fatalf("%s is no picture", kind.Extension)
		}
		equal(t, []string{found.Extension, found.MediaType}, []string{kind.Extension, kind.Type})
	}
	for _, other := range cases.NotPictures {
		if _, ok := KindOf(unhex(t, other)); ok {
			t.Fatalf("%s is a picture", other)
		}
	}
}

func TestAFileIsNamedByItsBytes(t *testing.T) {
	for _, c := range loadPictureCases(t).FileNames {
		name, err := FileName(unhex(t, c.Bytes))
		must(t, err)
		equal(t, name, c.Name)
	}
	var invalid *InvalidError
	if _, err := FileName([]byte("%PDF")); !errors.As(err, &invalid) {
		t.Fatalf("a PDF has a name: %v", err)
	}
}

func TestAPictureTravelsToTheRendererAsEveryRuntimeSendsIt(t *testing.T) {
	for _, c := range loadPictureCases(t).Wire {
		var value *Picture
		switch c.Value {
		case "stored":
			value = StoredPicture()
		case "new":
			value = NewPicture(unhex(t, c.Bytes))
		}
		got, err := json.Marshal(value)
		must(t, err)
		var want, have any
		must(t, json.Unmarshal(c.JSON, &want))
		must(t, json.Unmarshal(got, &have))
		equal(t, have, want)
		var back *Picture
		must(t, json.Unmarshal(c.JSON, &back))
		equal(t, back, value)
	}
}

func TestReadingRecordsHoldsWhetherThereIsAPicture(t *testing.T) {
	presence := loadPictureCases(t).Presence
	for word, d := range map[string]Dialect{"sqlite": Sqlite, "postgres": Postgres, "mysql": Mysql, "mssql": Mssql} {
		equal(t, Presence(d, "cover"), presence[word])
	}
}

type album struct {
	ID    int64
	Title string
	Cover *Picture
	Photo *Picture
}

func (*album) Table() string     { return "album" }
func (*album) Columns() []string { return []string{"title", "cover", "photo"} }
func (*album) Pictures() []PictureColumn {
	return []PictureColumn{{Column: "cover"}, {Column: "photo", Store: "photos"}}
}
func (a *album) GetID() int64   { return a.ID }
func (a *album) SetID(id int64) { a.ID = id }
func (a *album) Values() []any {
	return []any{a.Title, PictureValue(a.Cover), PictureValue(a.Photo)}
}
func (a *album) Load(id int64, v []any) error {
	r := NewReader(v)
	*a = album{id, r.Text(0, "title"), r.Picture(1, "cover"), r.Picture(2, "photo")}
	return r.Err()
}

var (
	pngBytes = []byte{0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A}
	jpgBytes = []byte{0xFF, 0xD8, 0xFF, 0xE0}
)

func albums(t *testing.T, store string) *Db {
	db, err := Memory()
	must(t, err)
	t.Cleanup(func() { db.Close() })
	_, err = db.Migrate([][]string{{"CREATE TABLE album (id INTEGER PRIMARY KEY, title TEXT NOT NULL, cover BLOB, photo TEXT)"}})
	must(t, err)
	must(t, db.AddStore("photos", store))
	return db
}

func TestAPictureIsKeptInTheDatabaseAndFetchedOnItsOwn(t *testing.T) {
	db := albums(t, t.TempDir())
	a := &album{Title: "Blue Train", Cover: NewPicture(pngBytes)}
	must(t, Save(db, a))
	read, err := Get[album](db, a.ID)
	must(t, err)
	equal(t, read.Cover, StoredPicture())
	equal(t, read.Photo, (*Picture)(nil))
	got, err := PictureOf[album](db, a.ID, "cover")
	must(t, err)
	equal(t, got, pngBytes)

	// Saving the record again keeps the picture it has.
	read.Title = "Blue Train (mono)"
	must(t, Save(db, &read))
	got, _ = PictureOf[album](db, a.ID, "cover")
	equal(t, got, pngBytes)

	// A new picture replaces it; none empties it.
	read.Cover = NewPicture(jpgBytes)
	must(t, Save(db, &read))
	got, _ = PictureOf[album](db, a.ID, "cover")
	equal(t, got, jpgBytes)
	read.Cover = nil
	must(t, Save(db, &read))
	got, err = PictureOf[album](db, a.ID, "cover")
	must(t, err)
	equal(t, got, []byte(nil))

	// Only a picture is kept.
	read.Cover = NewPicture([]byte("%PDF-1.7"))
	var invalid *InvalidError
	if err := Save(db, &read); !errors.As(err, &invalid) {
		t.Fatalf("a PDF was kept: %v", err)
	}
}

func TestAPictureInAFileStoreIsAFileNamedByItsBytes(t *testing.T) {
	store := t.TempDir()
	db := albums(t, store)
	a := &album{Title: "Kind of Blue", Photo: NewPicture(jpgBytes)}
	must(t, Save(db, a))
	name, _ := FileName(jpgBytes)
	written, err := os.ReadFile(filepath.Join(store, name))
	must(t, err)
	equal(t, written, jpgBytes)
	got, err := PictureOf[album](db, a.ID, "photo")
	must(t, err)
	equal(t, got, jpgBytes)

	// A path that leaves the store is refused.
	must(t, db.Batch("", fmt.Sprintf("UPDATE album SET photo = '../secret.png' WHERE id = %d", a.ID)))
	var invalid *InvalidError
	if _, err := PictureOf[album](db, a.ID, "photo"); !errors.As(err, &invalid) {
		t.Fatalf("a path outside the store was read: %v", err)
	}
}

// Another record's picture, copied in code (album.cover = other.cover): one
// in hand is copied as it is, a stored one is read first, and one of a file
// store stays linked only in the same store.
func TestAPictureIsCopiedFromAnotherRecord(t *testing.T) {
	db := albums(t, t.TempDir())
	first := &album{Title: "Blue Train", Cover: NewPicture(pngBytes), Photo: NewPicture(jpgBytes)}
	must(t, Save(db, first))
	read, err := Get[album](db, first.ID)
	must(t, err)
	copied := func(value *Picture, column, from, to string) *Picture {
		t.Helper()
		got, err := CopiedPicture[album](db, value, read.ID, column, from, to)
		must(t, err)
		return got
	}
	// A stored picture: its bytes, from the database or from the store.
	equal(t, copied(read.Cover, "cover", "", ""), NewPicture(pngBytes))
	equal(t, copied(read.Photo, "photo", "photos", ""), NewPicture(jpgBytes))
	// One in hand, and none, as they are.
	equal(t, copied(first.Cover, "cover", "", ""), NewPicture(pngBytes))
	equal(t, copied(nil, "cover", "", ""), (*Picture)(nil))
	// A file of the store: linked in the same store, else its bytes.
	name, _ := FileName(jpgBytes)
	linked := &Picture{Path: name}
	equal(t, copied(linked, "photo", "photos", "photos"), linked)
	equal(t, copied(linked, "photo", "photos", ""), NewPicture(jpgBytes))

	// Saved into the other field, the copy is kept there.
	second := &album{Title: "Copy", Photo: copied(read.Cover, "cover", "", "photos")}
	must(t, Save(db, second))
	got, err := PictureOf[album](db, second.ID, "photo")
	must(t, err)
	equal(t, got, pngBytes)
}

func TestAStoreComesFromItsPlaceOrTheEnvironment(t *testing.T) {
	db, err := Memory()
	must(t, err)
	defer db.Close()
	if err := db.AddStore("photos", "env:CLARITY_NO_SUCH_STORE"); err == nil {
		t.Fatal("a store from an unset variable")
	}
	store := filepath.Join(t.TempDir(), "photos")
	must(t, db.AddStore("photos", store))
	if info, err := os.Stat(store); err != nil || !info.IsDir() {
		t.Fatal("the folder is not made")
	}
}

func TestAPictureShowsAsADataURL(t *testing.T) {
	url, err := DataURL(pngBytes)
	must(t, err)
	equal(t, url, "data:image/png;base64,iVBORw0KGgo=")
}

// A store whose folder is a setting (filestore photos "setting:photosFolder")
// keeps its files where the person's settings say, read again at every use,
// so choosing another folder takes effect at once.
func TestAStoreKeepsItsFilesInAFolderSetting(t *testing.T) {
	home := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", home)
	t.Setenv("APPDATA", home)
	t.Setenv("HOME", home)
	program := "clarity-folder-test"
	file, _ := SettingsFile(program)
	settingsFolder := filepath.Dir(file)
	db, err := Memory()
	must(t, err)
	t.Cleanup(func() { db.Close() })
	_, err = db.Migrate([][]string{{"CREATE TABLE album (id INTEGER PRIMARY KEY, title TEXT NOT NULL, cover BLOB, photo TEXT)"}})
	must(t, err)
	must(t, db.AddStoreSetting("photos", program, "photosFolder", "photos"))

	// Not chosen yet: the default, in the settings' folder.
	must(t, Save(db, &album{Title: "First", Photo: NewPicture(jpgBytes)}))
	name, _ := FileName(jpgBytes)
	written, err := os.ReadFile(filepath.Join(settingsFolder, "photos", name))
	must(t, err)
	equal(t, written, jpgBytes)

	// Chosen while the program runs: the next save keeps its file there.
	chosen := filepath.Join(home, "chosen")
	must(t, KeepSettings(program, map[string]any{"photosFolder": chosen}))
	second := &album{Title: "Second", Photo: NewPicture(pngBytes)}
	must(t, Save(db, second))
	name, _ = FileName(pngBytes)
	written, err = os.ReadFile(filepath.Join(chosen, name))
	must(t, err)
	equal(t, written, pngBytes)
	got, err := PictureOf[album](db, second.ID, "photo")
	must(t, err)
	equal(t, got, pngBytes)
}
