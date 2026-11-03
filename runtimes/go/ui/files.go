package ui

/*
#include "callbacks.h"
*/
import "C"

import (
	"io/fs"
	"path"
	"strings"
)

// Files are what the library shows a program from (ClarityFiles): its
// windows' and apps' markup (.cxaml, DESIGN 7.8), its resolved themes, its
// model as JSON, the files of its resources folder, and the title and size
// its main window opens with.
type Files struct {
	c *C.ClarityFiles
}

// NewFiles are empty files: no windows, Clarity's own themes, no model.
func NewFiles() (*Files, error) {
	var c *C.ClarityFiles
	if err := failed(C.clarity_files_new(&c), "clarity_files_new"); err != nil {
		return nil, err
	}
	return &Files{c: c}, nil
}

// Close lets the files go.
func (f *Files) Close() {
	if f.c != nil {
		C.clarity_files_free(f.c)
		f.c = nil
	}
}

// Window is a window's markup by the window's name ("Main").
func (f *Files) Window(name, markup string) error {
	n, nl := text(name)
	m, ml := text(markup)
	return failed(C.clarity_files_window(f.c, n, nl, m, ml), "clarity_files_window")
}

// App is an app's frame markup by the app's name.
func (f *Files) App(name, markup string) error {
	n, nl := text(name)
	m, ml := text(markup)
	return failed(C.clarity_files_app(f.c, n, nl, m, ml), "clarity_files_app")
}

// Theme is a resolved theme by its name and mode ("shop.light") as JSON.
func (f *Files) Theme(name, json string) error {
	n, nl := text(name)
	j, jl := text(json)
	return failed(C.clarity_files_theme(f.c, n, nl, j, jl), "clarity_files_theme")
}

// Model is the model the program is made with, as JSON: its translations, its language, its themes.
func (f *Files) Model(json string) error {
	j, jl := text(json)
	return failed(C.clarity_files_model(f.c, j, jl), "clarity_files_model")
}

// Resource is a file of the project's resources folder by its name ("logo.png", "fonts/x.ttf").
func (f *Files) Resource(name string, bytes []byte) error {
	n, nl := text(name)
	b, bl := bytesOf(bytes)
	return failed(C.clarity_files_resource(f.c, n, nl, b, bl), "clarity_files_resource")
}

// Title is the title of the desktop window.
func (f *Files) Title(title string) error {
	t, tl := text(title)
	return failed(C.clarity_files_title(f.c, t, tl), "clarity_files_title")
}

// Size is the size the main window opens at, in the screen's units; 0 for the host's own.
func (f *Files) Size(width, height float64) error {
	return failed(C.clarity_files_size(f.c, C.double(width), C.double(height)), "clarity_files_size")
}

// FilesOf are the files a generated program carries in folder, as the
// generator writes them: windows/Main.cxaml, apps/Shop.cxaml,
// themes/night-owl.light.json and resources/logo.png, with the model, the
// title and the main window's size.
func FilesOf(folder fs.FS, model []byte, title string, width, height float64) (*Files, error) {
	files, err := NewFiles()
	if err != nil {
		return nil, err
	}
	add := func(dir string, put func(name string, data []byte) error) error {
		return fs.WalkDir(folder, dir, func(at string, entry fs.DirEntry, err error) error {
			if err != nil {
				// A program without that folder (no apps, no resources) has nothing there.
				if at == dir {
					return fs.SkipDir
				}
				return err
			}
			if entry.IsDir() {
				return nil
			}
			data, err := fs.ReadFile(folder, at)
			if err != nil {
				return err
			}
			return put(strings.TrimPrefix(at, dir+"/"), data)
		})
	}
	named := func(extension string, put func(name, text string) error) func(string, []byte) error {
		return func(name string, data []byte) error {
			if path.Ext(name) != extension {
				return nil
			}
			return put(strings.TrimSuffix(name, extension), string(data))
		}
	}
	steps := []error{
		add("windows", named(".cxaml", files.Window)),
		add("apps", named(".cxaml", files.App)),
		add("themes", named(".json", files.Theme)),
		add("resources", files.Resource),
	}
	if len(model) > 0 {
		steps = append(steps, files.Model(string(model)))
	}
	steps = append(steps, files.Title(title), files.Size(width, height))
	for _, err := range steps {
		if err != nil {
			files.Close()
			return nil, err
		}
	}
	return files, nil
}
