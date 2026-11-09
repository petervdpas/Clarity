//! A tree of several kinds of records (DESIGN 5.33): `tree name modelTree`
//! with `level` lines, each the records of an entity under the row of the
//! level around it, `group` headings shown when the levels inside them have
//! rows, and `on select Entity: handler`. The generators get it as a tree of
//! records arranged by key, which every stack draws already: `lowered` writes
//! a record of the tree's rows, a list of them as the window's data, a
//! handler that loads them (when the window opens, and where code says
//! `modelTree.reload()`), and one that runs the handler of the kind of row
//! picked with its record.

use crate::code::{Stmt, StmtKind};
use crate::model::{FieldType, Project, TreeLevel, Widget};
use crate::widgets::{Event, WidgetKind};

/// The record that holds a row of tree `name`: `modelTree` gives `ModelTreeRow`.
pub fn row_record(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|c| c.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default() + "Row"
}

/// The field of a row that holds a record of `entity`.
fn ref_field(entity: &str) -> String {
    format!("of{entity}")
}

/// Where the places of the code a tree is lowered to begin. That code is
/// parsed from a text of its own, so its places would fall on the module's
/// own code, and the checker keys the types of expressions by place: moved
/// past any text, they take no type of the module's code.
const LOWERED: u32 = 0x4000_0000;

/// Moves the places of `body`'s expressions on by `by`.
fn moved_on(body: &mut [Stmt], by: u32) {
    crate::code::each_expr_in_mut(body, &mut |e| {
        e.span.start += by;
        e.span.end += by;
    });
}

/// The handlers the generated code adds, by tree.
fn load_handler(name: &str) -> String {
    format!("{name}Load")
}

fn picked_handler(name: &str) -> String {
    format!("{name}Picked")
}

/// Every entity a tree's levels show, in the order they are written.
pub fn entities(levels: &[TreeLevel]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(levels: &[TreeLevel], out: &mut Vec<String>) {
        for level in levels {
            if let Some(e) = &level.entity
                && !out.contains(e)
            {
                out.push(e.clone());
            }
            walk(&level.levels, out);
        }
    }
    walk(levels, &mut out);
    out
}

/// The text of a key in the project's own language; empty when it has none.
fn key_text(project: &Project, key: &str) -> String {
    let language = crate::translations::default_language(project);
    project.translations.get(&language).and_then(|t| t.get(key)).cloned().unwrap_or_default()
}

/// What a row of `entity` shows, as Clarity code of the record in `var`:
/// its `show` field, a key with its slots filled from the record's fields,
/// else its first text field, else its id.
fn label(project: &Project, level: &TreeLevel, entity: &str, var: &str) -> String {
    match level.show.as_deref() {
        Some(key) if key.starts_with('@') => {
            let slots = crate::translations::slots(&key_text(project, &key[1..]));
            if slots.is_empty() {
                key.to_string()
            } else {
                let filled: Vec<String> = slots.iter().map(|s| format!("{s} = {var}.{s}")).collect();
                format!("{key}({})", filled.join(", "))
            }
        }
        Some(field) => format!("\"{{{var}.{field}}}\""),
        None => {
            let text = project.entity(entity).and_then(|e| e.fields.iter().find(|f| matches!(f.ty, FieldType::Text { .. })).map(|f| f.name.name.clone()));
            match text {
                Some(field) => format!("\"{{{var}.{field}}}\""),
                None => format!("\"{{{var}.id}}\""),
            }
        }
    }
}

