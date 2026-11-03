// The files a program carries (DESIGN 7.9): the windows, app frames, themes
// and resources Clarity.UI draws from, on the desktop and in the browser,
// and the files a browser takes from the web target (the page, Clarity.UI's
// browser files, clarity/files.json). CMake puts them into the program as
// arrays (clarity_carry in clarity.cmake); nothing here needs the library.

#pragma once

#include <stddef.h>

/// A file the program carries: its name (a window's, a theme's
/// "shop.light", a resource's "logo.png", a browser file's
/// "clarity/files.json") and its bytes.
typedef struct {
    const char* name;
    const unsigned char* data;
    size_t size;
} ClarityCarried;

/// Everything the program carries, by kind, in the order the project gives it.
typedef struct {
    const ClarityCarried* windows;
    size_t window_count;
    const ClarityCarried* apps;
    size_t app_count;
    const ClarityCarried* themes;
    size_t theme_count;
    const ClarityCarried* resources;
    size_t resource_count;
    /// What the web target serves to a browser, by its path on the server.
    const ClarityCarried* browser;
    size_t browser_count;
} ClarityCarriedFiles;

/// The files the program carries: CMake writes them (clarity_carry in clarity.cmake).
extern const ClarityCarriedFiles clarity_carried;

/// The file of `name` among `count` of them; NULL when none is.
const ClarityCarried* clarity_carried_find(const ClarityCarried* files, size_t count, const char* name);
