// Pictures in records: an image field holds no picture (nil), the one the
// record has stored, or a new one loaded but not yet saved. Reading records
// never carries the pictures themselves, so a list of thousands stays
// light; the screen fetches each picture on its own. A picture is kept in
// the database, or as a file in a file store named by the hash of its
// bytes, the column holding that name. The other runtimes do the same
// (conformance/pictures.json).

package clarity

import (
	"bytes"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
)

// Picture is the picture of an image field that has one: stored (Bytes and
// Path empty), new (Bytes), or a file of the field's own store (Path).
type Picture struct {
	// Bytes are those of a picture loaded but not yet saved.
	Bytes []byte
	// Path is that of a file of the field's own file store: saved as that
	// path, nothing copied.
	Path string
}

var stored = &Picture{}

// StoredPicture is the picture of a record that has one stored; it is
// fetched on its own.
func StoredPicture() *Picture { return stored }

// NewPicture is a picture loaded but not yet saved.
func NewPicture(b []byte) *Picture { return &Picture{Bytes: b} }

// PictureColumn is an image field's column, and the file store it keeps its
// pictures in; without one they are in the database.
// For a file field, Name is the column beside it that holds the file's
// name: any file is kept, not only a picture, and in a store under its name.
type PictureColumn struct{ Column, Store, Name string }

// Pictured is a record with image fields: reading it gives only whether
// there is a picture, and saving leaves a stored picture alone.
type Pictured interface{ Pictures() []PictureColumn }

// PictureKind is a kind of picture every stack shows.
type PictureKind struct{ Extension, MediaType string }

// KindOf is the kind of picture b is, told by its first bytes.
func KindOf(b []byte) (PictureKind, bool) {
	switch {
	case bytes.HasPrefix(b, []byte{0x89, 'P', 'N', 'G'}):
		return PictureKind{"png", "image/png"}, true
	case bytes.HasPrefix(b, []byte{0xFF, 0xD8, 0xFF}):
		return PictureKind{"jpg", "image/jpeg"}, true
	case bytes.HasPrefix(b, []byte("GIF8")):
		return PictureKind{"gif", "image/gif"}, true
	case bytes.HasPrefix(b, []byte("BM")):
		return PictureKind{"bmp", "image/bmp"}, true
	case len(b) >= 12 && bytes.HasPrefix(b, []byte("RIFF")) && string(b[8:12]) == "WEBP":
		return PictureKind{"webp", "image/webp"}, true
	}
	return PictureKind{}, false
}

func checked(b []byte) (PictureKind, error) {
	kind, ok := KindOf(b)
	if !ok {
		return kind, &InvalidError{"this file is not a picture; use a png, jpg, gif, bmp or webp file"}
	}
	return kind, nil
}

// FileName is the name of a picture's file in a file store: the first 32
// hex digits of the SHA-256 of its bytes, and the extension of its kind.
func FileName(b []byte) (string, error) {
	kind, err := checked(b)
	if err != nil {
		return "", err
	}
	sum := sha256.Sum256(b)
	return hex.EncodeToString(sum[:16]) + "." + kind.Extension, nil
}

// DataURL is a picture as the page shows it.
func DataURL(b []byte) (string, error) {
	kind, err := checked(b)
	if err != nil {
		return "", err
	}
	return "data:" + kind.MediaType + ";base64," + base64.StdEncoding.EncodeToString(b), nil
}

// Presence is what the select list reads of a picture kept in the
// database: whether there is one, not the picture.
func Presence(d Dialect, column string) string {
	return "CASE WHEN " + d.Quote(column) + " IS NULL THEN 0 ELSE 1 END"
}

// keptValue is a picture the record has stored already: its column is left
// as it is when the record is saved. It never reaches a driver.
type keptValue struct{}

// PictureValue is a picture as it is saved: none, the one stored already
// (kept), or a new one.
func PictureValue(p *Picture) any {
	switch {
	case p == nil:
		return nil
	case p.Path != "":
		return p.Path
	case p.Bytes == nil:
		return keptValue{}
	default:
		return p.Bytes
	}
}

// Picture reads whether a record has a picture: what the select list reads
// of one kept in the database (1 or 0), or the path of its file in a store.
func (r *Reader) Picture(i int, column string) *Picture {
	var there bool
	switch v := r.value(i).(type) {
	case nil:
	case []byte:
		there = len(v) > 0
	case string:
		there = v != ""
	case bool:
		there = v
	default:
		n, ok := integer(v)
		if !ok {
			r.fail(column, v)
			return nil
		}
		there = n != 0
	}
	if there {
		return stored
	}
	return nil
}

