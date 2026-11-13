//! Renaming a field of an entity or a record across the project: its
//! declaration and every place that names it, found through the parsed
//! model, never by searching text. In code, a member is the field when the
//! checker typed its object as the entity's record; `find` order keys and
//! `print` columns are followed to their entity. In a window's `.cxaml` a
//! part names fields of the entity its data holds, and a tree level of the
//! entity it shows; renamed on what the file holds, the file written again.
//! A field of the same name in another entity is never touched.

use std::collections::BTreeSet;

use crate::code::check::{FileTypes, TypeInfo};
use crate::code::{Expr, ExprKind, Stmt, StmtKind};
use crate::design_edit::Designed;
use crate::model::{DataDecl, DataKind, Entity, FieldType, Project, SpecFile, TreeLevel, Widget};
use crate::rename::Renamed;
use crate::widgets::is_identifier;

/// Renames field `old` of entity or record `entity` to `new` in every file
/// that can see the entity. `source` gives the current text of a file by its
/// path in the project (translation files too: `translations/en.clarity`).
pub fn rename_field(project: &Project, source: impl Fn(&str) -> Option<String>, entity: &str, old: &str, new: &str) -> Result<Vec<Renamed>, String> {
    if !is_identifier(new) {
        return Err(format!("'{new}' is not a valid field name"));
    }
    let owners: Vec<(&SpecFile, &Entity)> =
        project.files.iter().flat_map(|f| f.entities.iter().chain(&f.records).filter(|e| e.name.name == entity).map(move |e| (f, e))).collect();
    let (owner, declared) = match owners.as_slice() {
        [one] => *one,
        [] => return Err(format!("there is no entity or record '{entity}'")),
        _ => return Err(format!("'{entity}' is declared in more than one module; rename its field in the file")),
    };
    if declared.settings {
        return Err("the settings are kept per person under their names; rename a setting in the module".into());
    }
    if project.in_web_api(entity) {
        return Err(format!("{entity} comes from a web API, whose fields have the API's names"));
    }
    let Some(field) = declared.field(old) else {
        return Err(format!("'{entity}' has no field '{old}'"));
    };
    if new == old {
        return Ok(Vec::new());
    }
    if declared.field(new).is_some() {
        return Err(format!("'{entity}' already has a field '{new}'"));
    }
    let module = owner.module.as_ref().map(|m| m.name.as_str());
    let types = crate::code::check::types(&project.files);
    let rename = Rename { project, entity, old, new };
    let mut keys = BTreeSet::new();
    let mut changed = Vec::new();
    for file in &project.files {
        let this = file.module.as_ref().map(|m| m.name.as_str());
        // A window's file sees what its module's file sees.
        let uses = project.files.iter().filter(|f| f.module.as_ref().map(|m| m.name.as_str()) == this).flat_map(|f| &f.uses);
        let sees = this == module || uses.into_iter().any(|u| Some(u.name.as_str()) == module);
        if !sees {
            continue;
        }
        let Some(text) = source(&file.path) else {
            return Err(format!("cannot read {}", file.path));
        };
        if crate::cxaml::is_page_file(&file.path) {
            let mut designed = crate::cxaml::read_designed(&text, &file.path)?;
            if rename.in_designed(&mut designed, &mut keys) {
                changed.push(Renamed { path: file.path.clone(), text: crate::cxaml::written_again(&text, &designed), moved_from: None });
            }
            continue;
        }
        let file_types = types.get(&file.path).cloned().unwrap_or_default();
        let mut spans = rename.in_code(file, &file_types);
        let mut inserts = Vec::new();
        if std::ptr::eq(file, owner) {
            spans.insert((field.name.span.start as usize, field.name.span.end as usize));
            // An existing table keeps its column, which followed the name.
            let column = crate::model::snake_case(old);
            if declared.table.is_some() && field.modifiers.column.is_none() && column != crate::model::snake_case(new) {
                inserts.push((line_end(&text, field.span.start as usize, field.span.end as usize), format!(", column \"{column}\"")));
            }
        }
        if spans.is_empty() && inserts.is_empty() {
            continue;
        }
        let mut edits: Vec<(usize, usize, &str, String)> = spans.into_iter().map(|(s, e)| (s, e, old, new.to_string())).collect();
        edits.extend(inserts.into_iter().map(|(at, put)| (at, at, "", put)));
        changed.push(Renamed { path: file.path.clone(), text: edited(&file.path, text, edits)?, moved_from: None });
    }
    // A tree level shown by a key fills the key's slots from the record's fields by name.
    if !keys.is_empty() {
        for language in project.translations.keys() {
            let path = format!("translations/{language}.clarity");
            let Some(text) = source(&path) else { continue };
            let renamed = slots_renamed(&text, &keys, old, new);
            if renamed != text {
                changed.push(Renamed { path, text: renamed, moved_from: None });
            }
        }
    }
    Ok(changed)
}

