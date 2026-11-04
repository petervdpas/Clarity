//! Reading a custom part's own code without building it (DESIGN 5.19): each
//! stack's scan (`code_mismatches` in rust/parts.rs, go/parts.rs,
//! c/parts.rs, cpp/parts.rs, csharp/parts.rs) checks that the code still
//! implements what Clarity's file in the part's folder asks, and uses
//! nothing Clarity no longer has. Comments and strings are blanked first,
//! so a name in either does not count.

use clarity_spec::Project;
use clarity_spec::code::{PartDecl, PartLibrary};

/// How a language writes comments and strings. C covers C++.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    C,
    Rust,
    Go,
    CSharp,
}

/// One file of a part's own code, its comments and strings blanked.
pub struct Source {
    /// Where it is in the project, as a person finds it (`parts/Dice/src/lib.rs`).
    pub path: String,
    pub code: String,
}

impl Source {
    /// `path:line` of byte `at` of the code.
    pub fn at(&self, at: usize) -> String {
        format!("{}:{}", self.path, self.code[..at].matches('\n').count() + 1)
    }
}

/// The folder of the part in the project.
fn folder(part: &PartDecl) -> String {
    match &part.library {
        Some(PartLibrary::Native { folder } | PartLibrary::Crate { folder } | PartLibrary::Go { folder }) => folder.trim_end_matches('/').to_string(),
        _ => clarity_spec::native::adapter_folder(&part.name.name),
    }
}

/// The part's own files that `keep` takes (by the path in its folder), blanked.
pub fn sources(project: &Project, part: &PartDecl, lang: Lang, keep: impl Fn(&str) -> bool) -> Vec<Source> {
    let folder = folder(part);
    project
        .part_files
        .iter()
        .flatten()
        .filter(|f| f.part == part.name.name && keep(&f.path))
        .filter_map(|f| std::str::from_utf8(&f.bytes).ok().map(|text| Source { path: format!("{folder}/{}", f.path), code: blank(text, lang) }))
        .collect()
}

