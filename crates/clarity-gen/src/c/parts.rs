//! Custom parts of a C program (DESIGN 5.19 and 7.9): a part is a library of
//! the project (`native "parts/Gauge"`, C with its CMakeLists.txt) that
//! implements the functions Clarity writes from the declaration into its
//! `clarity/<Name>.h`: a kind of part of Clarity.UI, drawing on the canvas
//! the library gives (`clarity_canvas_*` in clarity_ui.h). The program builds
//! each part into itself, and `src/parts.c` registers each kind with
//! Clarity.UI by its name as a table of callbacks.

use std::fmt::Write;

use clarity_spec::Project;
use clarity_spec::code::{Param, PartDecl, TypeRef, TypeRefKind};
use clarity_spec::native;

use crate::OutFile;

/// How a value crosses into the part: an integer as `int64_t`, a bool as
/// `bool`, everything else as text in its plain form.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Integer,
    Bool,
    Text,
}

fn kind(ty: &TypeRef) -> Kind {
    match ty.kind {
        TypeRefKind::Integer | TypeRefKind::Duration => Kind::Integer,
        TypeRefKind::Bool => Kind::Bool,
        _ => Kind::Text,
    }
}

fn c_type(kind: Kind) -> &'static str {
    match kind {
        Kind::Integer => "int64_t",
        Kind::Bool => "bool",
        Kind::Text => "const char*",
    }
}

/// The C expression that reads the JSON value `json` as the part takes it, text through `room`.
fn read(kind: Kind, json: &str, room: &str) -> String {
    match kind {
        Kind::Integer => format!("integer_of({json})"),
        Kind::Bool => format!("cJSON_IsTrue({json})"),
        Kind::Text => format!("text_of({json}, {room}, sizeof {room})"),
    }
}

/// The C expression that makes a JSON value of the C value `name`.
fn json_of(kind: Kind, name: &str) -> String {
    match kind {
        Kind::Integer => format!("cJSON_CreateNumber((double){name})"),
        Kind::Bool => format!("cJSON_CreateBool({name})"),
        Kind::Text => format!("cJSON_CreateString({name} ? {name} : \"\")"),
    }
}

fn params(list: &[Param]) -> String {
    list.iter().map(|p| format!(", {} {}", c_type(kind(&p.ty)), p.name.name)).collect()
}

/// The project's C parts, in the order they are declared.
pub fn parts(project: &Project) -> Vec<&PartDecl> {
    crate::parts::native_parts(project)
}

