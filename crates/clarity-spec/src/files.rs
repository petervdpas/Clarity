//! The files a folder holds, as a file browser writes them
//! (`extensions "*.md *.csv"` on a folder field): `*.md` the files with
//! that extension, ignoring case, or `*.*` alone for any file. Hidden files
//! and folders, whose name starts with a dot, count only with the folder's
//! `hidden` option. Without extensions a folder holds any file. Every
//! runtime matches alike (conformance/files.json).

/// What is wrong with the extensions of a folder: none when they read.
pub fn patterns_problem(patterns: &[String]) -> Option<String> {
    if patterns.is_empty() {
        return Some("name the files it holds, as in 'extensions \"*.md *.csv\"'".to_string());
    }
    for pattern in patterns {
        let fits = pattern == "*.*" || pattern.strip_prefix("*.").is_some_and(|ext| !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
        if !fits {
            return Some(format!("'{pattern}' is no extension of files; write '*.md', or '*.*' for any file"));
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
    fn only_extensions_read() {
        for good in ["*.md *.txt", "*.*", "*.md"] {
            assert!(patterns_problem(&p(good)).is_none(), "{good}");
        }
        for bad in ["md", ".md", "*.", "a*.md", ".*", "*", "*.* *.md"] {
            assert!(patterns_problem(&p(bad)).is_some(), "{bad}");
        }
    }
}
