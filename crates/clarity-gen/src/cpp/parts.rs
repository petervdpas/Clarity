//! Custom parts of a C++ program (DESIGN 5.19 and 7.9): a part is a library
//! of the project (`native "parts/OrbField"`, C++ with its CMakeLists.txt)
//! that implements the class Clarity writes from the declaration into its
//! `clarity/<Name>.h`: a control of Clarity.UI, drawing on the canvas the
//! library gives (the runtime's `ui/ui.hpp`). The program builds each part
//! into itself and registers its kind with Clarity.UI by its name.

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

fn cpp_type(kind: Kind) -> &'static str {
    match kind {
        Kind::Integer => "int64_t",
        Kind::Bool => "bool",
        Kind::Text => "std::string",
    }
}

/// A value as a parameter: text by reference.
fn given_type(kind: Kind) -> &'static str {
    match kind {
        Kind::Text => "const std::string&",
        other => cpp_type(other),
    }
}

/// The expression that reads the JSON value `json` as the part takes it.
fn read(kind: Kind, json: &str) -> String {
    match kind {
        Kind::Integer => format!("integer_of({json})"),
        Kind::Bool => format!("{json}.is_boolean() && {json}.get<bool>()"),
        Kind::Text => format!("text_of({json})"),
    }
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn params(list: &[Param]) -> String {
    list.iter().map(|p| format!(", {} {}", given_type(kind(&p.ty)), snake(&p.name.name))).collect()
}

/// The project's C++ parts, in the order they are declared.
pub fn parts(project: &Project) -> Vec<&PartDecl> {
    crate::parts::native_parts(project)
}

/// `parts/<Name>/clarity/<Name>.h`: the class the part's library implements,
/// typed from the declaration, and the function that makes one.
pub fn header(part: &PartDecl) -> String {
    let n = &part.name.name;
    let mut out = String::new();
    let _ = writeln!(out, "// Custom part {n} (DESIGN 5.19 and 7.9): the class its library implements, a");
    let _ = writeln!(out, "// control of Clarity.UI. Written by Clarity from the part's declaration, and");
    let _ = writeln!(out, "// written again when the declaration changes; the files beside this folder");
    let _ = writeln!(out, "// are yours.");
    let _ = writeln!(out, "//");
    let _ = writeln!(out, "// Every call comes on the screen's thread. The part draws on the canvas");
    let _ = writeln!(out, "// Clarity.UI gives, takes the pointer and keys (clarity::ui::CustomPart in");
    let _ = writeln!(out, "// ui/ui.hpp) and tells its window through the cx of the call. An integer is");
    let _ = writeln!(out, "// int64_t and a bool is bool; every other value (text, a decimal, money, a");
    let _ = writeln!(out, "// date, an enum) is text in its plain form.");
    let _ = writeln!(out, "\n#pragma once\n\n#include <cstdint>\n#include <memory>\n#include <string>\n#include <string_view>\n\n#include <nlohmann/json.hpp>\n\n#include \"ui/ui.hpp\"\n");
    let _ = writeln!(out, "class {n}Part : public clarity::ui::CustomPart {{");
    let _ = writeln!(out, "public:");
    let _ = writeln!(out, "    /// Whether the person can use it, as code sets `enabled`.");
    let _ = writeln!(out, "    virtual void show_enabled(bool enabled, clarity::ui::Cx& cx) {{\n        (void)enabled;\n        cx.invalidate();\n    }}");
    if let Some(binds) = &part.binds {
        let _ = writeln!(out, "    /// The value it shows, from what it binds; given when its window starts and when it changed.");
        let _ = writeln!(out, "    virtual void show_value({} value, clarity::ui::Cx& cx) = 0;", given_type(kind(binds)));
    }
    for property in &part.properties {
        let p = snake(&property.name.name);
        let _ = writeln!(out, "    /// Property {}, as the window or code sets it.", property.name.name);
        let _ = writeln!(out, "    virtual void show_{p}({} {p}, clarity::ui::Cx& cx) = 0;", given_type(kind(&property.ty)));
    }
    for hear in &part.hears {
        let _ = writeln!(out, "    /// What code tells it: {n}.{}.", hear.name.name);
        let _ = writeln!(out, "    virtual void on_{}(clarity::ui::Cx& cx{}) = 0;", snake(&hear.name.name), params(&hear.params));
    }
    // set and hear, read into the typed calls.
    let _ = writeln!(out, "\n    void set(std::string_view name, std::string_view value, clarity::ui::Cx& cx) final {{");
    let _ = writeln!(out, "        nlohmann::json given = nlohmann::json::parse(value, nullptr, false);");
    let _ = writeln!(out, "        if (name == \"enabled\") show_enabled(given.is_boolean() && given.get<bool>(), cx);");
    if let Some(binds) = &part.binds {
        let _ = writeln!(out, "        else if (name == \"value\") show_value({}, cx);", read(kind(binds), "given"));
    }
    for property in &part.properties {
        let _ = writeln!(out, "        else if (name == \"{}\") show_{}({}, cx);", property.name.name, snake(&property.name.name), read(kind(&property.ty), "given"));
    }
    let _ = writeln!(out, "    }}\n");
    let _ = writeln!(out, "    void hear(std::string_view hear, std::string_view values, clarity::ui::Cx& cx) final {{");
    let _ = writeln!(out, "        nlohmann::json given = nlohmann::json::parse(values, nullptr, false);");
    let _ = writeln!(out, "        if (!given.is_array()) given = nlohmann::json::array();");
    let _ = writeln!(out, "        (void)given;");
    for (i, hear) in part.hears.iter().enumerate() {
        let keyword = if i == 0 { "if" } else { "else if" };
        let args: String = hear.params.iter().enumerate().map(|(k, p)| format!(", {}", read(kind(&p.ty), &format!("at(given, {k})")))).collect();
        let _ = writeln!(out, "        {keyword} (hear == \"{}\") on_{}(cx{args});", hear.name.name, snake(&hear.name.name));
    }
    if part.hears.is_empty() {
        let _ = writeln!(out, "        (void)hear;\n        (void)cx;");
    }
    let _ = writeln!(out, "    }}\n");
    let _ = writeln!(out, "protected:");
    if let Some(binds) = &part.binds {
        let _ = writeln!(out, "    /// The person changed the value: what the part binds takes it (one edit, which Ctrl+Z takes back) and its change handler runs.");
        let _ = writeln!(out, "    void changed(clarity::ui::Cx& cx, {} value) {{ cx.change(nlohmann::json(value).dump()); }}", given_type(kind(binds)));
    }
    for tell in &part.tells {
        let names: Vec<String> = tell.params.iter().map(|p| snake(&p.name.name)).collect();
        let _ = writeln!(out, "    /// Tells {}: the handler wired to it runs with these values.", tell.name.name);
        let _ = writeln!(
            out,
            "    void {}(clarity::ui::Cx& cx{}) {{ cx.tell(\"{}\", nlohmann::json::array({{{}}}).dump()); }}",
            snake(&tell.name.name),
            params(&tell.params),
            tell.name.name,
            names.join(", ")
        );
    }
    let _ = writeln!(out, "\nprivate:");
    let _ = writeln!(out, "    static int64_t integer_of(const nlohmann::json& v) {{\n        if (v.is_number()) return v.get<int64_t>();\n        if (v.is_string()) return std::strtoll(v.get<std::string>().c_str(), nullptr, 10);\n        return 0;\n    }}");
    let _ = writeln!(out, "    static std::string text_of(const nlohmann::json& v) {{\n        if (v.is_string()) return v.get<std::string>();\n        if (v.is_null() || v.is_discarded()) return {{}};\n        return v.dump();\n    }}");
    let _ = writeln!(out, "    static nlohmann::json at(const nlohmann::json& values, size_t i) {{ return i < values.size() ? values[i] : nlohmann::json(); }}");
    let _ = writeln!(out, "}};\n");
    let _ = writeln!(out, "/// Makes a new {n}; the part's library implements it.");
    let _ = writeln!(out, "std::unique_ptr<{n}Part> make_{n}();");
    out.replace("#include <cstdint>\n", "#include <cstdint>\n#include <cstdlib>\n")
}

/// `src/parts.cpp`: every kind of part of the program, registered with Clarity.UI by its name.
pub fn glue(program: &str, parts: &[&PartDecl]) -> String {
    let includes: String = parts.iter().map(|p| format!("#include \"clarity/{}.h\"\n", p.name.name)).collect();
    let registers: String = parts
        .iter()
        .map(|p| format!("    clarity::ui::register_part(\"{n}\", [] {{ return std::unique_ptr<clarity::ui::CustomPart>(make_{n}()); }});\n", n = p.name.name))
        .collect();
    format!(
        "{}\n// The program's custom parts, controls of Clarity.UI (DESIGN 7.9).\n\n{includes}\nnamespace {program} {{\n\n/// Tells Clarity.UI about every kind of part the program has, by its name.\nvoid register_parts() {{\n{registers}}}\n\n}}  // namespace {program}\n",
        super::HEADER
    )
}

/// The lines of the program's CMakeLists.txt that build each part into it.
pub fn cmake_lines(program: &str, parts: &[&PartDecl]) -> String {
    if parts.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n# The custom parts (DESIGN 5.19), each a library of its own folder, built into\n# the program; they draw with Clarity.UI (the runtime's ui/ui.hpp).\n");
    for part in parts {
        let n = &part.name.name;
        let _ = writeln!(out, "add_subdirectory(parts/{n})\ntarget_link_libraries({n} PUBLIC clarity_desktop)\ntarget_link_libraries({program} PRIVATE {n})");
    }
    out
}

/// The parts' folders in the output: each part's own files and its header written from the declaration.
pub fn out_files(project: &Project) -> Vec<OutFile> {
    let mut files = Vec::new();
    for part in parts(project) {
        let n = &part.name.name;
        // The folder's own files; Clarity's header is written from the declaration below.
        let own = format!("{}/", native::OWN_FOLDER);
        for file in project.part_files.iter().flatten().filter(|f| f.part == *n && !f.path.starts_with(&own)) {
            files.push(OutFile { path: format!("{}/{n}/{}", crate::parts::FOLDER, file.path), text: String::new(), bytes: Some(file.bytes.clone().into()) });
        }
        files.push(OutFile { path: format!("{}/{n}/{}/{n}.h", crate::parts::FOLDER, native::OWN_FOLDER), text: header(part), bytes: None });
    }
    files
}

/// What declaring a part in a C++ project writes into its folder: a CMake
/// file and C++ that keep the part's values and draw a frame, and the
/// header. Clarity never writes over the first two.
pub fn starter(part: &PartDecl) -> Vec<(String, String)> {
    let n = &part.name.name;
    let cmake = format!(
        "# The library of custom part {n}, a control of Clarity.UI. Clarity builds it\n# into the program; the target keeps the part's name.\ncmake_minimum_required(VERSION 3.20)\nproject({n} LANGUAGES CXX)\n\nset(CMAKE_CXX_STANDARD 20)\nadd_library({n} STATIC {n}.cpp)\ntarget_include_directories({n} PUBLIC ${{CMAKE_CURRENT_SOURCE_DIR}})\n"
    );
    let mut cpp = String::new();
    let _ = writeln!(cpp, "// Custom part {n}: draws on the canvas Clarity.UI gives it. Keep the");
    let _ = writeln!(cpp, "// functions of clarity/{n}.h, which follow the part's declaration.\n");
    let _ = writeln!(cpp, "#include \"clarity/{n}.h\"\n");
    let _ = writeln!(cpp, "namespace {{\n\nclass {n} final : public {n}Part {{\npublic:");
    let field = |name: &str, k: Kind| match k {
        Kind::Integer => format!("    int64_t {name} = 0;"),
        Kind::Bool => format!("    bool {name} = false;"),
        Kind::Text => format!("    std::string {name};"),
    };
    let _ = writeln!(cpp, "    void draw(clarity::ui::Canvas& canvas, double x, double y, double width, double height) override {{");
    let _ = writeln!(cpp, "        canvas.stroke(x, y, width, height, canvas.look(\"border\"), 1, 4);\n    }}");
    if let Some(binds) = &part.binds {
        let _ = writeln!(cpp, "\n    void show_value({} value, clarity::ui::Cx& cx) override {{\n        shown = value;\n        cx.invalidate();\n    }}", given_type(kind(binds)));
    }
    for property in &part.properties {
        let p = snake(&property.name.name);
        let _ = writeln!(cpp, "\n    void show_{p}({} {p}, clarity::ui::Cx& cx) override {{\n        {p}_ = {p};\n        cx.invalidate();\n    }}", given_type(kind(&property.ty)));
    }
    for hear in &part.hears {
        let names: Vec<String> = hear.params.iter().map(|p| format!("(void){};", snake(&p.name.name))).collect();
        let _ = writeln!(cpp, "\n    void on_{}(clarity::ui::Cx& cx{}) override {{\n        (void)cx;\n        {}\n    }}", snake(&hear.name.name), params(&hear.params), names.join(" "));
    }
    let _ = writeln!(cpp, "\nprivate:");
    if let Some(binds) = &part.binds {
        let _ = writeln!(cpp, "{}", field("shown", kind(binds)));
    }
    for property in &part.properties {
        let _ = writeln!(cpp, "{}", field(&format!("{}_", snake(&property.name.name)), kind(&property.ty)));
    }
    let _ = writeln!(cpp, "}};\n\n}}  // namespace\n");
    let _ = writeln!(cpp, "std::unique_ptr<{n}Part> make_{n}() {{\n    return std::make_unique<{n}>();\n}}");
    vec![(native::BUILD_FILE.to_string(), cmake), (format!("{n}.cpp"), cpp), (format!("{}/{n}.h", native::OWN_FOLDER), header(part))]
}