/// `parts/<Name>/clarity/<Name>.h`: the functions the part's library
/// implements, typed from the declaration, and the tells it calls.
pub fn header(part: &PartDecl) -> String {
    let n = &part.name.name;
    let mut out = String::new();
    let _ = writeln!(out, "// Custom part {n} (DESIGN 5.19 and 7.9): what its library implements, a kind");
    let _ = writeln!(out, "// of part of Clarity.UI. Written by Clarity from the part's declaration, and");
    let _ = writeln!(out, "// written again when the declaration changes; the files beside this folder");
    let _ = writeln!(out, "// are yours.");
    let _ = writeln!(out, "//");
    let _ = writeln!(out, "// Every call comes on the screen's thread. The part draws on the canvas");
    let _ = writeln!(out, "// Clarity.UI gives (clarity_canvas_* in clarity_ui.h) and tells its window");
    let _ = writeln!(out, "// through the cx of the call. An integer is int64_t and a bool is bool; every");
    let _ = writeln!(out, "// other value (text, a decimal, money, a date, an enum) is text in its plain");
    let _ = writeln!(out, "// form, valid during the call.");
    let _ = writeln!(out, "\n#pragma once\n\n#include <stdbool.h>\n#include <stdint.h>\n\n#include \"clarity_ui.h\"\n\n#ifdef __cplusplus\nextern \"C\" {{\n#endif\n");
    let _ = writeln!(out, "/// The part as its library keeps it.");
    let _ = writeln!(out, "typedef struct {n}Part {n}Part;\n");
    let _ = writeln!(out, "/// Makes a new part.");
    let _ = writeln!(out, "{n}Part* {n}_create(void);");
    let _ = writeln!(out, "/// The part is gone; it is not called again.");
    let _ = writeln!(out, "void {n}_destroy({n}Part* part);");
    let _ = writeln!(out, "/// Draws it in its box, in the screen's units.");
    let _ = writeln!(out, "void {n}_draw({n}Part* part, ClarityCanvas* canvas, double x, double y, double width, double height);");
    let _ = writeln!(out, "/// The main button pressed on it, the pointer moved, the button let go.");
    let _ = writeln!(out, "void {n}_pointer_pressed({n}Part* part, const ClarityPointer* at, ClarityCustomCx* cx);");
    let _ = writeln!(out, "void {n}_pointer_moved({n}Part* part, const ClarityPointer* at, ClarityCustomCx* cx);");
    let _ = writeln!(out, "void {n}_pointer_released({n}Part* part, const ClarityPointer* at, ClarityCustomCx* cx);");
    let _ = writeln!(out, "/// Whether it takes the focus, and with it the keys and Ctrl+Z for what it binds.");
    let _ = writeln!(out, "bool {n}_focusable({n}Part* part);");
    let _ = writeln!(out, "/// A key by its name (\"Enter\", \"Left\", \"A\"); whether the part took it.");
    let _ = writeln!(out, "bool {n}_key({n}Part* part, const char* name, uint32_t modifiers, ClarityCustomCx* cx);");
    let _ = writeln!(out, "/// Whether the person can use it, as code sets `enabled`.");
    let _ = writeln!(out, "void {n}_show_enabled({n}Part* part, bool enabled, ClarityCustomCx* cx);");
    if let Some(binds) = &part.binds {
        let _ = writeln!(out, "/// The value it shows, from what it binds; given when its window starts and when it changed.");
        let _ = writeln!(out, "void {n}_show_value({n}Part* part, {} value, ClarityCustomCx* cx);", c_type(kind(binds)));
    }
    for property in &part.properties {
        let p = &property.name.name;
        let _ = writeln!(out, "/// Property {p}, as the window or code sets it.");
        let _ = writeln!(out, "void {n}_show_{p}({n}Part* part, {} {p}, ClarityCustomCx* cx);", c_type(kind(&property.ty)));
    }
    for hear in &part.hears {
        let _ = writeln!(out, "/// What code tells it: {n}.{}.", hear.name.name);
        let _ = writeln!(out, "void {n}_on_{}({n}Part* part, ClarityCustomCx* cx{});", hear.name.name, params(&hear.params));
    }
    let _ = writeln!(out, "\n// Called by the part; Clarity implements them.");
    if let Some(binds) = &part.binds {
        let _ = writeln!(out, "/// The person changed the value: what the part binds takes it (one edit, which Ctrl+Z takes back) and its change handler runs.");
        let _ = writeln!(out, "void {n}_changed(ClarityCustomCx* cx, {} value);", c_type(kind(binds)));
    }
    for tell in &part.tells {
        let _ = writeln!(out, "/// Tells {}: the handler wired to it runs with these values.", tell.name.name);
        let _ = writeln!(out, "void {n}_{}(ClarityCustomCx* cx{});", tell.name.name, params(&tell.params));
    }
    let _ = writeln!(out, "\n#ifdef __cplusplus\n}}\n#endif");
    out
}

