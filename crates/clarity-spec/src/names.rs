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

/// The functions an engine answers its host by: fixed names, which a
/// project writes as they are.
pub const HOOKS: [&str; 7] = ["clarity_tick", "clarity_draw", "clarity_pointer", "clarity_key", "clarity_open", "clarity_alloc", "clarity_free"];

/// What the checker says of `name`, a `what` (field, function, local...),
/// when it is not camelCase. Fine are `_`, the hooks of an engine, and a
/// camelCase name with one `_` after it, which steps around a word Clarity
/// keeps (`key_`, as an imported description names a field `key`).
pub fn name_problem(what: &str, name: &str) -> Option<String> {
    let bare = name.strip_suffix('_').unwrap_or(name);
    (name != "_" && !is_camel_case(bare) && !HOOKS.contains(&name)).then(|| camel_case_problem(what, name))
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
            && tokens.first().is_some_and(|t| {
                crate::widgets::by_keyword(&t.text).is_some() || crate::frame::command_by_keyword(&t.text).is_some() || matches!(t.text.as_str(), "sidebar" | "ribbon" | "tab" | "group" | "menu" | "toolbar")
            });
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
    let out = renamed_in(source, &lines, &scoped, &renames);
    (out, renames)
}

/// `source` with `renames` made on its lines that `scoped` marks: a part's or
/// handler's name wherever those lines use it, and the slots of their texts.
/// A name after a dot (a field), a list of fields and text stay as they are.
fn renamed_in(source: &str, lines: &[crate::lexer::Line], scoped: &[bool], renames: &[(String, String)]) -> String {
    use crate::lexer::TokenKind;
    let mut out = String::with_capacity(source.len());
    let mut at = 0;
    for line in lines.iter().zip(scoped).filter(|(_, s)| **s).map(|(l, _)| l) {
        // A list of fields after `columns` or `show`: `columns name, price`.
        let mut in_fields = false;
        for (i, t) in line.tokens.iter().enumerate() {
            let (start, end) = (t.span.start as usize, t.span.end as usize);
            if t.kind == TokenKind::Str {
                // The parts named in its slots: `{grid.selected.title}`.
                let mut raw = source[start..end].to_string();
                for (old, new) in renames {
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
                // `columns title 240, artist`: a width may follow a field.
                if listed || t.kind == TokenKind::Comma || t.kind == TokenKind::Number {
                    continue;
                }
                in_fields = false;
            }
            // `order by title`: a field of a table, not a part.
            let ordered = previous.is_some_and(|p| p.text == "by") && i >= 2 && line.tokens[i - 2].text == "order";
            if t.kind == TokenKind::Ident && ordered {
                continue;
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
    out
}

/// A window's, view's or app's own file (`.cxaml`, `markup`) and its module's
/// text (`module`) with every part and handler of that page that is not
/// camelCase renamed to camelCase: a part where the markup declares it
/// (`name`), names it (`for`) or shows it in a slot (`{grid.selected}`), a
/// handler where an event runs it (`onclick="goClicked(1)"`); in the module,
/// in the page's own code block (`code Main`) only. Its data, the views it
/// shows and another page's code keep their names. Gives the markup, the
/// module and the renames (old name, new name), for the translations' slots.
pub fn camel_names_of_page(markup: &str, module: &str) -> (String, String, Vec<(String, String)>) {
    use crate::lexer::{TokenKind, lex};
    let unchanged = || (markup.to_string(), module.to_string(), Vec::new());
    let (Some((_, owner)), Ok(root)) = (crate::cxaml::class_of(markup), crate::xml::read(markup)) else { return unchanged() };
    let (lines, _) = lex(module);
    let mut in_scope = false;
    let scoped: Vec<bool> = lines
        .iter()
        .map(|l| {
            if l.indent == 0 {
                in_scope = l.tokens.first().is_some_and(|t| t.text == "code") && l.tokens.get(1).is_some_and(|t| t.text == owner);
            }
            in_scope
        })
        .collect();
    // Elements whose `name` is no part: the page's data, what it tells, the view a content shows.
    const NOT_PARTS: [&str; 3] = ["Data", "Tells", "View"];
    fn parts(element: &crate::xml::Element, found: &mut Vec<String>) {
        for child in &element.children {
            if NOT_PARTS.contains(&child.name.as_str()) {
                continue;
            }
            if let Some(name) = child.get("name") {
                found.push(name.to_string());
            }
            parts(child, found);
        }
    }
    let mut declared = Vec::new();
    parts(&root, &mut declared);
    for line in lines.iter().zip(&scoped).filter(|(l, s)| **s && l.indent > 0).map(|(l, _)| l) {
        if line.tokens.first().is_some_and(|t| t.text == "handler")
            && let Some(name) = line.tokens.get(1).filter(|t| t.kind == TokenKind::Ident)
        {
            declared.push(name.text.clone());
        }
    }
    let mut taken: std::collections::HashSet<String> = lines.iter().flat_map(|l| &l.tokens).filter(|t| t.kind == TokenKind::Ident).map(|t| t.text.clone()).collect();
    taken.extend(declared.iter().cloned());
    let mut renames: Vec<(String, String)> = Vec::new();
    for name in declared.iter().filter(|n| !is_camel_case(n)) {
        if renames.iter().any(|(old, _)| old == name) {
            continue;
        }
        let base = camel_case(name);
        let new = if taken.contains(&base) { (2..).map(|n| format!("{base}{n}")).find(|n| !taken.contains(n)).expect("a free name") } else { base };
        taken.insert(new.clone());
        renames.push((name.clone(), new));
    }
    if renames.is_empty() {
        return unchanged();
    }
    let module_after = renamed_in(module, &lines, &scoped, &renames);
    // Each attribute value that changes, by where it is in the markup.
    fn values(element: &crate::xml::Element, renames: &[(String, String)], is_part: bool, out: &mut Vec<(usize, usize, String)>) {
        let markup_name = |value: &str| renames.iter().find(|(old, _)| old == value).map(|(_, new)| new.clone());
        for attribute in &element.attributes {
            let value = attribute.value.as_str();
            let new = match attribute.name.as_str() {
                "name" if is_part => markup_name(value),
                "for" => markup_name(value),
                // A handler, with the values it is run with: `goClicked(1)`.
                word if (word.starts_with("on") && word != "opens") || word == "handler" => {
                    let (handler, rest) = value.split_at(value.find('(').unwrap_or(value.len()));
                    markup_name(handler.trim()).map(|new| format!("{new}{rest}"))
                }
                _ => None,
            };
            let new = new.unwrap_or_else(|| rename_slots(value, renames));
            if new != value {
                out.push((attribute.span.start as usize, attribute.span.end as usize, crate::xml::escaped(&new)));
            }
        }
        for child in &element.children {
            if !matches!(child.name.as_str(), "Data" | "Tells") {
                values(child, renames, child.name != "View", out);
            }
        }
    }
    let mut changes = Vec::new();
    values(&root, &renames, false, &mut changes);
    changes.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    let mut markup_after = markup.to_string();
    for (start, end, text) in changes {
        markup_after.replace_range(start..end, &text);
    }
    (markup_after, module_after, renames)
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

/// What a part could be named after what it already knows of its purpose:
/// the input it is for (`vatNumberLabel`), what it shows (`vatNumber`,
/// `ordersTable`, `customerForm`), its file (`logoImage`) or its caption
/// (`addLine`). None when it knows none of these, as a part just placed with
/// its default caption. A name that would be a word of the language gets the
/// part's kind: `saveButton`.
pub fn suggested_name(props: &crate::model::WidgetProps) -> Option<String> {
    use crate::widgets::WidgetKind as K;
    // A custom part is named after what it is: `orbField`.
    if let (K::Custom, Some(part)) = (props.kind, props.part.as_deref()) {
        return Some(camel_case(part));
    }
    let kind = crate::widgets::keyword(props.kind);
    let suffix = |base: &str, word: &str| format!("{base}{}{}", word[..1].to_uppercase(), &word[1..]);
    let base = if let Some(target) = props.for_part.as_deref().filter(|_| matches!(props.kind, K::Label | K::Help)) {
        suffix(target, kind)
    } else if let Some(bind) = props.bind.as_deref() {
        let last = camel_case(bind.rsplit('.').next().unwrap_or(bind));
        match props.kind {
            K::Table | K::List | K::Form | K::Display => suffix(&last, kind),
            K::Text | K::Markdown => suffix(&last, "text"),
            _ => last,
        }
    } else if let Some(source) = props.source.as_deref() {
        let stem = source.rsplit_once('.').map_or(source, |(stem, _)| stem);
        suffix(&camel_case(stem), if props.kind == K::Markdown { "text" } else { kind })
    } else if let Some(caption) = props.caption.as_deref() {
        let named = match caption.strip_prefix('@') {
            Some(key) => ["Caption", "Label", "Title", "Tooltip", "Placeholder"]
                .iter()
                .find_map(|end| key.strip_suffix(end).filter(|k| !k.is_empty()))
                .unwrap_or(key)
                .to_string(),
            None if crate::widgets::info(props.kind).caption.as_deref() == Some(caption) => return None,
            None => camel_case(caption),
        };
        let says_its_kind = matches!(props.kind, K::Label | K::Help | K::Heading | K::Panel);
        let ends = named.to_lowercase().ends_with(kind);
        if (says_its_kind && !ends) || crate::highlight::is_word_of_the_language(&named) { suffix(&named, kind) } else { named }
    } else {
        return None;
    };
    Some(base).filter(|name| is_camel_case(name))
}
