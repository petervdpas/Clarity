// The files of a file store, which code reaches through `files f in store`,
// `read text of f` and `write t to store "path"`, and of any other folder
// (`files f in folder category.folder`), by its path after "folder:", with
// the extensions of the files it holds and whether its hidden ones count. A file found can become a record's picture: in the field's
// own store by its path, elsewhere as a copy. The other runtimes do the
// same (conformance/files.json).

package clarity

import (
	"cmp"
	"os"
	"path/filepath"
	"slices"
	"sort"
	"strings"
	"unicode/utf8"
)

// FileEntry is a file of a file store: its path inside the store with '/'
// between folders, its extension in lower case without the dot.
type FileEntry struct {
	Store     string
	Path      string
	Name      string
	Extension string
	Size      int64
	// Modified is when the file last changed, in UTC to the second.
	Modified *DateTime
}

// Files is every file of a store and its folders, but those whose name or
// folder starts with a dot, sorted by path.
func (db *Db) Files(store string) ([]FileEntry, error) {
	root, err := db.storeFolder(store)
	if err != nil {
		return nil, err
	}
	var found []FileEntry
	if err := walkStore(store, root, "", &found); err != nil {
		return nil, err
	}
	sort.Slice(found, func(i, j int) bool { return found[i].Path < found[j].Path })
	return found, nil
}

// FolderStore is the store of a folder that is no store: "folder:" and its path.
const FolderStore = "folder:"

// Takes is whether a file's name (Notes.MD, .env) is one a folder takes: by
// its extensions ("*.md", ignoring case; none or "*.*": any file), a hidden
// one (its name starting with a dot) only with hidden (conformance/files.json).
func Takes(patterns []string, hidden bool, name string) bool {
	if strings.HasPrefix(name, ".") && !hidden {
		return false
	}
	if len(patterns) == 0 || slices.Contains(patterns, "*.*") {
		return true
	}
	extension := strings.ToLower(strings.TrimPrefix(filepath.Ext(name), "."))
	for _, p := range patterns {
		if ext, ok := strings.CutPrefix(p, "*."); ok && extension != "" && extension == strings.ToLower(ext) {
			return true
		}
	}
	return false
}

// FilesIn is the files of a folder ("folder:" and its path) its extensions
// take, in its folders too, sorted by path; hidden ones only with hidden.
func (db *Db) FilesIn(folder string, patterns []string, hidden bool) ([]FileEntry, error) {
	root, err := db.storeFolder(folder)
	if err != nil {
		return nil, err
	}
	var found []FileEntry
	if err := walkFolder(folder, root, "", hidden, &found); err != nil {
		return nil, err
	}
	kept := []FileEntry{}
	for _, f := range found {
		if Takes(patterns, hidden, f.Name) {
			kept = append(kept, f)
		}
	}
	sort.Slice(kept, func(i, j int) bool { return kept[i].Path < kept[j].Path })
	return kept, nil
}

// WriteTextIn writes text as the file path of a folder, when it takes the file.
func (db *Db) WriteTextIn(folder string, patterns []string, hidden bool, path, text string) error {
	return db.WriteBytesIn(folder, patterns, hidden, path, []byte(text))
}

// WriteBytesIn writes bytes as the file path of a folder, when it takes the file.
func (db *Db) WriteBytesIn(folder string, patterns []string, hidden bool, path string, bytes []byte) error {
	name := path[strings.LastIndex(path, "/")+1:]
	if !Takes(patterns, hidden, name) {
		holds := strings.Join(patterns, " ")
		if strings.HasPrefix(name, ".") && !hidden {
			holds = "no hidden"
		}
		return &InvalidError{"the folder holds " + holds + " files, not '" + path + "'"}
	}
	return db.WriteBytes(folder, path, bytes)
}

func walkStore(store, folder, prefix string, found *[]FileEntry) error {
	return walkFolder(store, folder, prefix, false, found)
}

// walkFolder finds the files under folder; hidden ones only with hidden.
func walkFolder(store, folder, prefix string, hidden bool, found *[]FileEntry) error {
	entries, err := os.ReadDir(folder)
	if err != nil {
		return &StorageError{What: "cannot read " + folder + ": " + err.Error(), Err: err}
	}
	for _, e := range entries {
		name := e.Name()
		if strings.HasPrefix(name, ".") && !hidden {
			continue
		}
		path := filepath.Join(folder, name)
		if e.IsDir() {
			if err := walkFolder(store, path, prefix+name+"/", hidden, found); err != nil {
				return err
			}
			continue
		}
		info, err := e.Info()
		if err != nil {
			return &StorageError{What: "cannot read " + path + ": " + err.Error(), Err: err}
		}
		if !info.Mode().IsRegular() {
			continue
		}
		modified := DateTimeOf(info.ModTime().UTC())
		extension := strings.ToLower(strings.TrimPrefix(filepath.Ext(name), "."))
		*found = append(*found, FileEntry{store, prefix + name, name, extension, info.Size(), &modified})
	}
	return nil
}