/// `text` with each `(start, end, was, now)` made, from the last back; a
/// place that no longer says what it said refuses the whole rename.
fn edited(path: &str, mut text: String, mut edits: Vec<(usize, usize, &str, String)>) -> Result<String, String> {
    edits.sort_by_key(|(start, end, _, _)| (*start, *end));
    edits.dedup_by_key(|(start, end, _, _)| (*start, *end));
    for (start, end, was, now) in edits.into_iter().rev() {
        if text.get(start..end) != Some(was) {
            return Err(format!("{path} changed while renaming; try again"));
        }
        text.replace_range(start..end, &now);
    }
    Ok(text)
}

/// Where a declaration line ends before a comment, as the place to add a modifier.
fn line_end(text: &str, start: usize, end: usize) -> usize {
    let line = &text[start..end.min(text.len())];
    let line = &line[..line.find('\n').unwrap_or(line.len())];
    let mut quoted = false;
    let mut cut = line.len();
    let bytes = line.as_bytes();
    for i in 0..bytes.len() {
        match bytes[i] {
            b'"' => quoted = !quoted,
            b'/' if !quoted && bytes.get(i + 1) == Some(&b'/') => {
                cut = i;
                break;
            }
            _ => {}
        }
    }
    start + line[..cut].trim_end().len()
}

/// A translation file with `{old}` as `{new}` in the values of `keys`.
fn slots_renamed(text: &str, keys: &BTreeSet<String>, old: &str, new: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let key = line.trim_start().split_whitespace().next().unwrap_or("");
        if keys.contains(key) {
            out.push_str(&line.replace(&format!("{{{old}}}"), &format!("{{{new}}}")));
        } else {
            out.push_str(line);
        }
    }
    out
}

struct Rename<'a> {
    project: &'a Project,
    entity: &'a str,
    old: &'a str,
    new: &'a str,
}