/// `src/parts.c`: each part's callbacks as Clarity.UI calls them, read into
/// its typed functions, its tells back, and every kind registered by its name.
pub fn glue(parts: &[&PartDecl]) -> String {
    let mut out = format!("{}\n// The program's custom parts, kinds of part of Clarity.UI (DESIGN 7.9).\n\n#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n\n#include \"cjson/cJSON.h\"\n#include \"desktop/desktop.h\"\n#include \"parts.h\"\n", super::HEADER);
    for part in parts {
        let _ = writeln!(out, "#include \"clarity/{}.h\"", part.name.name);
    }
    out.push_str(
        "\n/// Whether `name` (of `length` bytes) is `word`.\nstatic bool is(const char* name, size_t length, const char* word) { return strlen(word) == length && memcmp(name, word, length) == 0; }\n\n/// A value as an integer: a number, or text that is one.\nstatic int64_t integer_of(const cJSON* value) {\n    if (cJSON_IsNumber(value)) return (int64_t)value->valuedouble;\n    if (cJSON_IsString(value)) return strtoll(value->valuestring, NULL, 10);\n    return 0;\n}\n\n/// A value as plain text, written into `room` when it is a number.\nstatic const char* text_of(const cJSON* value, char* room, size_t size) {\n    if (cJSON_IsString(value)) return value->valuestring;\n    if (cJSON_IsNumber(value)) {\n        snprintf(room, size, \"%.15g\", value->valuedouble);\n        return room;\n    }\n    if (cJSON_IsBool(value)) return cJSON_IsTrue(value) ? \"true\" : \"false\";\n    return \"\";\n}\n\n/// Gives JSON to the library, which copies it; the program may hear it before this returns.\nstatic void tell(ClarityCustomCx* cx, const char* name, cJSON* json) {\n    char* text = cJSON_PrintUnformatted(json);\n    if (!text) return;\n    if (name) clarity_custom_tell(cx, name, strlen(name), text, strlen(text));\n    else clarity_custom_change(cx, text, strlen(text));\n}\n",
    );
    let mut registers = String::new();
    for part in parts {
        let n = &part.name.name;
        let _ = write!(
            out,
            "\n// -- {n} --\n\nstatic void* {n}_make(void* user) {{\n    (void)user;\n    return {n}_create();\n}}\n\nstatic void {n}_set(void* part, const char* name, size_t name_length, const char* value, size_t value_length, ClarityCustomCx* cx) {{\n    clarity_desktop_part_begin();\n    cJSON* given = cJSON_ParseWithLength(value, value_length);\n    char room[64];\n    (void)room;\n    if (is(name, name_length, \"enabled\")) {n}_show_enabled(part, cJSON_IsTrue(given), cx);\n"
        );
        if let Some(binds) = &part.binds {
            let _ = writeln!(out, "    else if (is(name, name_length, \"value\")) {n}_show_value(part, {}, cx);", read(kind(binds), "given", "room"));
        }
        for property in &part.properties {
            let p = &property.name.name;
            let _ = writeln!(out, "    else if (is(name, name_length, \"{p}\")) {n}_show_{p}(part, {}, cx);", read(kind(&property.ty), "given", "room"));
        }
        out.push_str("    clarity_desktop_part_end();\n}\n");
        let _ = write!(out, "\nstatic void {n}_hear(void* part, const char* hear, size_t hear_length, const char* values, size_t values_length, ClarityCustomCx* cx) {{\n    clarity_desktop_part_begin();\n    cJSON* given = cJSON_ParseWithLength(values, values_length);\n    (void)part, (void)hear, (void)hear_length, (void)cx;\n");
        for hear in &part.hears {
            let rooms: String = (0..hear.params.len()).map(|k| format!("        char room{k}[64];\n        (void)room{k};\n")).collect();
            let args: String = hear.params.iter().enumerate().map(|(k, p)| format!(", {}", read(kind(&p.ty), &format!("cJSON_GetArrayItem(given, {k})"), &format!("room{k}")))).collect();
            let _ = write!(out, "    if (is(hear, hear_length, \"{h}\")) {{\n{rooms}        {n}_on_{h}(part, cx{args});\n    }}\n", h = hear.name.name);
        }
        out.push_str("    clarity_desktop_part_end();\n}\n");
        let _ = write!(
            out,
            "\nstatic void {n}_drawn(void* part, ClarityCanvas* canvas, double x, double y, double width, double height) {{ {n}_draw(part, canvas, x, y, width, height); }}\nstatic void {n}_pressed(void* part, const ClarityPointer* at, ClarityCustomCx* cx) {{ {n}_pointer_pressed(part, at, cx); }}\nstatic void {n}_moved(void* part, const ClarityPointer* at, ClarityCustomCx* cx) {{ {n}_pointer_moved(part, at, cx); }}\nstatic void {n}_released(void* part, const ClarityPointer* at, ClarityCustomCx* cx) {{ {n}_pointer_released(part, at, cx); }}\nstatic void {n}_gone(void* part) {{ {n}_destroy(part); }}\nstatic int32_t {n}_focus(void* part) {{ return {n}_focusable(part) ? 1 : 0; }}\n\nstatic int32_t {n}_keyed(void* part, const char* name, size_t name_length, uint32_t modifiers, ClarityCustomCx* cx) {{\n    char key[64];\n    if (name_length >= sizeof key) return 0;\n    memcpy(key, name, name_length);\n    key[name_length] = '\\0';\n    return {n}_key(part, key, modifiers, cx) ? 1 : 0;\n}}\n"
        );
        if let Some(binds) = &part.binds {
            let _ = write!(out, "\nvoid {n}_changed(ClarityCustomCx* cx, {} value) {{\n    clarity_desktop_part_begin();\n    tell(cx, NULL, {});\n    clarity_desktop_part_end();\n}}\n", c_type(kind(binds)), json_of(kind(binds), "value"));
        }
        for t in &part.tells {
            let adds: String = t.params.iter().map(|p| format!("    cJSON_AddItemToArray(values, {});\n", json_of(kind(&p.ty), &p.name.name))).collect();
            let _ = write!(out, "\nvoid {n}_{name}(ClarityCustomCx* cx{}) {{\n    clarity_desktop_part_begin();\n    cJSON* values = cJSON_CreateArray();\n{adds}    tell(cx, \"{name}\", values);\n    clarity_desktop_part_end();\n}}\n", params(&t.params), name = t.name.name);
        }
        let _ = write!(
            registers,
            "    ClarityCustomCallbacks {lower} = {{.make = {n}_make, .set = {n}_set, .hear = {n}_hear, .draw = {n}_drawn, .pointer_pressed = {n}_pressed, .pointer_moved = {n}_moved, .pointer_released = {n}_released, .key = {n}_keyed, .focusable = {n}_focus, .destroy = {n}_gone}};\n    clarity_custom_register(\"{n}\", {len}, &{lower});\n",
            lower = n.to_lowercase(),
            len = n.len()
        );
    }
    let _ = write!(out, "\nvoid program_parts_register(void) {{\n{registers}}}\n");
    out
}

