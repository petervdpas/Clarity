//! Native custom parts in the output (DESIGN 5.19 and 6.5), the same for
//! every stack: the C header each part's library implements, the glue that
//! turns the part host's plain calls into the header's typed ones, the CMake
//! project that builds the part host, the glue and the parts into one shared
//! library, `clarity_parts`, and the starter a newly declared part gets.

use std::fmt::Write;

use clarity_spec::code::{Param, PartDecl, PartLibrary, TypeRef, TypeRefKind};
use clarity_spec::{Project, native};

use crate::OutFile;

/// The part host's sources, copied into every program with native parts.
const HOST: [(&str, &str); 7] = [
    ("clarity_parts.h", include_str!("../../../runtimes/parts/clarity_parts.h")),
    ("clarity_parts.c", include_str!("../../../runtimes/parts/clarity_parts.c")),
    ("platform.h", include_str!("../../../runtimes/parts/platform.h")),
    ("platform_x11.c", include_str!("../../../runtimes/parts/platform_x11.c")),
    ("platform_win32.c", include_str!("../../../runtimes/parts/platform_win32.c")),
    ("platform_none.c", include_str!("../../../runtimes/parts/platform_none.c")),
    ("cJSON.c", include_str!("../../../runtimes/c/third_party/cjson/cJSON.c")),
];
const CJSON_H: &str = include_str!("../../../runtimes/c/third_party/cjson/cJSON.h");

/// The folder of the output the parts are built in.
pub const FOLDER: &str = "parts";

/// The project's native parts, in the order they are declared.
pub fn native_parts(project: &Project) -> Vec<&PartDecl> {
    project.files.iter().flat_map(|f| &f.parts).filter(|p| matches!(p.library, Some(PartLibrary::Native { .. }))).collect()
}

/// How a value of `ty` crosses into C: an integer as `int64_t`, a bool as
/// `bool`, everything else as text in its plain form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CValue {
    Integer,
    Bool,
    Text,
}

fn c_value(ty: &TypeRef) -> CValue {
    match ty.kind {
        TypeRefKind::Integer => CValue::Integer,
        TypeRefKind::Bool => CValue::Bool,
        _ => CValue::Text,
    }
}

fn c_type(value: CValue) -> &'static str {
    match value {
        CValue::Integer => "int64_t",
        CValue::Bool => "bool",
        CValue::Text => "const char*",
    }
}

fn params(list: &[Param]) -> String {
    list.iter().map(|p| format!(", {} {}", c_type(c_value(&p.ty)), p.name.name)).collect()
}

