//! How the names of parts and handlers are written: camelCase, letters and
//! digits only, starting with a small letter (`newGame`, `square1`,
//! `boardClick`). Every stack's generator turns them into its own style.

/// Whether `name` is camelCase: a small letter, then letters and digits.
pub fn is_camel_case(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase()) && chars.all(|c| c.is_ascii_alphanumeric())
}

/// `name` as camelCase: `board_clicked` is `boardClicked`, `Square_1` is
/// `square1`. Characters other than letters and digits split words.
pub fn camel_case(name: &str) -> String {
    let mut out = String::new();
    for word in name.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()) {
        if out.is_empty() {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                out.push(first.to_ascii_lowercase());
                out.push_str(chars.as_str());
            }
        } else {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                out.push(first.to_ascii_uppercase());
                out.push_str(chars.as_str());
            }
        }
    }
    // A name starts with a letter.
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, 'n');
    }
    out
}

/// What the checker says of a name that is not camelCase.
pub fn camel_case_problem(what: &str, name: &str) -> String {
    format!("{what} '{name}' is not camelCase: letters and digits, starting with a small letter, such as '{}'", camel_case(name))
}

/// `source` with every part and handler that is not camelCase renamed to
/// camelCase, wherever the file names it: its declaration, its events, the
/// code that sets or calls it. A name after a dot (a field) and text in
/// quotes stay as they are; a camelCase name already taken gets a number.
pub fn camel_names(source: &str) -> String {
    camel_names_with(source).0
}

/// `camel_names`, and the renames it made (old name, new name).
pub fn camel_names_with(source: &str) -> (String, Vec<(String, String)>) {
    use crate::lexer::{TokenKind, lex};
    let (lines, _) = lex(source);
    // Only windows, the app and their code blocks have parts and handlers;
    // an entity's lines and its data events keep their names.
    let owners: Vec<&str> = lines
        .iter()
        .filter(|l| l.indent == 0 && l.tokens.first().is_some_and(|t| t.text == "page" || t.text == "app"))
        .filter_map(|l| l.tokens.get(1).map(|t| t.text.as_str()))
        .collect();
    let mut in_scope = false;
    let scoped: Vec<bool> = lines
        .iter()
        .map(|l| {
            if l.indent == 0 {
                let first = l.tokens.first().map(|t| t.text.as_str());
                in_scope = matches!(first, Some("page" | "app"))
                    || (first == Some("code") && l.tokens.get(1).is_some_and(|t| owners.contains(&t.text.as_str())));
            }
            in_scope
        })
        .collect();
    let mut taken: std::collections::HashSet<String> =
        lines.iter().flat_map(|l| &l.tokens).filter(|t| t.kind == TokenKind::Ident).map(|t| t.text.clone()).collect();
    let mut renames: Vec<(String, String)> = Vec::new();
    for line in lines.iter().zip(&scoped).filter(|(_, s)| **s).map(|(l, _)| l) {
        let tokens = &line.tokens;
        // A part, or a menu item or button of the frame.
        let is_part = line.indent > 0
            && tokens.first().is_some_and(|t| crate::widgets::by_keyword(&t.text).is_some() || crate::frame::command_by_keyword(&t.text).is_some());
        let declared = if line.indent > 0 && tokens.first().is_some_and(|t| t.text == "handler") {
            tokens.get(1)
        } else if is_part {
            tokens.windows(2).find(|w| w[0].kind == TokenKind::Ident && w[0].text == "name" && w[1].kind == TokenKind::Ident).map(|w| &w[1])
        } else {
            None
        };
        if let Some(name) = declared.filter(|t| t.kind == TokenKind::Ident && !is_camel_case(&t.text))
            && !renames.iter().any(|(old, _)| *old == name.text)
        {
            let base = camel_case(&name.text);
            let new = if taken.contains(&base) { (2..).map(|n| format!("{base}{n}")).find(|n| !taken.contains(n)).expect("a free name") } else { base };
            taken.insert(new.clone());
            renames.push((name.text.clone(), new));
        }
    }
    if renames.is_empty() {
        return (source.to_string(), renames);
    }
    let mut out = String::with_capacity(source.len());
    let mut at = 0;
    for line in lines.iter().zip(&scoped).filter(|(_, s)| **s).map(|(l, _)| l) {
        // A list of fields after `columns` or `show`: `columns name, price`.
        let mut in_fields = false;
        for (i, t) in line.tokens.iter().enumerate() {
            let (start, end) = (t.span.start as usize, t.span.end as usize);
            if t.kind == TokenKind::Str {
                // The parts named in its slots: `{grid.selected.title}`.
                let mut raw = source[start..end].to_string();
                for (old, new) in &renames {
                    raw = raw.replace(&format!("{{{old}."), &format!("{{{new}.")).replace(&format!("{{{old}}}"), &format!("{{{new}}}"));
                }
                if raw != source[start..end] {
                    out.push_str(&source[at..start]);
                    out.push_str(&raw);
                    at = end;
                }
                continue;
            }
            let previous = i.checked_sub(1).map(|p| &line.tokens[p]);
            if t.kind == TokenKind::Ident && matches!(t.text.as_str(), "columns" | "show" | "edit") {
                in_fields = true;
                continue;
            }
            if in_fields {
                let listed = t.kind == TokenKind::Ident && previous.is_some_and(|p| p.kind == TokenKind::Comma || matches!(p.text.as_str(), "columns" | "show" | "edit"));
                if listed || t.kind == TokenKind::Comma {
                    continue;
                }
                in_fields = false;
            }
            let after_dot = previous.is_some_and(|p| p.kind == TokenKind::Dot);
            if t.kind != TokenKind::Ident || after_dot {
                continue;
            }
            if let Some((_, new)) = renames.iter().find(|(old, _)| *old == t.text) {
                out.push_str(&source[at..start]);
                out.push_str(new);
                at = end;
            }
        }
    }
    out.push_str(&source[at..]);
    (out, renames)
}

/// `text` with the parts its slots name renamed: `{loan_grid.selected}`
/// becomes `{loanGrid.selected}`. For translation files, whose texts are the
/// module's texts as written.
pub fn rename_slots(text: &str, renames: &[(String, String)]) -> String {
    let mut out = text.to_string();
    for (old, new) in renames {
        out = out.replace(&format!("{{{old}."), &format!("{{{new}.")).replace(&format!("{{{old}}}"), &format!("{{{new}}}"));
    }
    out
}

/// The name a menu item or button gets: its caption and its kind, as
/// `newGameItem` for `item "New game"` and `ordersButton` for `button
/// "Orders"`. The kind keeps it apart from the window's data and parts of
/// the same word.
pub fn command_name(caption: Option<&str>, keyword: &str) -> String {
    let kind = if keyword == "item" { "Item" } else { "Button" };
    match caption.map(camel_case).filter(|c| !c.is_empty()) {
        Some(caption) => format!("{caption}{kind}"),
        None => keyword.to_string(),
    }
}
