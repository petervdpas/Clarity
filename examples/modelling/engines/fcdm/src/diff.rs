//! The differences between a file on disk and what the app writes for it,
//! as a unified diff: its lines taken out (`-`) and added (`+`) with three
//! lines around each part, under a header naming the file. The file's byte
//! order mark and CRLF line endings are no difference: the text is compared
//! as the app writes it.

/// The unified diff of `on_disk` against `written`; empty when they are the same.
pub fn unified(path: &str, on_disk: &str, written: &str) -> String {
    let old_text = on_disk.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let new_text = written.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    if old_text == new_text {
        return String::new();
    }
    let old: Vec<&str> = old_text.lines().collect();
    let new: Vec<&str> = new_text.lines().collect();
    // The longest lines both share, in order (the classic table of common lengths).
    let (n, m) = (old.len(), new.len());
    let mut common = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            common[i][j] = if old[i] == new[j] { common[i + 1][j + 1] + 1 } else { common[i + 1][j].max(common[i][j + 1]) };
        }
    }
    // Each line as it goes: kept, taken out or added, with its place in each text.
    let mut steps: Vec<(char, usize, usize)> = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && old[i] == new[j] {
            steps.push((' ', i, j));
            i += 1;
            j += 1;
        } else if i < n && (j == m || common[i + 1][j] >= common[i][j + 1]) {
            // What is taken out comes before what is added in its place.
            steps.push(('-', i, j));
            i += 1;
        } else {
            steps.push(('+', i, j));
            j += 1;
        }
    }
    let line = |(kind, i, j): (char, usize, usize)| match kind {
        '-' => format!("-{}", old[i]),
        '+' => format!("+{}", new[j]),
        _ => format!(" {}", old[i]),
    };
    // Parts: the changes with three kept lines around them, near changes joined.
    const AROUND: usize = 3;
    let changed: Vec<usize> = steps.iter().enumerate().filter(|(_, s)| s.0 != ' ').map(|(k, _)| k).collect();
    let mut parts: Vec<(usize, usize)> = Vec::new();
    for k in changed {
        let (from, to) = (k.saturating_sub(AROUND), (k + AROUND + 1).min(steps.len()));
        match parts.last_mut() {
            Some(last) if from <= last.1 => last.1 = to,
            _ => parts.push((from, to)),
        }
    }
    let mut out = format!("--- {path} (on disk)\n+++ {path} (written)\n");
    for (from, to) in parts {
        let part = &steps[from..to];
        let old_count = part.iter().filter(|s| s.0 != '+').count();
        let new_count = part.iter().filter(|s| s.0 != '-').count();
        let old_start = part.iter().find(|s| s.0 != '+').map_or(part[0].1, |s| s.1) + 1;
        let new_start = part.iter().find(|s| s.0 != '-').map_or(part[0].2, |s| s.2) + 1;
        out += &format!("@@ -{old_start},{old_count} +{new_start},{new_count} @@\n");
        for s in part {
            out += &line(*s);
            out += "\n";
        }
    }
    if on_disk.ends_with('\n') != written.ends_with('\n') {
        out += "\\ the file's last line ends differently\n";
    }
    out
}

#[cfg(test)]
mod tests {
    use super::unified;

    #[test]
    fn the_same_text_has_no_differences_whatever_its_mark_and_line_endings() {
        assert_eq!(unified("A.cs", "\u{feff}a\r\nb\r\n", "a\nb\n"), "");
    }

    #[test]
    fn a_changed_line_shows_taken_out_and_added_with_lines_around() {
        let old = "1\n2\n3\n4\nfunctieBK\n6\n7\n8\n9\n";
        let new = "1\n2\n3\n4\nfunctieBk\n6\n7\n8\n9\n";
        assert_eq!(unified("A.cs", old, new), "--- A.cs (on disk)\n+++ A.cs (written)\n@@ -2,7 +2,7 @@\n 2\n 3\n 4\n-functieBK\n+functieBk\n 6\n 7\n 8\n");
    }

    #[test]
    fn far_changes_are_parts_of_their_own() {
        let old: String = (1..=20).map(|n| format!("{n}\n")).collect();
        let new = old.replace("2\n", "two\n").replace("19\n", "nineteen\n");
        let diff = unified("A.cs", &old, &new);
        assert_eq!(diff.matches("@@ ").count(), 2, "{diff}");
    }
}