/// `parts/Arena/clarity/Arena.h`: what the library of part `part` implements,
/// and the tells it calls, typed from the declaration.
pub fn header(part: &PartDecl) -> String {
    let n = &part.name.name;
    let mut out = String::new();
    let _ = writeln!(out, "// Custom part {n} (DESIGN 5.19): what its library implements and the tells it");
    let _ = writeln!(out, "// calls. Written by Clarity from the part's declaration, and written again");
    let _ = writeln!(out, "// when the declaration changes; the files beside this folder are yours.");
    let _ = writeln!(out, "//");
    let _ = writeln!(out, "// Every function is called on the part host's thread, one at a time. An");
    let _ = writeln!(out, "// integer is int64_t and a bool is bool; every other value (text, a decimal,");
    let _ = writeln!(out, "// money, a date, an enum) is text in its plain form, valid during the call.");
    let _ = writeln!(out, "\n#pragma once\n\n#include <stdbool.h>\n#include <stdint.h>\n\n#ifdef __cplusplus\nextern \"C\" {{\n#endif\n");
    let _ = writeln!(out, "/// The part as the library keeps it.");
    let _ = writeln!(out, "typedef struct {n}Part {n}Part;");
    let _ = writeln!(out, "/// Where the part tells what happened; given to {n}_create.");
    let _ = writeln!(out, "typedef struct {n}Host {n}Host;\n");
    let _ = writeln!(out, "/// Makes the part in `parent`, the native window it draws in (an X11 window");
    let _ = writeln!(out, "/// on Linux), `width` by `height` pixels.");
    let _ = writeln!(out, "{n}Part* {n}_create(void* parent, int width, int height, {n}Host* host);");
    let _ = writeln!(out, "/// The window it draws in changed size.");
    let _ = writeln!(out, "void {n}_resize({n}Part* part, int width, int height);");
    let _ = writeln!(out, "/// About 60 times a second while the part is shown: draw, and read input.");
    let _ = writeln!(out, "void {n}_frame({n}Part* part);");
    let _ = writeln!(out, "/// The window closed; the part is not called again.");
    let _ = writeln!(out, "void {n}_destroy({n}Part* part);");
    let _ = writeln!(out, "/// Whether the person can use it, as code sets `enabled`.");
    let _ = writeln!(out, "void {n}_set_enabled({n}Part* part, bool enabled);");
    if let Some(binds) = &part.binds {
        let _ = writeln!(out, "/// The value it shows, from what it binds.");
        let _ = writeln!(out, "void {n}_set_value({n}Part* part, {} value);", c_type(c_value(binds)));
    }
    for property in &part.properties {
        let p = &property.name.name;
        let _ = writeln!(out, "/// Property {p}, as the window or code sets it.");
        let _ = writeln!(out, "void {n}_set_{p}({n}Part* part, {} {p});", c_type(c_value(&property.ty)));
    }
    for hear in &part.hears {
        let _ = writeln!(out, "/// What code tells it: {n}.{}.", hear.name.name);
        let _ = writeln!(out, "void {n}_{}({n}Part* part{});", hear.name.name, params(&hear.params));
    }
    let _ = writeln!(out, "\n// Called by the library; Clarity implements them.");
    if let Some(binds) = &part.binds {
        let _ = writeln!(out, "/// The person changed its value: what it binds takes it, and `on change` runs.");
        let _ = writeln!(out, "void {n}_changed({n}Host* host, {} value);", c_type(c_value(binds)));
    }
    for tell in &part.tells {
        let _ = writeln!(out, "/// Tells {}: its handler runs with these values.", tell.name.name);
        let _ = writeln!(out, "void {n}_{}({n}Host* host{});", tell.name.name, params(&tell.params));
    }
    let _ = writeln!(out, "\n#ifdef __cplusplus\n}}\n#endif");
    out
}

/// The C expression that reads a JSON value as `value`.
fn read(value: CValue, json: &str, room: &str) -> String {
    match value {
        CValue::Integer => format!("clarity_integer({json})"),
        CValue::Bool => format!("cJSON_IsTrue({json})"),
        CValue::Text => format!("clarity_text({json}, {room}, sizeof {room})"),
    }
}

/// The C expression that makes a JSON value of the C value `name`.
fn json_of(value: CValue, name: &str) -> String {
    match value {
        CValue::Integer => format!("cJSON_CreateNumber((double){name})"),
        CValue::Bool => format!("cJSON_CreateBool({name})"),
        CValue::Text => format!("cJSON_CreateString({name} ? {name} : \"\")"),
    }
}