// inStore is the file path names in the store at root; a path that would
// leave the store is refused.
func inStore(root, path string) (string, error) {
	parts := strings.Split(strings.ReplaceAll(path, "\\", "/"), "/")
	for _, p := range parts {
		if p == "" || p == "." || p == ".." || filepath.IsAbs(path) {
			return "", &InvalidError{"'" + path + "' is not a picture of its file store"}
		}
	}
	return filepath.Join(append([]string{root}, parts...)...), nil
}

type pictureWire struct {
	Stored bool   `json:"stored,omitempty"`
	Data   string `json:"data,omitempty"`
	File   string `json:"file,omitempty"`
}

// MarshalJSON writes {"stored": true}, {"data": base64} for a new picture,
// or {"file": path} for a file of the field's own store.
func (p *Picture) MarshalJSON() ([]byte, error) {
	if p.Path != "" {
		return json.Marshal(pictureWire{File: p.Path})
	}
	if p.Bytes == nil {
		return json.Marshal(pictureWire{Stored: true})
	}
	return json.Marshal(pictureWire{Data: base64.StdEncoding.EncodeToString(p.Bytes)})
}

func (p *Picture) UnmarshalJSON(text []byte) error {
	var wire pictureWire
	if err := json.Unmarshal(text, &wire); err != nil {
		return err
	}
	if wire.Data == "" {
		*p = Picture{Path: wire.File}
		return nil
	}
	b, err := base64.StdEncoding.DecodeString(wire.Data)
	if err != nil {
		return err
	}
	*p = Picture{Bytes: b}
	return nil
}

func picturesOf(r Record) []PictureColumn {
	if p, ok := r.(Pictured); ok {
		return p.Pictures()
	}
	return nil
}

func pictureColumn(r Record, column string) (PictureColumn, bool) {
	for _, p := range picturesOf(r) {
		if p.Column == column {
			return p, true
		}
	}
	return PictureColumn{}, false
}

// AddStore adds the file store name at a place, a folder (made when it is
// not there yet); env:NAME takes it from that environment variable.
func (db *Db) AddStore(name, at string) error {
	delete(db.storeSettings, name)
	folder, err := location(at)
	if err != nil {
		return err
	}
	if err := os.MkdirAll(folder, 0o755); err != nil {
		return &StorageError{What: "cannot make the file store " + folder + ": " + err.Error(), Err: err}
	}
	if db.stores == nil {
		db.stores = map[string]string{}
	}
	db.stores[name] = folder
	return nil
}

// storeSetting is a file store whose folder is the folder setting setting
// of program (filestore photos "setting:photosFolder"), or def while it is
// not kept.
type storeSetting struct{ program, setting, def string }

// AddStoreSetting adds the file store name whose folder is the folder
// setting setting of program, or def while it is not kept; read again at
// every use, so a folder chosen while the application runs takes effect at
// once.
func (db *Db) AddStoreSetting(name, program, setting, def string) error {
	delete(db.stores, name)
	if db.storeSettings == nil {
		db.storeSettings = map[string]storeSetting{}
	}
	db.storeSettings[name] = storeSetting{program, setting, def}
	_, err := db.storeFolder(name)
	return err
}

func (db *Db) storeFolder(name string) (string, error) {
	if s, ok := db.storeSettings[name]; ok {
		file, ok := SettingsFile(s.program)
		if !ok {
			return "", &StorageError{What: "this system names no folder for application settings"}
		}
		folder := filepath.FromSlash(StoreFolderIn(settingsOS(), filepath.ToSlash(filepath.Dir(file)), name, ReadSettings(s.program)[s.setting], s.def))
		if err := os.MkdirAll(folder, 0o755); err != nil {
			return "", &StorageError{What: "cannot make the file store " + folder + ": " + err.Error(), Err: err}
		}
		return folder, nil
	}
	folder, ok := db.stores[name]
	if !ok {
		return "", &StorageError{What: "there is no file store '" + name + "'"}
	}
	return folder, nil
}