impl Rename<'_> {
    /// An entity or record of the project by name.
    fn entity_named(&self, name: &str) -> Option<&Entity> {
        self.project.files.iter().flat_map(|f| f.entities.iter().chain(&f.records)).find(|e| e.name.name == name)
    }

    /// The entity a reference field `field` of entity `of` leads to.
    fn through(&self, of: &str, field: &str) -> Option<String> {
        match &self.entity_named(of)?.field(field)?.ty {
            FieldType::Ref { entity } => Some(entity.name.clone()),
            _ => None,
        }
    }

    fn is_it(&self, of: &str, name: &str) -> bool {
        of == self.entity && name == self.old
    }

    // -----------------------------------------------------------------
    // Code

    /// The places in a module file's code that name the field.
    fn in_code(&self, file: &SpecFile, types: &FileTypes) -> BTreeSet<(usize, usize)> {
        let mut found = BTreeSet::new();
        let mut bodies: Vec<&[Stmt]> = Vec::new();
        let mut exprs: Vec<&Expr> = Vec::new();
        for block in &file.code {
            for handler in &block.handlers {
                bodies.push(&handler.body);
                exprs.extend(handler.params.iter().filter_map(|p| p.default.as_ref()));
            }
            bodies.extend(block.events.iter().map(|e| e.body.as_slice()));
        }
        let engine_functions = file.engines.iter().filter_map(|e| e.body.as_ref()).flat_map(|b| &b.functions);
        for function in file.functions.iter().chain(engine_functions) {
            bodies.push(&function.body);
            exprs.extend(function.requires.iter().chain(&function.ensures));
            exprs.extend(function.params.iter().filter_map(|p| p.default.as_ref()));
        }
        for command in &file.commands {
            bodies.push(&command.body);
            exprs.extend(command.help.iter());
            for option in &command.options {
                exprs.extend([&option.default, &option.min, &option.max, &option.help].into_iter().flatten());
            }
        }
        exprs.extend(file.consts.iter().map(|c| &c.value));
        for body in file.engines.iter().filter_map(|e| e.body.as_ref()) {
            exprs.extend(body.consts.iter().map(|c| &c.value).chain(body.keeps.iter().map(|k| &k.value)));
        }
        for body in bodies {
            crate::code::each_expr_in(body, &mut |e| self.in_expr(e, types, &mut found));
            self.in_prints(body, types, &mut found);
        }
        for e in exprs {
            crate::code::each_expr(e, &mut |e| self.in_expr(e, types, &mut found));
        }
        found
    }

    /// One expression: a member of the entity's record, or the order of a `find`.
    fn in_expr(&self, e: &Expr, types: &FileTypes, found: &mut BTreeSet<(usize, usize)>) {
        let mut at = |span: crate::span::Span| {
            found.insert((span.start as usize, span.end as usize));
        };
        match &e.kind {
            ExprKind::Member { object, member } => {
                if let TypeInfo::Record(of) = types.type_of(object)
                    && self.is_it(&of, &member.name)
                {
                    at(member.span);
                }
            }
            ExprKind::Find { entity, order, .. } => {
                for key in order {
                    let mut of = Some(entity.name.clone());
                    for hop in &key.through {
                        let Some(current) = of else { break };
                        if self.is_it(&current, &hop.name) {
                            at(hop.span);
                        }
                        of = self.through(&current, &hop.name);
                    }
                    if let Some(of) = of
                        && self.is_it(&of, &key.field.name)
                    {
                        at(key.field.span);
                    }
                }
            }
            _ => {}
        }
    }

    /// `print orders columns number, total`: the columns are fields of the list's entity.
    fn in_prints(&self, body: &[Stmt], types: &FileTypes, found: &mut BTreeSet<(usize, usize)>) {
        for stmt in body {
            match &stmt.kind {
                StmtKind::Print { value, columns } => {
                    if let TypeInfo::List(of) = types.type_of(value) {
                        for column in columns.iter().filter(|c| self.is_it(&of, &c.name)) {
                            found.insert((column.span.start as usize, column.span.end as usize));
                        }
                    }
                }
                StmtKind::If { branches, otherwise } => {
                    for branch in branches {
                        self.in_prints(&branch.body, types, found);
                    }
                    if let Some(body) = otherwise {
                        self.in_prints(body, types, found);
                    }
                }
                StmtKind::For { body, .. } => self.in_prints(body, types, found),
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------
    // Windows, views and the app

    /// The field renamed in a window's, view's or app's model; the keys of
    /// tree levels that show it through a translation are added to `keys`.
    /// Whether anything changed.
    fn in_designed(&self, designed: &mut Designed, keys: &mut BTreeSet<String>) -> bool {
        let (data, mut all, frame): (_, Vec<&mut Widget>, _) = match designed {
            Designed::Page(page) => (page.data.clone(), page.widgets.iter_mut().collect(), &mut page.frame),
            Designed::App(app) => (app.data.clone(), Vec::new(), &mut app.frame),
        };
        let mut changed = false;
        for sidebar in &mut frame.sidebars {
            all.extend(sidebar.widgets.iter_mut());
        }
        if let Some(footer) = &mut frame.footer {
            all.extend(footer.widgets.iter_mut());
        }
        for widget in all {
            changed |= self.in_widget(widget, &data, keys);
        }
        changed
    }

    /// `name` as the new name when it is the field of entity `of`; whether it was.
    fn put(&self, name: &mut String, of: &str) -> bool {
        let it = self.is_it(of, name);
        if it {
            *name = self.new.to_string();
        }
        it
    }

    fn in_widget(&self, widget: &mut Widget, data: &[DataDecl], keys: &mut BTreeSet<String>) -> bool {
        let mut changed = self.in_props(&mut widget.props, data);
        for level in &mut widget.levels {
            changed |= self.in_level(level, keys);
        }
        for child in widget.children.iter_mut().chain(widget.pages.iter_mut().flat_map(|p| p.children.iter_mut())) {
            changed |= self.in_widget(child, data, keys);
        }
        changed
    }

    /// A part's properties that name fields of the entity its data holds.
    fn in_props(&self, p: &mut crate::model::WidgetProps, data: &[DataDecl]) -> bool {
        let held = |data_name: &str| {
            data.iter().find(|d| d.name.name == data_name && !d.made).and_then(|d| match &d.of {
                DataKind::Record { entity } => Some((entity.name.clone(), false)),
                DataKind::List { entity } => Some((entity.name.clone(), true)),
                DataKind::Value { .. } => None,
            })
        };
        let mut changed = false;
        match p.bind.clone().as_deref().map(|b| b.split_once('.').ok_or(b)) {
            // `customer.name`: a field of the record; a search shows fields of the record a reference leads to.
            Some(Ok((data_name, field))) => {
                if let Some((of, false)) = held(data_name) {
                    let mut renamed = field.to_string();
                    if self.put(&mut renamed, &of) {
                        p.bind = Some(format!("{data_name}.{renamed}"));
                        changed = true;
                    }
                    if let Some(target) = self.through(&of, field) {
                        for shown in &mut p.show {
                            changed |= self.put(shown, &target);
                        }
                    }
                }
            }
            // `customers`: the columns and what the part shows are fields of the list's entity.
            Some(Err(list)) => {
                if let Some((of, true)) = held(list) {
                    for field in p.columns.iter_mut().chain(p.show.iter_mut()).chain(p.edit.iter_mut()).chain(p.switches.iter_mut()) {
                        changed |= self.put(field, &of);
                    }
                    for link in &mut p.links {
                        changed |= self.put(&mut link.column, &of);
                    }
                    if let Some(order) = &mut p.order {
                        changed |= self.put(&mut order.field, &of);
                    }
                    for field in [&mut p.reorder, &mut p.by, &mut p.children, &mut p.row_key, &mut p.parent_key].into_iter().flatten() {
                        changed |= self.put(field, &of);
                    }
                    if let Some(crate::model::ChartMeasure::Sum { field } | crate::model::ChartMeasure::Average { field }) = &mut p.measure {
                        changed |= self.put(field, &of);
                    }
                    if of == self.entity
                        && let Some(width) = p.widths.remove(self.old)
                    {
                        p.widths.insert(self.new.to_string(), width);
                        changed = true;
                    }
                }
            }
            None => {}
        }
        if let Some(objects) = &mut p.objects
            && let Some((of, true)) = held(&objects.list)
        {
            changed |= self.put(&mut objects.x, &of);
            changed |= self.put(&mut objects.y, &of);
        }
        changed
    }

    /// A tree level of the entity: what it hangs under, shows and is ordered by.
    fn in_level(&self, level: &mut TreeLevel, keys: &mut BTreeSet<String>) -> bool {
        let mut changed = false;
        if level.entity.as_deref() == Some(self.entity) {
            for field in [&mut level.under, &mut level.show, &mut level.order].into_iter().flatten() {
                if field == self.old {
                    *field = self.new.to_string();
                    changed = true;
                }
            }
            if let Some(key) = level.show.as_deref().and_then(|s| s.strip_prefix('@')) {
                keys.insert(key.to_string());
            }
        }
        for inner in &mut level.levels {
            changed |= self.in_level(inner, keys);
        }
        changed
    }
}