/// `src/parts.h`, which main.c calls.
pub const GLUE_H: &str = "#pragma once\n\n/// Tells Clarity.UI about every kind of part the program has, by its name (DESIGN 7.9).\nvoid program_parts_register(void);\n";

/// The lines of the program's CMakeLists.txt that build each part into it.
pub fn cmake_lines(program: &str, parts: &[&PartDecl]) -> String {
    if parts.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n# The custom parts (DESIGN 5.19), each a library of its own folder, built into\n# the program; they draw with Clarity.UI (clarity_ui.h).\n");
    for part in parts {
        let n = &part.name.name;
        let _ = writeln!(out, "add_subdirectory(parts/{n})\ntarget_link_libraries({n} PUBLIC clarity_ui)\ntarget_link_libraries({program} PRIVATE {n})");
    }
    out
}

/// The parts' folders in the output: each part's own files and its header written from the declaration.
pub fn out_files(project: &Project) -> Vec<OutFile> {
    let mut files = Vec::new();
    for part in parts(project) {
        let n = &part.name.name;
        // The header is Clarity's, written from the declaration, in place of the folder's copy.
        let own = format!("{}/{n}.h", native::OWN_FOLDER);
        for file in project.part_files.iter().flatten().filter(|f| f.part == *n && f.path != own) {
            files.push(OutFile { path: format!("{}/{n}/{}", crate::parts::FOLDER, file.path), text: String::new(), bytes: Some(file.bytes.clone().into()) });
        }
        files.push(OutFile { path: format!("{}/{n}/{own}", crate::parts::FOLDER), text: header(part), bytes: None });
    }
    files
}

