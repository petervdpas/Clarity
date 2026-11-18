// The files of a file store (X2), against the cases every runtime shares
// (conformance/files.json): listed, read and written as text, and a file as
// a record's picture.

package clarity

import (
	"encoding/base64"
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"testing"
	"time"
)

type fileCases struct {
	Tree []struct {
		Path, Text, Modified string
	}
	Listing []struct {
		Path, Name, Extension, Modified string
		Size                            int64
	}
	Refused []string
	Folders []struct {
		Patterns []string
		Hidden   bool
		Listed   []string
	}
	FolderWrites []struct {
		Patterns []string
		Hidden   bool
		Path     string
		Written  bool
	} `json:"folderWrites"`
	FieldFiles []struct {
		Patterns []string
		Hidden   bool
		Name     string
		Taken    bool
	} `json:"fieldFiles"`
	Bytes []struct {
		Path   string
		Base64 string
		Size   int64
	}
}

func loadFileCases(t *testing.T) fileCases {
	text, err := os.ReadFile("../../../conformance/files.json")
	must(t, err)
	var cases fileCases
	must(t, json.Unmarshal(text, &cases))
	return cases
}

func planted(t *testing.T) (*Db, string, fileCases) {
	cases := loadFileCases(t)
	root := t.TempDir()
	for _, f := range cases.Tree {
		path := filepath.Join(root, filepath.FromSlash(f.Path))
		must(t, os.MkdirAll(filepath.Dir(path), 0o755))
		must(t, os.WriteFile(path, []byte(f.Text), 0o644))
		at, err := time.Parse("2006-01-02T15:04:05", f.Modified)
		must(t, err)
		must(t, os.Chtimes(path, at, at))
	}
	db, err := Memory()
	must(t, err)
	t.Cleanup(func() { db.Close() })
	must(t, db.AddStore("photos", root))
	return db, root, cases
}

func TestAStoreListsItsFilesAsEveryRuntimeDoes(t *testing.T) {
	db, _, cases := planted(t)
	files, err := db.Files("photos")
	must(t, err)
	equal(t, len(files), len(cases.Listing))
	for i, want := range cases.Listing {
		got := files[i]
		equal(t, []any{got.Path, got.Name, got.Extension, got.Size, got.Modified.String()}, []any{want.Path, want.Name, want.Extension, want.Size, want.Modified})
	}
	if _, err := db.Files("films"); err == nil {
		t.Fatal("a store that is not there was listed")
	}
}

func TestTextIsReadAndWrittenInsideTheStore(t *testing.T) {
	db, root, cases := planted(t)
	files, err := db.Files("photos")
	must(t, err)
	var notes FileEntry
	for _, f := range files {
		if f.Name == "notes.txt" {
			notes = f
		}
	}
	text, err := db.ReadText(notes)
	must(t, err)
	equal(t, text, "Hello\nworld\n")
	must(t, db.WriteText("photos", "exports/titles.txt", "Blue Train\n"))
	written, err := os.ReadFile(filepath.Join(root, "exports", "titles.txt"))
	must(t, err)
	equal(t, string(written), "Blue Train\n")
	for _, path := range cases.Refused {
		var invalid *InvalidError
		if err := db.WriteText("photos", path, "x"); !errors.As(err, &invalid) {
			t.Fatalf("%q was written: %v", path, err)
		}
	}
}

func TestAFileBecomesAPictureLinkedInItsOwnStore(t *testing.T) {
	db, root, _ := planted(t)
	png := []byte{0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A}
	must(t, os.MkdirAll(filepath.Join(root, "covers"), 0o755))
	must(t, os.WriteFile(filepath.Join(root, "covers", "cover.png"), png, 0o644))
	files, err := db.Files("photos")
	must(t, err)
	find := func(name string) FileEntry {
		for _, f := range files {
			if f.Name == name {
				return f
			}
		}
		t.Fatalf("no %s", name)
		return FileEntry{}
	}
	linked, err := db.PictureFrom(find("cover.png"), "photos")
	must(t, err)
	equal(t, linked, &Picture{Path: "covers/cover.png"})
	equal(t, PictureValue(linked), any("covers/cover.png"))
	copied, err := db.PictureFrom(find("cover.png"), "")
	must(t, err)
	equal(t, copied, NewPicture(png))
	var invalid *InvalidError
	if _, err := db.PictureFrom(find("notes.txt"), ""); !errors.As(err, &invalid) {
		t.Fatalf("a text file became a picture: %v", err)
	}
	stored, err := db.StoreFile("photos", "covers/cover.png")
	must(t, err)
	equal(t, stored, png)
	if _, err := db.StoreFile("photos", "../x.png"); !errors.As(err, &invalid) {
		t.Fatalf("a path outside the store was read: %v", err)
	}
	wire, err := json.Marshal(linked)
	must(t, err)
	equal(t, string(wire), `{"file":"covers/cover.png"}`)
}