/// The lines that load the rows of `levels` into the list `list`, under the
/// record in `parent` (its entity's row, with key `parent_key`), at `depth`.
#[allow(clippy::too_many_arguments)]
fn load_lines(project: &Project, row: &str, levels: &[TreeLevel], parent: Option<&str>, parent_key: &str, list: &str, depth: usize, groups: &mut usize, out: &mut Vec<String>) {
    let pad = "    ".repeat(depth + 2);
    for level in levels {
        if let Some(heading) = &level.group {
            *groups += 1;
            let n = *groups;
            out.push(format!("{pad}let g{n}: list<{row}> = []"));
            out.push(format!("{pad}let gk{n} = {parent_key} + \"/{}\"", heading.trim_start_matches('@')));
            load_lines(project, row, &level.levels, parent, &format!("gk{n}"), &format!("g{n}"), depth, groups, out);
            out.push(format!("{pad}if count(g{n}) > 0"));
            out.push(format!("{pad}    let h{n} = new {row}"));
            out.push(format!("{pad}    h{n}.key = gk{n}"));
            out.push(format!("{pad}    h{n}.parent = {parent_key}"));
            out.push(format!("{pad}    h{n}.label = {heading}"));
            out.push(format!("{pad}    {list} = {list} + [h{n}] + g{n}"));
            continue;
        }
        let Some(entity) = &level.entity else { continue };
        let v = format!("v{depth}");
        let mut query = format!("find q{depth} in {entity}");
        if let (Some(under), Some(parent)) = (&level.under, parent) {
            query.push_str(&format!(" where q{depth}.{under} == {parent}"));
        }
        if let Some(order) = &level.order {
            query.push_str(&format!(" order by q{depth}.{order}{}", if level.descending { " desc" } else { "" }));
        }
        out.push(format!("{pad}for {v} in {query}"));
        let inner = format!("{pad}    ");
        out.push(format!("{inner}let r{depth} = new {row}"));
        out.push(format!("{inner}r{depth}.key = \"{entity}{{{v}.id}}\""));
        out.push(format!("{inner}r{depth}.parent = {parent_key}"));
        out.push(format!("{inner}r{depth}.label = {}", label(project, level, entity, &v)));
        out.push(format!("{inner}r{depth}.kind = \"{entity}\""));
        out.push(format!("{inner}r{depth}.{} = {v}", ref_field(entity)));
        out.push(format!("{inner}{list} = {list} + [r{depth}]"));
        out.push(format!("{inner}let k{depth} = r{depth}.key"));
        load_lines(project, row, &level.levels, Some(&v), &format!("k{depth}"), list, depth + 1, groups, out);
    }
}

/// A tree of levels found on a page: its name, levels and what it runs when a row is picked.
struct Found {
    name: String,
    levels: Vec<TreeLevel>,
    selects: Vec<crate::model::TreeSelect>,
}

/// Makes each named tree of levels among `widgets` (and the containers in
/// them) a tree of its rows, arranged by key, and gives what it was.
fn take_trees(widgets: &mut [Widget], out: &mut Vec<Found>) {
    for w in widgets.iter_mut() {
        if w.props.kind == WidgetKind::Tree
            && !w.levels.is_empty()
            && let Some(name) = w.props.name.clone()
        {
            w.props.bind = Some(format!("{name}Rows"));
            w.props.row_key = Some("key".into());
            w.props.parent_key = Some("parent".into());
            w.props.show = vec!["label".into()];
            w.props.handlers.retain(|h| h.event != Event::Select);
            w.props.handlers.push(crate::model::Handler { event: Event::Select, handler: picked_handler(&name), args: Vec::new() });
            out.push(Found { name, levels: std::mem::take(&mut w.levels), selects: std::mem::take(&mut w.selects) });
        }
        take_trees(&mut w.children, out);
        for page in &mut w.pages {
            take_trees(&mut page.children, out);
        }
    }
}

/// Each `tree.reload()` in `body` as a call of the tree's load handler.
fn reloads_as_calls(body: &mut [Stmt], tree: &str) {
    for stmt in body {
        match &mut stmt.kind {
            StmtKind::Draw { canvas, shape, args } if canvas.name == tree && shape.name == "reload" && args.is_empty() => {
                let function = crate::model::Ident { name: load_handler(tree), span: canvas.span };
                stmt.kind = StmtKind::Call { call: crate::code::Expr { kind: crate::code::ExprKind::Call { function, args: Vec::new(), named: Vec::new() }, span: stmt.span } };
            }
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    reloads_as_calls(&mut branch.body, tree);
                }
                if let Some(otherwise) = otherwise {
                    reloads_as_calls(otherwise, tree);
                }
            }
            StmtKind::For { body, .. } => reloads_as_calls(body, tree),
            _ => {}
        }
    }
}