/// What declaring a part in a C project writes into its folder: a CMake file
/// and C that keep the part's values and draw a frame, and the header.
/// Clarity never writes over the first two.
pub fn starter(part: &PartDecl) -> Vec<(String, String)> {
    let n = &part.name.name;
    let cmake = format!(
        "# The library of custom part {n}, a kind of part of Clarity.UI. Clarity builds\n# it into the program; the target keeps the part's name.\ncmake_minimum_required(VERSION 3.20)\nproject({n} LANGUAGES C)\n\nset(CMAKE_C_STANDARD 17)\nadd_library({n} STATIC {n}.c)\ntarget_include_directories({n} PUBLIC ${{CMAKE_CURRENT_SOURCE_DIR}})\n# Built on its own (Build in the part's view), clarity_ui.h comes from the folder CLARITY_UI names.\nif(NOT TARGET clarity_ui AND DEFINED ENV{{CLARITY_UI}})\n    target_include_directories({n} PUBLIC \"$ENV{{CLARITY_UI}}/include\")\nendif()\n"
    );
    let field = |name: &str, k: Kind| match k {
        Kind::Integer => format!("    int64_t {name};"),
        Kind::Bool => format!("    bool {name};"),
        Kind::Text => format!("    char {name}[256];"),
    };
    let keep = |name: &str, value: &str, k: Kind| match k {
        Kind::Text => format!("    snprintf(part->{name}, sizeof part->{name}, \"%s\", {value});"),
        _ => format!("    part->{name} = {value};"),
    };
    let mut c = String::new();
    let _ = writeln!(c, "// Custom part {n}: draws on the canvas Clarity.UI gives it. Keep the");
    let _ = writeln!(c, "// functions of clarity/{n}.h, which follow the part's declaration.\n");
    let _ = writeln!(c, "#include <stdio.h>\n#include <stdlib.h>\n\n#include \"clarity/{n}.h\"\n");
    let _ = writeln!(c, "struct {n}Part {{");
    let _ = writeln!(c, "    bool enabled;");
    if let Some(binds) = &part.binds {
        let _ = writeln!(c, "{}", field("value", kind(binds)));
    }
    for property in &part.properties {
        let _ = writeln!(c, "{}", field(&property.name.name, kind(&property.ty)));
    }
    let _ = writeln!(c, "}};\n");
    let _ = writeln!(c, "{n}Part* {n}_create(void) {{\n    return calloc(1, sizeof({n}Part));\n}}\n");
    let _ = writeln!(c, "void {n}_destroy({n}Part* part) {{\n    free(part);\n}}\n");
    let _ = writeln!(
        c,
        "void {n}_draw({n}Part* part, ClarityCanvas* canvas, double x, double y, double width, double height) {{\n    (void)part;\n    uint32_t border = 0xFF888888;\n    clarity_canvas_look_color(canvas, \"border\", 6, &border);\n    clarity_canvas_stroke(canvas, x, y, width, height, border, 1, 4);\n}}\n"
    );
    for event in ["pressed", "moved", "released"] {
        let _ = writeln!(c, "void {n}_pointer_{event}({n}Part* part, const ClarityPointer* at, ClarityCustomCx* cx) {{\n    (void)part, (void)at, (void)cx;\n}}\n");
    }
    let _ = writeln!(c, "bool {n}_focusable({n}Part* part) {{\n    (void)part;\n    return {};\n}}\n", part.binds.is_some());
    let _ = writeln!(c, "bool {n}_key({n}Part* part, const char* name, uint32_t modifiers, ClarityCustomCx* cx) {{\n    (void)part, (void)name, (void)modifiers, (void)cx;\n    return false;\n}}\n");
    let _ = writeln!(c, "void {n}_show_enabled({n}Part* part, bool enabled, ClarityCustomCx* cx) {{\n    part->enabled = enabled;\n    clarity_custom_invalidate(cx);\n}}\n");
    if let Some(binds) = &part.binds {
        let _ = writeln!(c, "void {n}_show_value({n}Part* part, {} value, ClarityCustomCx* cx) {{\n{}\n    clarity_custom_invalidate(cx);\n}}\n", c_type(kind(binds)), keep("value", "value", kind(binds)));
    }
    for property in &part.properties {
        let p = &property.name.name;
        let _ = writeln!(c, "void {n}_show_{p}({n}Part* part, {} {p}, ClarityCustomCx* cx) {{\n{}\n    clarity_custom_invalidate(cx);\n}}\n", c_type(kind(&property.ty)), keep(p, p, kind(&property.ty)));
    }
    for hear in &part.hears {
        let names: String = hear.params.iter().map(|p| format!(", (void){}", p.name.name)).collect();
        let _ = writeln!(c, "void {n}_on_{}({n}Part* part, ClarityCustomCx* cx{}) {{\n    (void)part, (void)cx{names};\n}}\n", hear.name.name, params(&hear.params));
    }
    vec![
        (native::BUILD_FILE.to_string(), cmake),
        (format!("{n}.c"), c.trim_end().to_string() + "\n"),
        (format!("{}/{n}.h", native::OWN_FOLDER), header(part)),
    ]
}