/// `parts/glue.c`: the part host's plain calls to each part's typed functions, and its tells back.
pub fn glue(parts: &[&PartDecl]) -> String {
    let mut out = String::new();
    out.push_str("// The project's native parts for the part host (DESIGN 6.5): its plain calls\n// to each part's typed functions, and their tells back. Written by Clarity.\n\n");
    out.push_str("#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n\n#include \"clarity_parts.h\"\n");
    for part in parts {
        let n = &part.name.name;
        let _ = writeln!(out, "#include \"{n}/clarity/{n}.h\"");
    }
    out.push_str(
        "\n/// A value as an integer: a number, or text that is one.\nstatic int64_t clarity_integer(const cJSON* value) {\n    if (cJSON_IsNumber(value)) return (int64_t)value->valuedouble;\n    if (cJSON_IsString(value)) return strtoll(value->valuestring, NULL, 10);\n    return 0;\n}\n\n/// A value as plain text, written into `room` when it is a number.\nstatic const char* clarity_text(const cJSON* value, char* room, size_t size) {\n    if (cJSON_IsString(value)) return value->valuestring;\n    if (cJSON_IsNumber(value)) {\n        snprintf(room, size, \"%.15g\", value->valuedouble);\n        return room;\n    }\n    if (cJSON_IsBool(value)) return cJSON_IsTrue(value) ? \"true\" : \"false\";\n    return \"\";\n}\n",
    );
    for part in parts {
        let n = &part.name.name;
        let _ = write!(
            out,
            "\n// -- {n} --\n\nstatic void* {n}_create_glue(void* parent, int width, int height, ClarityPartHost* host) {{\n    return {n}_create(parent, width, height, ({n}Host*)host);\n}}\n\nstatic void {n}_resize_glue(void* part, int width, int height) {{ {n}_resize(({n}Part*)part, width, height); }}\nstatic void {n}_frame_glue(void* part) {{ {n}_frame(({n}Part*)part); }}\nstatic void {n}_destroy_glue(void* part) {{ {n}_destroy(({n}Part*)part); }}\n\nstatic void {n}_set_glue(void* part, const char* name, const cJSON* value) {{\n    char room[64];\n    (void)room;\n    if (strcmp(name, \"enabled\") == 0) {{\n        {n}_set_enabled(({n}Part*)part, cJSON_IsTrue(value));\n    }}"
        );
        if let Some(binds) = &part.binds {
            let _ = write!(out, " else if (strcmp(name, \"value\") == 0) {{\n        {n}_set_value(({n}Part*)part, {});\n    }}", read(c_value(binds), "value", "room"));
        }
        for property in &part.properties {
            let p = &property.name.name;
            let _ = write!(out, " else if (strcmp(name, \"{p}\") == 0) {{\n        {n}_set_{p}(({n}Part*)part, {});\n    }}", read(c_value(&property.ty), "value", "room"));
        }
        out.push_str("\n}\n");
        let _ = write!(out, "\nstatic void {n}_hear_glue(void* part, const char* hear, const cJSON* values) {{\n    (void)part, (void)hear, (void)values;\n");
        for (i, hear) in part.hears.iter().enumerate() {
            let rooms: String = (0..hear.params.len()).map(|k| format!("    char room{k}[64];\n    (void)room{k};\n")).collect();
            let args: String = hear
                .params
                .iter()
                .enumerate()
                .map(|(k, p)| format!(", {}", read(c_value(&p.ty), &format!("cJSON_GetArrayItem(values, {k})"), &format!("room{k}"))))
                .collect();
            let keyword = if i == 0 { "if" } else { "} else if" };
            let _ = write!(out, "    {keyword} (strcmp(hear, \"{}\") == 0) {{\n{rooms}        {n}_{}(({n}Part*)part{args});\n", hear.name.name, hear.name.name);
        }
        if !part.hears.is_empty() {
            out.push_str("    }\n");
        }
        out.push_str("}\n");
        if let Some(binds) = &part.binds {
            let _ = write!(
                out,
                "\nvoid {n}_changed({n}Host* host, {} value) {{\n    cJSON* values = cJSON_CreateArray();\n    cJSON_AddItemToArray(values, {});\n    clarity_parts_tell((ClarityPartHost*)host, \"change\", values);\n}}\n",
                c_type(c_value(binds)),
                json_of(c_value(binds), "value")
            );
        }
        for tell in &part.tells {
            let adds: String = tell.params.iter().map(|p| format!("    cJSON_AddItemToArray(values, {});\n", json_of(c_value(&p.ty), &p.name.name))).collect();
            let _ = write!(
                out,
                "\nvoid {n}_{}({n}Host* host{}) {{\n    cJSON* values = cJSON_CreateArray();\n{adds}    clarity_parts_tell((ClarityPartHost*)host, \"{}\", values);\n}}\n",
                tell.name.name,
                params(&tell.params),
                tell.name.name
            );
        }
    }
    out.push_str("\nconst ClarityPartKind clarity_part_kinds[] = {\n");
    for part in parts {
        let n = &part.name.name;
        let _ = writeln!(out, "    {{\"{n}\", {n}_create_glue, {n}_resize_glue, {n}_frame_glue, {n}_destroy_glue, {n}_set_glue, {n}_hear_glue}},");
    }
    out.push_str("    {NULL, NULL, NULL, NULL, NULL, NULL, NULL},\n};\n");
    out
}