/// The project with each tree of levels written as a tree of records (see the module's comment).
pub fn lowered(project: &Project) -> Project {
    let mut out = project.clone();
    // Each tree's code gets places of its own, past those of the trees before it.
    let mut next = LOWERED;
    for f in 0..out.files.len() {
        for p in 0..out.files[f].pages.len() {
            let mut found = Vec::new();
            take_trees(&mut out.files[f].pages[p].widgets, &mut found);
            // A tree in a sidebar of the window, as an explorer stands.
            for sidebar in &mut out.files[f].pages[p].frame.sidebars {
                take_trees(&mut sidebar.widgets, &mut found);
            }
            for Found { name, levels, selects } in found {
                let row = row_record(&name);
                let entities = entities(&levels);
                let mut text = format!("module {}\n\nrecord {row}\n    key: text\n    parent: text\n    label: text\n    kind: text\n", out.files[f].module.as_ref().map_or("m", |m| m.name.as_str()));
                for e in &entities {
                    text.push_str(&format!("    {}: ref {e}\n", ref_field(e)));
                }
                let window = out.files[f].pages[p].name.name.clone();
                text.push_str(&format!("\ncode {window}\n    handler {}\n        let rows: list<{row}> = []\n", load_handler(&name)));
                let mut lines = Vec::new();
                let mut groups = 0;
                load_lines(&out, &row, &levels, None, "\"\"", "rows", 0, &mut groups, &mut lines);
                text.push_str(&lines.join("\n"));
                text.push_str(&format!("\n        {name}Rows = rows\n\n    handler {}\n        let r = {name}.selected\n", picked_handler(&name)));
                for (i, select) in selects.iter().enumerate() {
                    text.push_str(&format!("        if r.kind == \"{}\"\n            let x{i} = r.{}\n            {}(x{i})\n", select.entity, ref_field(&select.entity), select.handler));
                }
                let (made, _) = crate::parser::parse(&text, &out.files[f].path);
                out.files[f].records.extend(made.records);
                // The window's rows of the tree: `<name>Rows`, a list of the row record.
                let ident = |name: String| crate::model::Ident { name, span: crate::span::Span::default() };
                out.files[f].pages[p].data.push(crate::model::DataDecl { name: ident(format!("{name}Rows")), of: crate::model::DataKind::List { entity: ident(row.clone()) }, made: false, span: crate::span::Span::default() });
                let mut handlers = made.code.into_iter().flat_map(|c| c.handlers).collect::<Vec<_>>();
                let at = next;
                next += text.len() as u32 + 1;
                for h in &mut handlers {
                    moved_on(&mut h.body, at);
                }
                // The call that loads the rows when the window opens is the tree's too.
                let load_at = crate::span::Span { start: at, end: at, ..crate::span::Span::default() };
                // The rows load when the window opens: its open handler loads them first.
                let page = &mut out.files[f].pages[p];
                let opened = page.handlers.iter().find(|h| h.event == Event::Open).map(|h| h.handler.clone());
                let opened = match opened {
                    Some(h) => h,
                    None => {
                        let h = format!("{name}Opened");
                        page.handlers.push(crate::model::Handler { event: Event::Open, handler: h.clone(), args: Vec::new() });
                        h
                    }
                };
                let code = match out.files[f].code.iter().position(|c| c.name.name == window) {
                    Some(i) => &mut out.files[f].code[i],
                    None => {
                        let block = crate::code::CodeBlock { name: crate::model::Ident { name: window.clone(), span: crate::span::Span::default() }, handlers: Vec::new(), events: Vec::new(), span: crate::span::Span::default() };
                        out.files[f].code.push(block);
                        out.files[f].code.last_mut().expect("just added")
                    }
                };
                let call = |span| Stmt { kind: StmtKind::Call { call: crate::code::Expr { kind: crate::code::ExprKind::Call { function: crate::model::Ident { name: load_handler(&name), span }, args: Vec::new(), named: Vec::new() }, span } }, span };
                match code.handlers.iter_mut().find(|h| h.name.name == opened) {
                    Some(h) => h.body.insert(0, call(load_at)),
                    None => {
                        let mut h = handlers.iter().find(|h| h.name.name == load_handler(&name)).cloned().expect("the load handler");
                        h.name.name = opened.clone();
                        h.params.clear();
                        h.body = vec![call(load_at)];
                        code.handlers.push(h);
                    }
                }
                for h in &mut code.handlers {
                    reloads_as_calls(&mut h.body, &name);
                }
                code.handlers.extend(handlers);
            }
        }
    }
    out
}

