//! The files a folder holds, as a file browser writes them
//! (`extensions "*.md *.csv"` on a folder field): `*.md` the files with
//! that extension, ignoring case, or `*.*` alone for any file. Hidden files
//! and folders, whose name starts with a dot, count only with the folder's
//! `hidden` option. Without extensions a folder holds any file. Every
//! runtime matches alike (conformance/files.json).

/// The patterns of extensions as written: `.png, .jpg, .gif` (or the
/// pattern form `*.png *.jpg`), separated by commas or spaces, or `*.*` for
/// any file. `.png` is the pattern `*.png`; anything else stays as written,
/// for `patterns_problem` to say what is wrong with it.
pub fn patterns_of(written: &str) -> Vec<String> {
    written
        .split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(|w| match w.strip_prefix('.') {
            Some(ext) if !ext.is_empty() && !ext.contains(['*', '.']) => format!("*.{ext}"),
            _ => w.to_string(),
        })
        .collect()
}

/// The patterns as a part's or field's line writes them: `.png, .jpg`, or `*.*`.
pub fn written_of(patterns: &[String]) -> String {
    patterns.iter().map(|p| if p == "*.*" { p.clone() } else { p.strip_prefix('*').unwrap_or(p).to_string() }).collect::<Vec<_>>().join(", ")
}

/// What is wrong with the extensions of a folder: none when they read.
pub fn patterns_problem(patterns: &[String]) -> Option<String> {
    if patterns.is_empty() {
        return Some("name the files it holds, as in 'extensions \".md, .csv\"'".to_string());
    }
    for pattern in patterns {
        let fits = pattern == "*.*" || pattern.strip_prefix("*.").is_some_and(|ext| !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
        if !fits {
            return Some(format!("'{pattern}' is no extension of files; write '.md', or '*.*' for any file"));
        }
    }
    // Any file is any file: it stands without extensions beside it.
    if patterns.len() > 1 && patterns.iter().any(|p| p == "*.*") {
        return Some("'*.*' takes any file, so no other extension goes with it".to_string());
    }
    None
}

/// Whether a file's name (`Notes.MD`, `.env`) is one a folder takes: by
/// its extensions (none, or `*.*`: any file), a hidden one only with `hidden`.
pub fn matches(patterns: &[String], hidden: bool, name: &str) -> bool {
    if name.starts_with('.') && !hidden {
        return false;
    }
    if patterns.is_empty() || patterns.iter().any(|p| p == "*.*") {
        return true;
    }
    let extension = std::path::Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase());
    patterns.iter().any(|pattern| pattern.strip_prefix("*.").is_some_and(|ext| extension.as_deref() == Some(ext.to_lowercase().as_str())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(written: &str) -> Vec<String> {
        written.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn extensions_take_their_files_and_hidden_ones_only_when_hidden() {
        assert!(matches(&p("*.md *.txt"), false, "Notes.MD") && matches(&p("*.md *.txt"), false, "a.txt") && !matches(&p("*.md"), false, "a.csv"));
        assert!(matches(&p("*.*"), false, "README") && !matches(&p("*.*"), false, ".env") && matches(&p("*.*"), true, ".env"));
        assert!(matches(&[], false, "a.md") && !matches(&[], false, ".hidden") && matches(&[], true, ".hidden"));
        assert!(!matches(&p("*.md"), false, ".notes.md") && matches(&p("*.md"), true, ".notes.md"));
    }

    #[test]
    fn extensions_are_written_with_a_dot_and_commas_or_as_patterns() {
        assert_eq!(patterns_of(".png, .jpg, .gif"), ["*.png", "*.jpg", "*.gif"]);
        assert_eq!(patterns_of(".png,.jpg"), ["*.png", "*.jpg"]);
        assert_eq!(patterns_of("*.md *.csv"), ["*.md", "*.csv"]);
        assert_eq!(patterns_of("*.*"), ["*.*"]);
        assert_eq!(written_of(&patterns_of("*.md *.csv")), ".md, .csv");
        assert_eq!(written_of(&patterns_of("*.*")), "*.*");
        for bad in ["md", "*.", "a*.md", ".*", "*", ".*.md"] {
            assert!(patterns_problem(&patterns_of(bad)).is_some(), "{bad}");
        }
        assert!(patterns_problem(&patterns_of("*.*, .md")).is_some(), "*.* stands alone");
    }

    #[test]
    fn only_extensions_read() {
        for good in ["*.md *.txt", "*.*", "*.md"] {
            assert!(patterns_problem(&p(good)).is_none(), "{good}");
        }
        for bad in ["md", "*.", "a*.md", ".*", "*", "*.* *.md"] {
            assert!(patterns_problem(&p(bad)).is_some(), "{bad}");
        }
    }
}