/// `parts/CMakeLists.txt`: the part host, the glue and every native part
/// built into the shared library `clarity_parts`, copied into each folder of
/// `beside` (relative to the parts folder), where the program is.
pub fn cmake(parts: &[&PartDecl], beside: &[&str]) -> String {
    let mut out = String::new();
    out.push_str("# The project's native parts with Clarity's part host, built into the shared\n# library clarity_parts that the program loads (DESIGN 6.5). Written by Clarity.\ncmake_minimum_required(VERSION 3.20)\nproject(clarity_parts LANGUAGES C CXX)\n\nset(CMAKE_C_STANDARD 17)\nset(CMAKE_POSITION_INDEPENDENT_CODE ON)\nfind_package(Threads REQUIRED)\n\n");
    for part in parts {
        let _ = writeln!(out, "add_subdirectory({})", part.name.name);
    }
    out.push_str("\nif(UNIX AND NOT APPLE)\n    find_package(X11 REQUIRED)\n    set(CLARITY_PLATFORM host/platform_x11.c)\nelseif(WIN32)\n    set(CLARITY_PLATFORM host/platform_win32.c)\nelse()\n    set(CLARITY_PLATFORM host/platform_none.c)\nendif()\n\nadd_library(clarity_parts SHARED host/clarity_parts.c ${CLARITY_PLATFORM} host/cJSON.c glue.c)\ntarget_include_directories(clarity_parts PRIVATE host ${CMAKE_CURRENT_SOURCE_DIR})\n");
    let names: Vec<&str> = parts.iter().map(|p| p.name.name.as_str()).collect();
    let _ = writeln!(out, "target_link_libraries(clarity_parts PRIVATE {} Threads::Threads)", names.join(" "));
    out.push_str("if(UNIX AND NOT APPLE)\n    target_link_libraries(clarity_parts PRIVATE X11::X11)\nendif()\n# Beside the program when a program's own CMake project builds this folder.\nset_target_properties(clarity_parts PROPERTIES LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR} RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR})\n# The shells load it by one name on every system; MinGW would call it libclarity_parts.dll.\nif(WIN32)\n    set_target_properties(clarity_parts PROPERTIES PREFIX \"\")\nendif()\n");
    for dir in beside {
        let _ = writeln!(
            out,
            "add_custom_command(TARGET clarity_parts POST_BUILD\n    COMMAND ${{CMAKE_COMMAND}} -E make_directory ${{CMAKE_CURRENT_SOURCE_DIR}}/{dir}\n    COMMAND ${{CMAKE_COMMAND}} -E copy_if_different $<TARGET_FILE:clarity_parts> ${{CMAKE_CURRENT_SOURCE_DIR}}/{dir})"
        );
    }
    out
}

/// The files of the parts folder of a program: the part host, the glue, the
/// CMake project, and each native part's folder with its header written
/// from the declaration. None without native parts.
pub fn out_files(project: &Project, beside: &[&str]) -> Vec<OutFile> {
    let parts = native_parts(project);
    if parts.is_empty() {
        return Vec::new();
    }
    let text = |path: String, text: String| OutFile { path, text, bytes: None };
    let mut files: Vec<OutFile> = HOST.iter().map(|(name, source)| text(format!("{FOLDER}/host/{name}"), source.to_string())).collect();
    files.push(text(format!("{FOLDER}/host/cJSON.h"), CJSON_H.to_string()));
    files.push(text(format!("{FOLDER}/glue.c"), glue(&parts)));
    files.push(text(format!("{FOLDER}/CMakeLists.txt"), cmake(&parts, beside)));
    for part in &parts {
        let n = &part.name.name;
        for file in project.part_files.iter().flatten().filter(|f| f.part == *n) {
            files.push(OutFile { path: format!("{FOLDER}/{n}/{}", file.path), text: String::new(), bytes: Some(file.bytes.clone().into()) });
        }
        files.push(text(format!("{FOLDER}/{n}/{}/{n}.h", native::OWN_FOLDER), header(part)));
    }
    files
}