/// Rows a tree of levels shows where there is no data, in the designer and
/// the preview: two records of each level under the first record of the
/// level around it, a group's heading over its rows, each label the entity's
/// name and number. Each row as the tree's record holds it: key, parent, label, kind.
pub fn sample_rows(project: &Project, levels: &[TreeLevel]) -> Vec<serde_json::Map<String, serde_json::Value>> {
    fn row(key: &str, parent: &str, label: &str, kind: &str) -> serde_json::Map<String, serde_json::Value> {
        let mut r = serde_json::Map::new();
        r.insert("key".into(), key.into());
        r.insert("parent".into(), parent.into());
        r.insert("label".into(), label.into());
        r.insert("kind".into(), kind.into());
        r
    }
    fn walk(project: &Project, levels: &[TreeLevel], parent: &str, depth: usize, out: &mut Vec<serde_json::Map<String, serde_json::Value>>) {
        if depth > 6 {
            return;
        }
        for level in levels {
            if let Some(heading) = &level.group {
                let key = format!("{parent}/{}", heading.trim_start_matches('@'));
                let mut inner = Vec::new();
                walk(project, &level.levels, &key, depth, &mut inner);
                if !inner.is_empty() {
                    let text = key_text(project, heading.trim_start_matches('@'));
                    out.push(row(&key, parent, if text.is_empty() { heading } else { &text }, ""));
                    out.extend(inner);
                }
                continue;
            }
            let Some(entity) = &level.entity else { continue };
            for n in 1..=2 {
                let key = format!("{parent}/{entity}{n}");
                out.push(row(&key, parent, &format!("{entity} {n}"), entity));
                // The levels inside show under the first record only, so the sample stays short.
                if n == 1 {
                    walk(project, &level.levels, &key, depth + 1, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(project, levels, "", 0, &mut out);
    // The top's rows stand under no row.
    for r in &mut out {
        if r.get("parent").and_then(|p| p.as_str()) == Some("") {
            r.insert("parent".into(), "".into());
        }
    }
    out
}

/// The sample rows of every tree of levels in the project, by the data that holds them (`modelTreeRows`).
pub fn sample_data(project: &Project) -> std::collections::BTreeMap<String, Vec<serde_json::Map<String, serde_json::Value>>> {
    let mut out = std::collections::BTreeMap::new();
    fn walk(project: &Project, widgets: &[Widget], out: &mut std::collections::BTreeMap<String, Vec<serde_json::Map<String, serde_json::Value>>>) {
        for w in widgets {
            if w.props.kind == WidgetKind::Tree
                && !w.levels.is_empty()
                && let Some(name) = &w.props.name
            {
                out.insert(format!("{name}Rows"), sample_rows(project, &w.levels));
            }
            walk(project, &w.children, out);
            for page in &w.pages {
                walk(project, &page.children, out);
            }
        }
    }
    for page in project.pages() {
        walk(project, &page.widgets, &mut out);
        for sidebar in &page.frame.sidebars {
            walk(project, &sidebar.widgets, &mut out);
        }
    }
    out
}

/// `widgets` with each tree of levels as the tree of its rows, as `lowered` makes it, for showing them.
pub fn lowered_widgets(_project: &Project, widgets: &[Widget]) -> Vec<Widget> {
    let mut out = widgets.to_vec();
    let mut found = Vec::new();
    take_trees(&mut out, &mut found);
    out
}