// savedValues are the values of a record as they are saved: a new picture
// checked, and written to its file store when it has one, its column then
// holding the file's name.
func (db *Db) savedValues(record Record) ([]any, error) {
	values := record.Values()
	columns := record.Columns()
	for i, c := range columns {
		p, ok := pictureColumn(record, c)
		b, isNew := values[i].([]byte)
		if !ok || !isNew {
			continue
		}
		// A file of a file field is kept under its own name, in a folder named by its contents.
		if p.Name != "" {
			if p.Store == "" {
				continue
			}
			named := ""
			for j, c := range columns {
				if c == p.Name {
					named, _ = values[j].(string)
				}
			}
			folder, err := db.storeFolder(p.Store)
			if err != nil {
				return nil, err
			}
			path := StorePath(b, named)
			file, err := inStore(folder, path)
			if err != nil {
				return nil, err
			}
			if _, err := os.Stat(file); err != nil {
				if err := os.MkdirAll(filepath.Dir(file), 0o755); err != nil {
					return nil, &StorageError{What: "cannot make " + filepath.Dir(file) + ": " + err.Error(), Err: err}
				}
				if err := os.WriteFile(file, b, 0o644); err != nil {
					return nil, &StorageError{What: "cannot write " + file + ": " + err.Error(), Err: err}
				}
			}
			values[i] = path
			continue
		}
		name, err := FileName(b)
		if err != nil {
			return nil, err
		}
		if p.Store == "" {
			continue
		}
		folder, err := db.storeFolder(p.Store)
		if err != nil {
			return nil, err
		}
		file := filepath.Join(folder, name)
		if _, err := os.Stat(file); err != nil {
			if err := os.WriteFile(file, b, 0o644); err != nil {
				return nil, &StorageError{What: "cannot write " + file + ": " + err.Error(), Err: err}
			}
		}
		values[i] = name
	}
	return values, nil
}

// PictureOf is the picture of field column of record id, read from the
// database or from its file store; nil when the record has none.
func PictureOf[T any, P interface {
	*T
	Record
}](db *Db, id int64, column string) ([]byte, error) {
	var probe T
	record := P(&probe)
	picture, ok := pictureColumn(record, column)
	if !ok {
		return nil, &StorageError{What: record.Table() + " has no picture '" + column + "'"}
	}
	s, err := db.source(sourceOf(record))
	if err != nil {
		return nil, err
	}
	d := s.driver.dialect()
	rows, err := s.driver.rows("SELECT "+d.Quote(column)+" FROM "+d.Quote(record.Table())+" WHERE "+d.Quote(keyOf(record))+" = "+d.Mark(1), []any{id})
	if err != nil {
		return nil, err
	}
	if len(rows) == 0 {
		return nil, &NotFoundError{record.Table(), &id}
	}
	switch v := rows[0][0].(type) {
	case nil:
		return nil, nil
	case []byte:
		if picture.Store == "" {
			return v, nil
		}
		return db.readStored(picture.Store, string(v))
	case string:
		return db.readStored(picture.Store, v)
	default:
		return nil, &StorageError{What: column + " holds no picture"}
	}
}

// CopiedPicture is another record's picture for a field kept in store to
// ("" for the database), as code copies it (album.cover = other.cover).
// value is field column of record id, kept in store from. A picture in hand
// is copied as it is, a stored one is read first, and a file of a store
// stays linked only in that same store.
func CopiedPicture[T any, P interface {
	*T
	Record
}](db *Db, value *Picture, id int64, column, from, to string) (*Picture, error) {
	switch {
	case value == nil:
		return nil, nil
	case value.Bytes != nil:
		return NewPicture(value.Bytes), nil
	case value.Path != "":
		if from == "" {
			return nil, &StorageError{What: column + " is kept in the database, not in a file store"}
		}
		if from == to {
			return &Picture{Path: value.Path}, nil
		}
		b, err := db.StoreFile(from, value.Path)
		if err != nil {
			return nil, err
		}
		return NewPicture(b), nil
	}
	b, err := PictureOf[T, P](db, id, column)
	if err != nil || b == nil {
		return nil, err
	}
	return NewPicture(b), nil
}

func (db *Db) readStored(store, path string) ([]byte, error) {
	if path == "" {
		return nil, nil
	}
	folder, err := db.storeFolder(store)
	if err != nil {
		return nil, err
	}
	file, err := inStore(folder, path)
	if err != nil {
		return nil, err
	}
	b, err := os.ReadFile(file)
	if err != nil {
		return nil, &StorageError{What: "cannot read " + file + ": " + err.Error(), Err: err}
	}
	return b, nil
}