/// The commands that build the parts folder before the program, run in the
/// program's folder: none when it has no native parts, or when its own CMake
/// project builds them (the C and C++ stacks).
pub fn steps_before(out: &std::path::Path, backend: clarity_project::Backend) -> Vec<Vec<String>> {
    let own = matches!(backend, clarity_project::Backend::C | clarity_project::Backend::Cpp);
    if own || !out.join(FOLDER).join("CMakeLists.txt").exists() {
        return Vec::new();
    }
    build_steps()
}

/// The commands that build the parts folder on its own: configure, then build.
pub fn build_steps() -> Vec<Vec<String>> {
    vec![
        vec!["cmake".into(), "-S".into(), FOLDER.into(), "-B".into(), format!("{FOLDER}/build"), "-DCMAKE_BUILD_TYPE=Release".into()],
        vec!["cmake".into(), "--build".into(), format!("{FOLDER}/build")],
    ]
}

/// What declaring a native part writes into its folder, by path in the
/// folder: a CMake file and C++ that build and keep the part's values,
/// drawing nothing yet, and the header. Clarity never writes over the
/// first two.
pub fn starter(part: &PartDecl) -> Vec<(String, String)> {
    let n = &part.name.name;
    let mut cmake = String::new();
    let _ = write!(
        cmake,
        "# The library that draws custom part {n}. Clarity builds it into the program;\n# the target keeps the part's name.\ncmake_minimum_required(VERSION 3.20)\nproject({n} LANGUAGES CXX)\n\nset(CMAKE_CXX_STANDARD 17)\nadd_library({n} STATIC {n}.cpp)\ntarget_include_directories({n} PUBLIC ${{CMAKE_CURRENT_SOURCE_DIR}})\n\n# To draw with SFML, install it (Debian: libsfml-dev) and use these lines:\n# find_package(SFML 2.6 COMPONENTS graphics window system REQUIRED)\n# target_link_libraries({n} PRIVATE sfml-graphics sfml-window sfml-system)\n"
    );
    let mut cpp = String::new();
    let _ = writeln!(cpp, "// Custom part {n}: draws in the native window Clarity gives it. Keep the");
    let _ = writeln!(cpp, "// functions of clarity/{n}.h, which follow the part's declaration.\n");
    let _ = writeln!(cpp, "#include <string>\n\n#include \"clarity/{n}.h\"\n");
    let _ = writeln!(cpp, "struct {n}Part {{");
    let _ = writeln!(cpp, "    {n}Host* host;");
    let _ = writeln!(cpp, "    void* window;");
    let _ = writeln!(cpp, "    int width;");
    let _ = writeln!(cpp, "    int height;");
    let _ = writeln!(cpp, "    bool enabled = true;");
    let field = |name: &str, value: CValue| match value {
        CValue::Integer => format!("    int64_t {name} = 0;"),
        CValue::Bool => format!("    bool {name} = false;"),
        CValue::Text => format!("    std::string {name};"),
    };
    if let Some(binds) = &part.binds {
        let _ = writeln!(cpp, "{}", field("value", c_value(binds)));
    }
    for property in &part.properties {
        let _ = writeln!(cpp, "{}", field(&property.name.name, c_value(&property.ty)));
    }
    let _ = writeln!(cpp, "}};\n");
    let _ = writeln!(cpp, "{n}Part* {n}_create(void* parent, int width, int height, {n}Host* host) {{");
    let _ = writeln!(cpp, "    return new {n}Part{{host, parent, width, height}};\n}}\n");
    let _ = writeln!(cpp, "void {n}_resize({n}Part* part, int width, int height) {{\n    part->width = width;\n    part->height = height;\n}}\n");
    let _ = writeln!(cpp, "void {n}_frame({n}Part* part) {{\n    // Draw here, about 60 times a second, and read the mouse and keys.\n    (void)part;\n}}\n");
    let _ = writeln!(cpp, "void {n}_destroy({n}Part* part) {{\n    delete part;\n}}\n");
    let _ = writeln!(cpp, "void {n}_set_enabled({n}Part* part, bool enabled) {{\n    part->enabled = enabled;\n}}\n");
    if let Some(binds) = &part.binds {
        let _ = writeln!(cpp, "void {n}_set_value({n}Part* part, {} value) {{\n    part->value = value;\n}}\n", c_type(c_value(binds)));
    }
    for property in &part.properties {
        let p = &property.name.name;
        let _ = writeln!(cpp, "void {n}_set_{p}({n}Part* part, {} {p}) {{\n    part->{p} = {p};\n}}\n", c_type(c_value(&property.ty)));
    }
    for hear in &part.hears {
        let names: Vec<String> = hear.params.iter().map(|p| format!("(void){};", p.name.name)).collect();
        let _ = writeln!(cpp, "void {n}_{}({n}Part* part{}) {{\n    (void)part;\n    {}\n}}\n", hear.name.name, params(&hear.params), names.join(" "));
    }
    vec![
        (native::BUILD_FILE.to_string(), cmake),
        (format!("{n}.cpp"), cpp.trim_end().to_string() + "\n"),
        (format!("{}/{n}.h", native::OWN_FOLDER), header(part)),
    ]
}

