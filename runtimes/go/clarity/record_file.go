// Files in records (U1c): a file field holds no file, the one the record
// has stored, a new one chosen but not yet saved, or a file of its file
// store; always with its name. Like a picture, reading records never
// carries the file itself; the screen fetches it on its own to open it or
// save a copy. In the database the bytes are in the field's column and the
// name in the column beside it; in a file store the file keeps its name in
// a folder named by its contents (conformance/record-files.json).

package clarity

import (
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

// RecordFile is the file of a file field that has one, and its name.
type RecordFile struct {
	Name    string
	Content *Picture
}

// CleanName is the name a chosen file is kept under: the last part of what
// was chosen, trimmed; "file" when nothing is left.
func CleanName(chosen string) string {
	last := chosen
	if at := strings.LastIndexAny(chosen, `/\`); at >= 0 {
		last = chosen[at+1:]
	}
	last = strings.TrimSpace(last)
	if last == "" {
		return "file"
	}
	return last
}

func contents(b []byte) string {
	sum := sha256.Sum256(b)
	return hex.EncodeToString(sum[:16])
}

// StorePath is the path of a file in a file store: a folder named by the
// first 32 hex digits of the SHA-256 of its bytes, and in it the file
// under its name.
func StorePath(b []byte, name string) string {
	return contents(b) + "/" + CleanName(name)
}

// NameOf is the name a path in a store shows: its last part.
func NameOf(path string) string {
	if at := strings.LastIndexAny(path, `/\`); at >= 0 {
		return path[at+1:]
	}
	return path
}

// Extension is the extension of a file's name, in lower case without its dot.
func Extension(name string) string {
	return strings.ToLower(strings.TrimPrefix(filepath.Ext(name), "."))
}

// DownloadsIn is the folder a copy is saved in on system goos, with env
// giving the environment; false when it names no home.
func DownloadsIn(goos string, env func(string) string) (string, bool) {
	home := env("HOME")
	if goos == "windows" {
		home = env("USERPROFILE")
	}
	if home == "" {
		return "", false
	}
	return home + "/Downloads", true
}

// FreeName is the name a copy gets where taken names are in use: the name
// itself, else " (2)", " (3)" and on before its extension.
func FreeName(name string, taken func(string) bool) string {
	if !taken(name) {
		return name
	}
	stem, extension := name, ""
	// A leading dot starts a name, not an extension.
	if at := strings.LastIndex(name, "."); at > 0 {
		stem, extension = name[:at], name[at:]
	}
	for n := 2; ; n++ {
		if candidate := fmt.Sprintf("%s (%d)%s", stem, n, extension); !taken(candidate) {
			return candidate
		}
	}
}

func system() string {
	if runtime.GOOS == "darwin" {
		return "macos"
	}
	return runtime.GOOS
}

// SaveCopy saves a copy of a file in the person's Downloads folder, under a
// name free there; the file it wrote.
func SaveCopy(name string, b []byte) (string, error) {
	folder, ok := DownloadsIn(system(), os.Getenv)
	if !ok {
		return "", &InvalidError{What: "this system names no Downloads folder"}
	}
	folder = filepath.FromSlash(folder)
	if err := os.MkdirAll(folder, 0o755); err != nil {
		return "", &StorageError{What: "cannot make " + folder + ": " + err.Error(), Err: err}
	}
	free := FreeName(CleanName(name), func(n string) bool {
		_, err := os.Stat(filepath.Join(folder, n))
		return err == nil
	})
	file := filepath.Join(folder, free)
	if err := os.WriteFile(file, b, 0o644); err != nil {
		return "", &StorageError{What: "cannot write " + file + ": " + err.Error(), Err: err}
	}
	return file, nil
}

// OpenFile opens a file in the program the system uses for its kind, from a
// copy in a folder of its own under the system's temporary folder.
func OpenFile(name string, b []byte) error {
	folder := filepath.Join(os.TempDir(), "clarity-open-"+contents(b))
	if err := os.MkdirAll(folder, 0o755); err != nil {
		return &StorageError{What: "cannot make " + folder + ": " + err.Error(), Err: err}
	}
	file := filepath.Join(folder, CleanName(name))
	if err := os.WriteFile(file, b, 0o644); err != nil {
		return &StorageError{What: "cannot write " + file + ": " + err.Error(), Err: err}
	}
	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "windows":
		cmd = exec.Command("rundll32", "url.dll,FileProtocolHandler", file)
	case "darwin":
		cmd = exec.Command("open", file)
	default:
		cmd = exec.Command("xdg-open", file)
	}
	if err := cmd.Start(); err != nil {
		return &StorageError{What: "cannot open " + file + ": " + err.Error(), Err: err}
	}
	return nil
}

// FileData is a file as the page holds it.
func FileData(b []byte) string { return base64.StdEncoding.EncodeToString(b) }

// FromFileData is the bytes of a file the page sends.
func FromFileData(data string) ([]byte, error) {
	b, err := base64.StdEncoding.DecodeString(data)
	if err != nil {
		return nil, &InvalidError{What: "a file that is no base64: " + err.Error()}
	}
	return b, nil
}

// FileContent is what a file field stores in its own column: as a picture does.
func FileContent(f *RecordFile) any {
	if f == nil {
		return nil
	}
	return PictureValue(f.Content)
}

// FileNameValue is what a file field stores in the column of its name: an
// empty name for no file, as the column is never NULL.
func FileNameValue(f *RecordFile) any {
	if f == nil {
		return ""
	}
	return f.Name
}

// File reads a record's file: whether it has one (as for a picture) at i,
// and its name at i+1.
func (r *Reader) File(i int, column string) *RecordFile {
	content := r.Picture(i, column)
	name := r.Text(i+1, column+"_name")
	if content == nil {
		return nil
	}
	return &RecordFile{Name: CleanName(name), Content: content}
}

// RecordFileFrom is a file of a store as the file of a file field, under
// its name: linked when it is in the field's own store, else copied.
func (db *Db) RecordFileFrom(file FileEntry, fieldStore string) (*RecordFile, error) {
	if fieldStore == file.Store {
		return &RecordFile{Name: file.Name, Content: &Picture{Path: file.Path}}, nil
	}
	b, err := db.readFile(file)
	if err != nil {
		return nil, err
	}
	return &RecordFile{Name: file.Name, Content: NewPicture(b)}, nil
}

// CopiedFile is another record's file, with its name, as CopiedPicture
// copies a picture.
func CopiedFile[T any, P interface {
	*T
	Record
}](db *Db, value *RecordFile, id int64, column, from, to string) (*RecordFile, error) {
	if value == nil {
		return nil, nil
	}
	content, err := CopiedPicture[T, P](db, value.Content, id, column, from, to)
	if err != nil || content == nil {
		return nil, err
	}
	return &RecordFile{Name: value.Name, Content: content}, nil
}

type recordFileWire struct {
	Name   string `json:"name"`
	Stored bool   `json:"stored,omitempty"`
	Data   string `json:"data,omitempty"`
	File   string `json:"file,omitempty"`
}

// MarshalJSON writes {"name", "stored": true}, {"name", "data": base64} for
// a new file, or {"name", "file": path} for a file of the field's own store.
func (f *RecordFile) MarshalJSON() ([]byte, error) {
	w := recordFileWire{Name: f.Name}
	switch {
	case f.Content == nil || (f.Content.Bytes == nil && f.Content.Path == ""):
		w.Stored = true
	case f.Content.Path != "":
		w.File = f.Content.Path
	default:
		w.Data = FileData(f.Content.Bytes)
	}
	return json.Marshal(w)
}

// UnmarshalJSON reads what MarshalJSON writes.
func (f *RecordFile) UnmarshalJSON(text []byte) error {
	var w recordFileWire
	if err := json.Unmarshal(text, &w); err != nil {
		return err
	}
	f.Name = CleanName(w.Name)
	switch {
	case w.Data != "":
		b, err := FromFileData(w.Data)
		if err != nil {
			return err
		}
		f.Content = NewPicture(b)
	case w.File != "":
		f.Content = &Picture{Path: w.File}
	default:
		f.Content = StoredPicture()
	}
	return nil
}

// RecordFileName is the name of a record's file; empty when it has none.
func RecordFileName(f *RecordFile) string {
	if f == nil {
		return ""
	}
	return f.Name
}

// RecordFileExtension is the extension of a record's file, in lower case
// without its dot; empty when it has none.
func RecordFileExtension(f *RecordFile) string { return Extension(RecordFileName(f)) }