func (db *Db) readFile(file FileEntry) ([]byte, error) {
	root, err := db.storeFolder(file.Store)
	if err != nil {
		return nil, err
	}
	path, err := inStore(root, file.Path)
	if err != nil {
		return nil, err
	}
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, &StorageError{What: "cannot read " + path + ": " + err.Error(), Err: err}
	}
	return b, nil
}

// StoreFile is the bytes of the file path of a store: a picture linked from
// a file before its record is saved.
func (db *Db) StoreFile(store, path string) ([]byte, error) {
	return db.readFile(FileEntry{Store: store, Path: path})
}

// ReadText is the text a file holds.
func (db *Db) ReadText(file FileEntry) (string, error) {
	b, err := db.readFile(file)
	if err != nil {
		return "", err
	}
	if !utf8.Valid(b) {
		return "", &InvalidError{file.Path + " is not text"}
	}
	return string(b), nil
}

// WriteText writes text as the file path of a store, making its folders.
func (db *Db) WriteText(store, path, text string) error {
	return db.WriteBytes(store, path, []byte(text))
}

// ReadBytes is the contents of a file as they are.
func (db *Db) ReadBytes(file FileEntry) ([]byte, error) {
	return db.readFile(file)
}

// WriteBytes writes bytes as they are as the file path of a store, making its folders.
func (db *Db) WriteBytes(store, path string, bytes []byte) error {
	root, err := db.storeFolder(store)
	if err != nil {
		return err
	}
	file, err := inStore(root, path)
	if err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(file), 0o755); err != nil {
		return &StorageError{What: "cannot make " + filepath.Dir(file) + ": " + err.Error(), Err: err}
	}
	if err := os.WriteFile(file, bytes, 0o644); err != nil {
		return &StorageError{What: "cannot write " + file + ": " + err.Error(), Err: err}
	}
	return nil
}

// PictureFrom is a file as the picture of a field kept in fieldStore (empty:
// in the database): the file itself when it is in that store, else a copy.
func (db *Db) PictureFrom(file FileEntry, fieldStore string) (*Picture, error) {
	b, err := db.readFile(file)
	if err != nil {
		return nil, err
	}
	if _, err := checked(b); err != nil {
		return nil, err
	}
	if fieldStore == file.Store {
		return &Picture{Path: file.Path}, nil
	}
	return NewPicture(b), nil
}

// FilterFiles is the files keep holds for, in their order.
func FilterFiles(files []FileEntry, keep func(FileEntry) bool) []FileEntry {
	out := []FileEntry{}
	for _, f := range files {
		if keep(f) {
			out = append(out, f)
		}
	}
	return out
}

// FileOrder is one key of an order of files: a detail, ascending or not.
type FileOrder struct {
	detail     string
	descending bool
}

// Asc orders files by a detail, smallest first.
func Asc(detail string) FileOrder { return FileOrder{detail, false} }

// Desc orders files by a detail, largest first.
func Desc(detail string) FileOrder { return FileOrder{detail, true} }

// compareFiles compares two files by a detail; text by its bytes, as on
// every stack.
func compareFiles(a, b FileEntry, detail string) int {
	switch detail {
	case "name":
		return strings.Compare(a.Name, b.Name)
	case "extension":
		return strings.Compare(a.Extension, b.Extension)
	case "size":
		return cmp.Compare(a.Size, b.Size)
	case "modified":
		return CompareDateTimes(a.Modified, b.Modified)
	default:
		return strings.Compare(a.Path, b.Path)
	}
}

// SortFiles is a copy of files sorted by the keys, the first deciding first.
func SortFiles(files []FileEntry, keys ...FileOrder) []FileEntry {
	out := slices.Clone(files)
	slices.SortStableFunc(out, func(a, b FileEntry) int {
		for _, k := range keys {
			c := compareFiles(a, b, k.detail)
			if k.descending {
				c = -c
			}
			if c != 0 {
				return c
			}
		}
		return 0
	})
	return out
}

// New is a pointer to v, for a new record in code.
func New[T any](v T) *T { return &v }