/// The declaration of the custom part of kind `kind`.
pub(crate) fn declaration<'a>(project: &'a Project, kind: &str) -> Option<&'a PartDecl> {
    project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == kind)
}

/// A custom part's property as code reads it when no handler set it: the
/// value written on the part's line, else the declared default, else the
/// empty value of its type; as JSON, which each runtime reads as the type.
pub(crate) fn fallback_json(project: &Project, kind: &str, written: &std::collections::BTreeMap<String, clarity_spec::model::Literal>, property: &str) -> String {
    use clarity_spec::code::ExprKind;
    use clarity_spec::model::Literal;
    let Some(declared) = declaration(project, kind).and_then(|d| d.properties.iter().find(|p| p.name.name == property)) else {
        return "null".into();
    };
    let whole = matches!(declared.ty.kind, TypeRefKind::Integer | TypeRefKind::Duration);
    let number = |n: &str| if whole { n.to_string() } else { serde_json::Value::String(n.to_string()).to_string() };
    let text = |t: &str| serde_json::Value::String(t.to_string()).to_string();
    if let Some(value) = written.get(property) {
        return match value {
            Literal::Number(n) => number(n),
            Literal::Str(s) | Literal::Ident(s) => text(s),
            Literal::Bool(b) => b.to_string(),
        };
    }
    if let Some(default) = &declared.default {
        match &default.kind {
            ExprKind::Number { value } => return number(value),
            ExprKind::Unary { operand, .. } if matches!(&operand.kind, ExprKind::Number { .. }) => {
                if let ExprKind::Number { value } = &operand.kind {
                    return number(&format!("-{value}"));
                }
            }
            ExprKind::Str { value } => return text(value),
            ExprKind::Bool { value } => return value.to_string(),
            ExprKind::Name { name } => return text(name),
            _ => {}
        }
    }
    match declared.ty.kind {
        TypeRefKind::Integer | TypeRefKind::Duration => "0".into(),
        TypeRefKind::Decimal | TypeRefKind::Money => text("0"),
        TypeRefKind::Bool => "false".into(),
        TypeRefKind::Date | TypeRefKind::DateTime => "null".into(),
        _ => text(""),
    }
}

/// The handlers of `page` wired to a custom part's tells: they run with the
/// tell's values, as the handler an answer goes to does.
pub(crate) fn told_handlers(page: Option<&clarity_spec::model::Page>) -> Vec<String> {
    fn walk(widgets: &[clarity_spec::model::Widget], found: &mut Vec<String>) {
        for widget in widgets {
            for tell in &widget.props.tells {
                if !found.contains(&tell.handler) {
                    found.push(tell.handler.clone());
                }
            }
            walk(&widget.children, found);
            for page in &widget.pages {
                walk(&page.children, found);
            }
        }
    }
    let mut found = Vec::new();
    if let Some(page) = page {
        walk(&page.widgets, &mut found);
        for sidebar in &page.frame.sidebars {
            walk(&sidebar.widgets, &mut found);
        }
    }
    found
}