func TestFilesAreFilteredAndSortedByTheirDetails(t *testing.T) {
	at := func(day int) *DateTime { return &DateTime{Date{2026, 1, day}, 0, 0, 0} }
	files := []FileEntry{
		{Name: "b.jpg", Path: "x/b.jpg", Size: 2, Modified: at(3)},
		{Name: "a.png", Path: "a.png", Size: 9, Modified: at(1)},
		{Name: "c.jpg", Path: "c.jpg", Size: 2, Modified: at(2)},
	}
	jpgs := FilterFiles(files, func(f FileEntry) bool { return f.Name != "a.png" })
	equal(t, len(jpgs), 2)
	names := func(fs []FileEntry) []string {
		out := []string{}
		for _, f := range fs {
			out = append(out, f.Name)
		}
		return out
	}
	equal(t, names(SortFiles(files, Desc("size"), Asc("path"))), []string{"a.png", "c.jpg", "b.jpg"})
	equal(t, names(SortFiles(files, Asc("modified"))), []string{"a.png", "c.jpg", "b.jpg"})
	equal(t, names(SortFiles(files, Desc("name"))), []string{"c.jpg", "b.jpg", "a.png"})
	// The list sorted is a copy.
	equal(t, files[0].Name, "b.jpg")
	equal(t, *New(3), 3)
}

func TestBytesAreReadAndWrittenAsTheyAre(t *testing.T) {
	db, _, cases := planted(t)
	for _, c := range cases.Bytes {
		bytes, err := base64.StdEncoding.DecodeString(c.Base64)
		if err != nil {
			t.Fatal(err)
		}
		if err := db.WriteBytes("photos", c.Path, bytes); err != nil {
			t.Fatal(err)
		}
		files, err := db.Files("photos")
		if err != nil {
			t.Fatal(err)
		}
		for _, f := range files {
			if f.Path != c.Path {
				continue
			}
			read, err := db.ReadBytes(f)
			if err != nil || f.Size != c.Size || string(read) != string(bytes) {
				t.Fatalf("%v size %d: %v", read, f.Size, err)
			}
			return
		}
		t.Fatalf("%s is not listed", c.Path)
	}
}

// A folder that is no store, by its path after "folder:": it lists the
// files its extensions take, hidden ones only when it shows them, and
// writes no other (conformance/files.json, folders and folderWrites).
func TestAFolderListsAndWritesTheFilesOfItsExtensions(t *testing.T) {
	_, root, cases := planted(t)
	db, err := Memory()
	must(t, err)
	t.Cleanup(func() { db.Close() })
	place := "folder:" + root
	for _, c := range cases.Folders {
		files, err := db.FilesIn(place, c.Patterns, c.Hidden)
		must(t, err)
		listed := []string{}
		for _, f := range files {
			listed = append(listed, f.Path)
		}
		want := c.Listed
		if want == nil {
			want = []string{}
		}
		equal(t, listed, want)
	}
	notes, err := db.FilesIn(place, []string{"*.txt"}, false)
	must(t, err)
	text, err := db.ReadText(notes[0])
	must(t, err)
	equal(t, text, "Hello\nworld\n")
	for _, c := range cases.FolderWrites {
		err := db.WriteTextIn(place, c.Patterns, c.Hidden, c.Path, "x")
		equal(t, err == nil, c.Written)
	}
	if _, err := db.FilesIn("folder:", nil, false); err == nil {
		t.Fatal("a folder that was never chosen was listed")
	}
}

// A blob or file field that says its extensions takes only those files; one
// of another type is refused in Clarity's own words (conformance/files.json, fieldFiles).
func TestAFieldTakesOnlyTheFilesOfItsExtensions(t *testing.T) {
	cases := loadFileCases(t)
	for _, c := range cases.FieldFiles {
		err := CheckFileType("video", c.Patterns, c.Hidden, c.Name)
		if (err == nil) != c.Taken {
			t.Errorf("%v %q: taken %v, want %v", c.Patterns, c.Name, err == nil, c.Taken)
		}
	}
	err := CheckFileType("video", []string{"*.mp4"}, false, "notes.txt")
	refused, ok := err.(*RefusedError)
	if !ok || refused.Text.Key != "fileTypeRefused" || refused.Text.Show() != "video takes *.mp4 files, not notes.txt" {
		t.Errorf("refused as %#v", err)
	}
}

// A file a person picked in a web app goes into a file store on the server,
// in a folder named by its contents as a blob's is, and its path there is
// what a file field keeps (docs/ledger/files-pictures.md, step 5).
func TestAnUploadGoesIntoAStoreAndGivesItsPath(t *testing.T) {
	db, err := Memory()
	must(t, err)
	t.Cleanup(func() { db.Close() })
	root := t.TempDir()
	must(t, db.AddStore("photos", root))
	path, err := db.StoreUpload("photos", "C:\\me\\cat.png", []byte("meow"))
	must(t, err)
	if want := filepath.Join(root, filepath.FromSlash(StorePath([]byte("meow"), "cat.png"))); path != want {
		t.Fatalf("uploaded to %s, want %s", path, want)
	}
	if b, _ := os.ReadFile(path); string(b) != "meow" {
		t.Fatalf("it holds %q", b)
	}
	if _, err := db.StoreUpload("nowhere", "a.txt", []byte("x")); err == nil {
		t.Fatal("a store that is not there took it")
	}
	if got := db.Stores(); len(got) != 1 || got[0] != (StoreFolder{"photos", root}) {
		t.Fatalf("stores %v", got)
	}
}