pub fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `text` with every comment and string (and char literal) turned to
/// spaces, its lines kept, so names and line numbers stay where they were.
pub fn blank(text: &str, lang: Lang) -> String {
    let c: Vec<char> = text.chars().collect();
    let n = c.len();
    let mut out = String::with_capacity(text.len());
    let at = |i: usize, s: &str| s.chars().enumerate().all(|(k, ch)| c.get(i + k) == Some(&ch));
    let ident_before = |i: usize| i > 0 && is_ident(c[i - 1]);
    let mut i = 0;
    // Blanks c[from..to], keeping its line ends.
    let put = |out: &mut String, from: usize, to: usize| {
        for ch in &c[from..to.min(n)] {
            out.push(if *ch == '\n' { '\n' } else { ' ' });
        }
    };
    while i < n {
        let ch = c[i];
        let start = i;
        if at(i, "//") {
            while i < n && c[i] != '\n' {
                i += 1;
            }
            put(&mut out, start, i);
            continue;
        }
        if at(i, "/*") {
            // Rust's block comments nest.
            let mut depth = 0;
            while i < n {
                if at(i, "/*") {
                    depth += 1;
                    i += 2;
                } else if at(i, "*/") {
                    i += 2;
                    if lang != Lang::Rust || depth == 1 {
                        break;
                    }
                    depth -= 1;
                } else {
                    i += 1;
                }
            }
            put(&mut out, start, i);
            continue;
        }
        // Rust's raw strings: r"..", r#".."#, br"..".
        if lang == Lang::Rust && ch == 'r' && (!ident_before(i) || (c[i - 1] == 'b' && !ident_before(i - 1))) {
            let mut j = i + 1;
            while j < n && c[j] == '#' {
                j += 1;
            }
            if j < n && c[j] == '"' {
                let hashes = j - i - 1;
                let end: String = std::iter::once('"').chain(std::iter::repeat_n('#', hashes)).collect();
                i = j + 1;
                while i < n && !at(i, &end) {
                    i += 1;
                }
                i = (i + end.chars().count()).min(n);
                put(&mut out, start, i);
                continue;
            }
        }
        // C++'s raw strings: R"d(..)d".
        if lang == Lang::C && ch == 'R' && at(i + 1, "\"") && !ident_before(i) {
            let mut j = i + 2;
            while j < n && c[j] != '(' && c[j] != '\n' {
                j += 1;
            }
            if j < n && c[j] == '(' {
                let end: String = std::iter::once(')').chain(c[i + 2..j].iter().copied()).chain(std::iter::once('"')).collect();
                i = j + 1;
                while i < n && !at(i, &end) {
                    i += 1;
                }
                i = (i + end.chars().count()).min(n);
                put(&mut out, start, i);
                continue;
            }
        }
        // C#'s raw strings ("""..""") and verbatim ones (@"..", $@"..", @$"..").
        if lang == Lang::CSharp && at(i, "\"\"\"") {
            i += 3;
            while i < n && !at(i, "\"\"\"") {
                i += 1;
            }
            i = (i + 3).min(n);
            put(&mut out, start, i);
            continue;
        }
        if lang == Lang::CSharp && (at(i, "@\"") || at(i, "$@\"") || at(i, "@$\"")) {
            i += if ch == '@' && c[i + 1] == '"' { 2 } else { 3 };
            while i < n {
                if at(i, "\"\"") {
                    i += 2;
                } else if c[i] == '"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            put(&mut out, start, i);
            continue;
        }
        // Go's raw strings.
        if lang == Lang::Go && ch == '`' {
            i += 1;
            while i < n && c[i] != '`' {
                i += 1;
            }
            i = (i + 1).min(n);
            put(&mut out, start, i);
            continue;
        }
        if ch == '"' || ch == '\'' {
            // A Rust lifetime ('a, 'static) is not a char.
            if lang == Lang::Rust && ch == '\'' && c.get(i + 1) != Some(&'\\') && c.get(i + 2) != Some(&'\'') {
                out.push(ch);
                i += 1;
                continue;
            }
            i += 1;
            while i < n && c[i] != ch && c[i] != '\n' {
                i += if c[i] == '\\' { 2 } else { 1 };
            }
            i = (i + 1).min(n);
            put(&mut out, start, i);
            continue;
        }
        out.push(ch);
        i += 1;
    }
    out
}

/// The byte offsets where `word` stands as a whole name in `code`.
pub fn find_word(code: &str, word: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(k) = code[from..].find(word) {
        let at = from + k;
        let end = at + word.len();
        let before = code[..at].chars().next_back().is_some_and(is_ident);
        let after = code[end..].chars().next().is_some_and(is_ident);
        if !before && !after {
            found.push(at);
        }
        from = end;
    }
    found
}

/// The code after byte `at`, its spaces skipped.
pub fn after(code: &str, at: usize) -> &str {
    code[at..].trim_start()
}

/// The code before byte `at`, its spaces skipped.
pub fn before(code: &str, at: usize) -> &str {
    code[..at].trim_end()
}

/// The name that starts `code`, if one does.
pub fn first_word(code: &str) -> &str {
    let end = code.find(|c: char| !is_ident(c)).unwrap_or(code.len());
    &code[..end]
}

/// The byte just past the bracket that closes the one at `open`.
pub fn close(code: &str, open: usize) -> Option<usize> {
    let (o, c) = match code[open..].chars().next()? {
        '(' => ('(', ')'),
        '{' => ('{', '}'),
        '[' => ('[', ']'),
        _ => return None,
    };
    let mut depth = 0;
    for (k, ch) in code[open..].char_indices() {
        if ch == o {
            depth += 1;
        } else if ch == c {
            depth -= 1;
            if depth == 0 {
                return Some(open + k + 1);
            }
        }
    }
    None
}

/// Where `name` is a function with its body in C-like code: `name(..) {`,
/// with `const`, `override`, `final` or `noexcept` allowed before the body.
pub fn defines(code: &str, name: &str) -> Option<usize> {
    find_word(code, name).into_iter().find(|&at| {
        let rest = &code[at + name.len()..];
        let open = at + name.len() + (rest.len() - rest.trim_start().len());
        if !code[open..].starts_with('(') {
            return false;
        }
        let Some(end) = close(code, open) else { return false };
        let mut tail = after(code, end);
        loop {
            let word = first_word(tail);
            if matches!(word, "const" | "override" | "final" | "noexcept") {
                tail = tail[word.len()..].trim_start();
            } else {
                break;
            }
        }
        tail.starts_with('{')
    })
}

/// Where `name` is called or named as a function (`name(`), not after a `.`.
pub fn names_fn(code: &str, name: &str) -> Option<usize> {
    find_word(code, name).into_iter().find(|&at| after(code, at + name.len()).starts_with('(') && !before(code, at).ends_with('.'))
}

/// Where `word` is used as a name of its own: not a member (`x.word`) and
/// not declared by the code itself (`class word`, `struct word`, `type word`).
pub fn uses_name(code: &[Source], word: &str) -> Option<(usize, usize)> {
    let declared = code.iter().any(|s| {
        find_word(&s.code, word).into_iter().any(|at| matches!(before(&s.code, at).rsplit(|c: char| !is_ident(c)).next(), Some("class" | "struct" | "enum" | "interface" | "record" | "type" | "typedef" | "fn" | "func")))
    });
    if declared {
        return None;
    }
    code.iter().enumerate().find_map(|(k, s)| find_word(&s.code, word).into_iter().find(|&at| !before(&s.code, at).ends_with('.')).map(|at| (k, at)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_strings_do_not_count() {
        let code = blank("a // b\n/* c\n d */ e \"f\\\"g\" 'h' i", Lang::C);
        assert_eq!(find_word(&code, "b"), Vec::<usize>::new());
        assert!(find_word(&code, "d").is_empty() && find_word(&code, "f").is_empty() && find_word(&code, "h").is_empty());
        assert_eq!(code.lines().count(), 3);
        assert!(!find_word(&code, "e").is_empty() && !find_word(&code, "i").is_empty());
        let rust = blank("fn x<'a>(s: &'a str) { let t = r#\"y\"#; /* /* z */ w */ v }", Lang::Rust);
        assert!(find_word(&rust, "y").is_empty() && find_word(&rust, "w").is_empty());
        assert!(!find_word(&rust, "v").is_empty() && !find_word(&rust, "str").is_empty());
        let go = blank("a := `b` + c", Lang::Go);
        assert!(find_word(&go, "b").is_empty() && !find_word(&go, "c").is_empty());
        let cs = blank("var a = @\"b\"\"c\"; d", Lang::CSharp);
        assert!(find_word(&cs, "c").is_empty() && !find_word(&cs, "d").is_empty());
    }

    #[test]
    fn a_definition_has_a_body() {
        let code = blank("void A_draw(int x);\nvoid A_draw(int x) const {}\nint y = A_key(1);", Lang::C);
        assert_eq!(defines(&code, "A_draw").map(|at| code[..at].matches('\n').count()), Some(1));
        assert!(defines(&code, "A_key").is_none());
    }
}