/// Writes each native part's header from its declaration into the part's
/// folder in the project in `dir` (when it changed), and the starter into a
/// folder that has no build file yet. Gives the files written.
pub fn write_part_folders(dir: &std::path::Path, project: &Project) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut written = Vec::new();
    // A .NET part's adapter, written once.
    for part in crate::csharp::parts::dotnet_parts(project) {
        let n = &part.name.name;
        let adapter = dir.join(native::adapter_folder(n)).join(format!("{n}.cs"));
        if !adapter.exists() {
            if let Some(parent) = adapter.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&adapter, crate::csharp::parts::adapter_starter(project, part))?;
            written.push(adapter);
        }
    }
    // A native, Rust or Go part: its starter where the folder has none, and
    // the file of Clarity's (the header, `src/clarity.rs`, `clarity.go`) always.
    for part in project.files.iter().flat_map(|f| &f.parts) {
        let (folder, files, manifest, own): (&String, Vec<(String, String)>, &str, &str) = match &part.library {
            Some(PartLibrary::Native { folder }) => (folder, starter(part), native::BUILD_FILE, "clarity/"),
            Some(PartLibrary::Crate { folder }) => (folder, crate::rust::parts::starter(part), "Cargo.toml", "src/clarity.rs"),
            Some(PartLibrary::Go { folder }) => (folder, crate::go::parts::starter(part), "go.mod", "clarity.go"),
            _ => continue,
        };
        let root = dir.join(folder);
        let fresh = !root.join(manifest).exists();
        for (path, text) in files {
            let own = path.starts_with(own);
            if !own && !fresh {
                continue;
            }
            let target = root.join(&path);
            if std::fs::read_to_string(&target).ok().as_deref() == Some(text.as_str()) {
                continue;
            }
            // A file of the developer's is never written over.
            if !own && target.exists() {
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, text)?;
            written.push(target);
        }
    }
    // A library's adapter, written once, and the file of Clarity's always (DESIGN 5.21).
    let cpp = project_is_cpp(dir);
    for library in crate::libraries::libraries(project) {
        let root = dir.join(native::library_folder(&library.name.name));
        let Some(source) = &library.source else { continue };
        let fresh = !root.join(native::library_manifest(&library.name.name, source)).exists();
        let own = crate::libraries::own_file(library, cpp);
        for (path, text) in crate::libraries::starter(project, library, cpp, crate::go::runtime_version()) {
            let mine = path == own;
            let target = root.join(&path);
            if (!mine && (!fresh || target.exists())) || std::fs::read_to_string(&target).ok().as_deref() == Some(text.as_str()) {
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, text)?;
            written.push(target);
        }
        // A C# library's generated half lives in the project too, for the editor.
        if matches!(source, clarity_spec::code::LibrarySource::Nuget { .. } | clarity_spec::code::LibrarySource::Dll { .. }) {
            let target = root.join(&own);
            let text = crate::libraries::csharp_half(project, library);
            if std::fs::read_to_string(&target).ok().as_deref() != Some(text.as_str()) {
                std::fs::create_dir_all(&root)?;
                std::fs::write(&target, text)?;
                written.push(target);
            }
        }
    }
    Ok(written)
}

/// Whether the project in `dir` is on the C++ stack, which a CMake library's
/// adapter is then written for (else C).
fn project_is_cpp(dir: &std::path::Path) -> bool {
    let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    clarity_project::load(&file).settings.is_some_and(|s| s.stack.backend == clarity_project::Backend::Cpp)
}

/// The starter adapter of .NET part `name`, as declaring it writes it.
pub fn dotnet_adapter(project: &Project, name: &str) -> Option<String> {
    let part = crate::csharp::parts::dotnet_parts(project).into_iter().find(|p| p.name.name == name)?;
    Some(crate::csharp::parts::adapter_starter(project, part))
}

/// What declaring a part writes into its folder, by its library: C or C++
/// with CMake, a Rust crate, or a Go module.
pub fn starter_for(part: &PartDecl) -> Vec<(String, String)> {
    match &part.library {
        Some(PartLibrary::Native { .. }) => starter(part),
        Some(PartLibrary::Crate { .. }) => crate::rust::parts::starter(part),
        Some(PartLibrary::Go { .. }) => crate::go::parts::starter(part),
        _ => Vec::new(),
    }
}
