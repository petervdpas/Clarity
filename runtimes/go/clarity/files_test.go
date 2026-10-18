// The files of a file store (X2), against the cases every runtime shares
// (conformance/files.json): listed, read and written as text, and a file as
// a record's picture.

package clarity

import (
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
