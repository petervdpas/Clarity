//! The reports of a project (docs/ledger/reports-pdf.md, step 6): the `.report`
//! files of its `reports` folder, XML naming the paper, the data a report
//! takes, its bands and their parts, and the stylesheet in `reports` it
//! names. Read with the project and checked against its entities: each
//! Data's type, each bind through the fields, a detail's list, a sum's
//! numbers, a text's key and a part's class.

use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use crate::diagnostic::Diagnostic;
use crate::model::{FieldType, Project};
use crate::span::Span;
use crate::xml::{self, Element};

/// The folder of a project's reports.
pub const REPORTS_DIR: &str = "reports";
/// A report's extension.
pub const REPORT_EXTENSION: &str = "report";

/// The bands of a report, by their element's name.
pub const BANDS: [&str; 7] = ["ReportHeader", "PageHeader", "GroupHeader", "Detail", "GroupFooter", "ReportFooter", "PageFooter"];
/// The parts of a band, by their element's name.
pub const PARTS: [&str; 8] = ["Text", "Field", "Sum", "Markdown", "PageNumber", "Image", "Line", "Box"];

/// A report's file: its name (the file's stem), its path, its text, and the
/// stylesheet it names with that file's text (none when it names none or
/// the file is not there).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ReportFile {
    pub name: String,
    pub path: String,
    pub text: String,
    pub stylesheet: Option<String>,
}

/// The `.report` files of the reports folder in `dir`, by name, each with
/// the text of the stylesheet it names; none when there is no folder.
pub fn report_files(dir: &Path) -> std::io::Result<Vec<ReportFile>> {
    let folder = dir.join(REPORTS_DIR);
    if !folder.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&folder)? {
        let path = entry?.path();
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !path.is_file() || file.starts_with('.') || path.extension().is_none_or(|e| e != REPORT_EXTENSION) {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        let named = xml::read(&text).ok().and_then(|root| root.get("stylesheet").map(str::to_string));
        let stylesheet = named.and_then(|n| std::fs::read_to_string(folder.join(n)).ok());
        let name = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        found.push(ReportFile { name, path: path.display().to_string(), text, stylesheet });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

/// What a new report `name` starts with: A4 paper, a report header, and a
/// page footer with the page's number.
pub fn starter(name: &str) -> String {
    format!(
        "<Report name=\"{}\" paper=\"A4\" margin=\"20mm\">\n  <ReportHeader height=\"20mm\" />\n  <PageFooter height=\"8mm\">\n    <PageNumber at=\"80,0\" size=\"10,5\" align=\"center\" />\n  </PageFooter>\n</Report>\n",
        xml::escaped(name)
    )
}

/// Data to show a report with before code gathers its own (the IDE's
/// preview): for each Data a record of its entity, each field a value of
/// its type, a list of records three of them, as JSON.
pub fn sample_data(project: &Project, report: &str) -> serde_json::Value {
    use serde_json::{Map, Value, json};
    fn record(project: &Project, entity: &str, n: usize, depth: usize) -> Value {
        let Some(entity) = project.entity(entity) else { return Value::Null };
        let mut out = Map::new();
        for field in &entity.fields {
            let name = &field.name.name;
            let value = match &field.ty {
                FieldType::Text { .. } | FieldType::Email | FieldType::Url => Value::from(format!("{name} {n}")),
                // No picture in sample data: an Image bound to it shows nothing.
                FieldType::Image => Value::Null,
                FieldType::Integer | FieldType::Duration => Value::from(n as i64),
                FieldType::Decimal { scale, .. } => Value::from(format!("{n}.{}", "5".repeat((*scale as usize).clamp(1, 4)))),
                FieldType::Money => Value::from(format!("{n}.50")),
                FieldType::Bool => Value::Bool(n % 2 == 1),
                FieldType::Date => Value::from(format!("2026-01-{:02}", n.min(28))),
                FieldType::DateTime => Value::from(format!("2026-01-{:02}T10:00:00", n.min(28))),
                FieldType::Color => Value::from("#1f5fbf"),
                FieldType::Record { entity } if depth < 2 => record(project, &entity.name, n, depth + 1),
                FieldType::List { item } if depth < 2 => Value::Array((1..=3).map(|i| record(project, &item.name, i, depth + 1)).collect()),
                _ => Value::Null,
            };
            out.insert(name.clone(), value);
        }
        Value::Object(out)
    }
    let data: Map<String, Value> = report_data(project, report).unwrap_or_default().into_iter().map(|(name, ty)| (name, record(project, &ty, 1, 0))).collect();
    json!(data)
}

/// How each value a report shows is formatted, by the bind as the report
/// writes it (a Field's `bind`, a Sum's `of`, a group's `by`): its field's
/// type as Clarity writes formats (`money`, `decimal 2`, `date`), for
/// Clarity.Pdf to show it in the person's language. A text has none.
pub fn formats_of(project: &Project, report: &str) -> std::collections::BTreeMap<String, String> {
    let mut found = std::collections::BTreeMap::new();
    let Some(file) = project.reports.iter().find(|r| r.name == report) else { return found };
    let Ok(root) = xml::read(&file.text) else { return found };
    let mut checking = Checking { project, file, data: data_of(&file.text), found: Vec::new() };
    let detail = checking.detail_item();
    let mut binds: Vec<&str> = Vec::new();
    for band in root.children.iter().filter(|b| b.name != "Data") {
        binds.extend(band.get("by"));
        for part in &band.children {
            binds.extend(part.get("bind").or(part.get("of")));
        }
    }
    for bind in binds {
        let ty = match checking.named(bind, detail.map(|d| ("", d)), Span::default()) {
            Some(Named::Value(ty)) => ty,
            _ => continue,
        };
        let format = match ty {
            FieldType::Integer => "integer".to_string(),
            FieldType::Decimal { scale, .. } => format!("decimal {scale}"),
            FieldType::Money => "money".to_string(),
            FieldType::Date => "date".to_string(),
            FieldType::DateTime => "datetime".to_string(),
            FieldType::Duration => "duration".to_string(),
            _ => continue,
        };
        found.insert(bind.to_string(), format);
    }
    found
}

/// The data a report's text says it takes: each Data's name and type.
pub fn data_of(text: &str) -> Vec<(String, String)> {
    let Ok(root) = xml::read(text) else { return Vec::new() };
    root.children.iter().filter(|c| c.name == "Data").filter_map(|d| Some((d.get("name")?.to_string(), d.get("type")?.to_string()))).collect()
}

/// The data the project's report `name` takes; none when there is no such report.
pub fn report_data(project: &Project, name: &str) -> Option<Vec<(String, String)>> {
    project.reports.iter().find(|r| r.name == name).map(|r| data_of(&r.text))
}

fn pdf_reports<'w>(widgets: &'w [crate::model::Widget], found: &mut Vec<&'w crate::model::Widget>) {
    for w in widgets {
        if w.props.kind == crate::widgets::WidgetKind::PdfReport {
            found.push(w);
        }
        pdf_reports(&w.children, found);
        for page in &w.pages {
            pdf_reports(&page.children, found);
        }
    }
}

/// What is wrong with the pdfReport parts: a report that is not there, and
/// code setting a part's data the report does not take.
fn check_parts(project: &Project) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for file in &project.files {
        for page in &file.pages {
            let mut parts = Vec::new();
            pdf_reports(&page.widgets, &mut parts);
            for part in parts {
                let name = part.props.name.clone().unwrap_or_default();
                let Some(report) = &part.props.report else {
                    found.push(Diagnostic::error(format!("pdfReport {name} needs 'report', the report it shows"), part.span).in_file(&file.path));
                    continue;
                };
                let Some(data) = report_data(project, report) else {
                    let have: Vec<&str> = project.reports.iter().map(|r| r.name.as_str()).collect();
                    found.push(Diagnostic::error(format!("there is no report '{report}' in the reports folder{}", if have.is_empty() { String::new() } else { format!("; it has {}", have.join(", ")) }), part.span).in_file(&file.path));
                    continue;
                };
                // Code that sets `part.x`: x is a Data of the report.
                for code_file in &project.files {
                    for block in &code_file.code {
                        for handler in &block.handlers {
                            crate::code::check::each_stmt(&handler.body, &mut |stmt| {
                                let crate::code::StmtKind::Assign { target, .. } = &stmt.kind else { return };
                                let crate::code::ExprKind::Member { object, member } = &target.kind else { return };
                                let crate::code::ExprKind::Name { name: object } = &object.kind else { return };
                                if *object == name && !data.iter().any(|(n, _)| *n == member.name) {
                                    let takes: Vec<&str> = data.iter().map(|(n, _)| n.as_str()).collect();
                                    found.push(Diagnostic::error(format!("report {report} takes no data '{}'; it takes {}", member.name, takes.join(", ")), member.span).in_file(&code_file.path));
                                }
                            });
                        }
                    }
                }
            }
        }
    }
    found
}

/// What is wrong with code's PDFs of reports (`pdf of Invoice with order = current`):
/// a report that is not there, data it does not take, or data it takes left out.
fn check_code(project: &Project) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for file in &project.files {
        let mut look = |body: &[crate::code::Stmt]| {
            crate::code::each_expr_in(body, &mut |e| {
                let crate::code::ExprKind::PdfReport { report, data } = &e.kind else { return };
                let Some(takes) = report_data(project, &report.name) else {
                    let have: Vec<&str> = project.reports.iter().map(|r| r.name.as_str()).collect();
                    found.push(Diagnostic::error(format!("there is no report '{}' in the reports folder{}", report.name, if have.is_empty() { String::new() } else { format!("; it has {}", have.join(", ")) }), report.span).in_file(&file.path));
                    return;
                };
                for given in data.iter().filter(|g| !takes.iter().any(|(n, _)| *n == g.name.name)) {
                    let names: Vec<&str> = takes.iter().map(|(n, _)| n.as_str()).collect();
                    found.push(Diagnostic::error(format!("report {} takes no data '{}'; it takes {}", report.name, given.name.name, if names.is_empty() { "none".to_string() } else { names.join(", ") }), given.name.span).in_file(&file.path));
                }
                for (name, ty) in takes.iter().filter(|(n, _)| !data.iter().any(|g| g.name.name == *n)) {
                    found.push(Diagnostic::error(format!("report {} takes {name}, a {ty}: give it, as in pdf of {} with {name} = ...", report.name, report.name), e.span).in_file(&file.path));
                }
            });
        };
        for block in &file.code {
            block.handlers.iter().for_each(|h| look(&h.body));
            block.events.iter().for_each(|ev| look(&ev.body));
        }
        file.functions.iter().for_each(|f| look(&f.body));
        file.commands.iter().for_each(|c| look(&c.body));
    }
    found
}

/// What a bind names: a value, a list of records, or nothing found.
enum Named<'p> {
    Value(&'p FieldType),
    Records(&'p str),
}

struct Checking<'p> {
    project: &'p Project,
    file: &'p ReportFile,
    data: Vec<(String, String)>,
    found: Vec<Diagnostic>,
}

impl<'p> Checking<'p> {
    fn error(&mut self, message: String, span: Span) {
        self.found.push(Diagnostic::error(message, span).in_file(&self.file.path));
    }

    /// Follows `bind` from a Data (or, in a detail, from its item of `item`'s entity) through the fields.
    fn named(&mut self, bind: &str, item: Option<(&str, &'p str)>, span: Span) -> Option<Named<'p>> {
        let mut names = bind.split('.');
        let first = names.next().unwrap_or_default();
        let mut entity: &'p str = match (self.data.iter().find(|(n, _)| n == first), item) {
            (Some((_, ty)), _) => self.project.entity(ty).map(|e| e.name.name.as_str())?,
            (None, Some((_, item_entity))) => item_entity,
            (None, None) => {
                let takes: Vec<&str> = self.data.iter().map(|(n, _)| n.as_str()).collect();
                self.error(format!("'{bind}' names no data of the report; it takes {}", if takes.is_empty() { "none".to_string() } else { takes.join(", ") }), span);
                return None;
            }
        };
        let rest: Vec<&str> = names.collect();
        if rest.is_empty() {
            return Some(Named::Records(entity));
        }
        for (i, name) in rest.iter().enumerate() {
            let Some(field) = self.project.entity(entity).and_then(|e| e.field(name)) else {
                self.error(format!("{entity} has no field '{name}' ('{bind}')"), span);
                return None;
            };
            let last = i + 1 == rest.len();
            match &field.ty {
                FieldType::Record { entity: inner } => entity = self.project.entity(&inner.name).map(|e| e.name.name.as_str())?,
                FieldType::List { item } if last => return self.project.entity(&item.name).map(|e| Named::Records(e.name.name.as_str())),
                FieldType::Ref { entity: other } => {
                    self.error(format!("'{bind}' goes through '{name}', a reference to {}: give the report that record as data of its own", other.name), span);
                    return None;
                }
                ty if last => return Some(Named::Value(ty)),
                _ => {
                    self.error(format!("'{bind}': '{name}' holds no fields"), span);
                    return None;
                }
            }
        }
        Some(Named::Records(entity))
    }

    fn part(&mut self, part: &Element, item: Option<(&str, &'p str)>, texts: Option<&std::collections::BTreeMap<String, String>>, classes: &[String]) {
        let span = part.span;
        for name in ["at", "size"] {
            match part.get(name) {
                None => self.error(format!("{} needs '{name}', two numbers in mm, as in \"10,20\"", part.name), span),
                Some(text) if text.split(',').map(|n| n.trim().parse::<f64>()).collect::<Result<Vec<_>, _>>().map(|n| n.len()).ok() != Some(2) => {
                    self.error(format!("'{name}' of {} is two numbers in mm, as in \"10,20\", not '{text}'", part.name), span)
                }
                _ => {}
            }
        }
        if let Some(align) = part.get("align").filter(|a| !["left", "center", "right"].contains(a)) {
            self.error(format!("'align' of {} is left, center or right, not '{align}'", part.name), span);
        }
        if let Some(class) = part.get("class").filter(|c| !classes.iter().any(|k| k == c)) {
            self.error(format!("the report's stylesheet has no class '{class}' (.{class} {{ ... }})"), span);
        }
        match part.name.as_str() {
            "Text" => match part.get("key") {
                None => self.error("Text needs 'key', the text's key in the translations".into(), span),
                Some(key) if texts.is_some_and(|t| !t.contains_key(key)) => self.error(format!("there is no text '{key}' in the translations"), span),
                _ => {}
            },
            "Field" | "Markdown" => match part.get("bind") {
                None => self.error(format!("{} needs 'bind', the field it shows, as in order.number", part.name), span),
                Some(bind) => match self.named(bind, item, span) {
                    Some(Named::Records(_)) => self.error(format!("{} shows one value; '{bind}' holds records", part.name), span),
                    Some(Named::Value(ty)) if part.name == "Markdown" && !matches!(ty, FieldType::Text { .. }) => self.error(format!("Markdown shows a text; '{bind}' is no text"), span),
                    _ => {}
                },
            },
            "Sum" => match (part.get("of"), self.detail_item()) {
                (None, _) => self.error("Sum needs 'of', the detail's field it adds, as in line.price".into(), span),
                (Some(_), None) => self.error("a Sum adds the items of a Detail band, and the report has none".into(), span),
                (Some(of), Some(detail)) => match self.named(of, Some(("", detail)), span) {
                    Some(Named::Value(FieldType::Integer | FieldType::Decimal { .. } | FieldType::Money)) | None => {}
                    Some(_) => self.error(format!("a Sum adds numbers; '{of}' is none"), span),
                },
            },
            "PageNumber" | "Line" | "Box" => {}
            "Image" if part.get("barcode").is_some() => {
                let kind = part.get("barcode").unwrap_or_default();
                if !["qr", "code128", "ean13"].contains(&kind) {
                    self.error(format!("an Image draws no barcode '{kind}': it draws qr, code128, ean13"), span);
                }
                match part.get("bind") {
                    None => self.error("a barcode needs 'bind', the field whose value it draws".into(), span),
                    Some(bind) => {
                        if let Some(Named::Records(_)) = self.named(bind, item, span) {
                            self.error(format!("a barcode draws one value; '{bind}' holds records"), span);
                        }
                    }
                }
            }
            "Image" => match (part.get("source"), part.get("bind")) {
                (None, None) => self.error("Image needs 'source', a picture of the resources folder, or 'bind', an image field".into(), span),
                (Some(source), _) if self.project.resources.as_ref().is_some_and(|r| !r.files.iter().any(|f| f.name == source)) => {
                    self.error(format!("there is no picture {source} in the resources folder"), span)
                }
                (None, Some(bind)) => match self.named(bind, item, span) {
                    Some(Named::Value(FieldType::Image)) | None => {}
                    Some(_) => self.error(format!("an Image shows a picture; '{bind}' is no image field"), span),
                },
                _ => {}
            },
            other => self.error(format!("a band has no '{other}': its parts are {}", PARTS.join(", ")), span),
        }
    }

    /// The entity of the items of the report's Detail band.
    fn detail_item(&mut self) -> Option<&'p str> {
        let root = xml::read(&self.file.text).ok()?;
        let list = root.children.iter().find(|b| b.name == "Detail")?.get("list")?.to_string();
        let first = list.split('.').next().unwrap_or_default();
        let (_, ty) = self.data.iter().find(|(n, _)| n == first)?.clone();
        let mut entity = self.project.entity(&ty)?;
        for name in list.split('.').skip(1) {
            match &entity.field(name)?.ty {
                FieldType::List { item } | FieldType::Record { entity: item } => entity = self.project.entity(&item.name)?,
                _ => return None,
            }
        }
        Some(entity.name.name.as_str())
    }
}

/// The classes a stylesheet's text declares (`.title`).
fn classes(css: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut depth = 0;
    let mut prelude = String::new();
    for c in css.chars() {
        match c {
            '{' => {
                if depth == 0 {
                    for selector in prelude.split(',') {
                        if let Some(name) = selector.trim().strip_prefix('.') {
                            found.push(name.trim().to_string());
                        }
                    }
                }
                depth += 1;
                prelude.clear();
            }
            '}' => depth -= 1,
            _ if depth == 0 => prelude.push(c),
            _ => {}
        }
    }
    found
}

/// The papers as a sentence: "A3, A4, A5, Letter or Legal".
fn papers_said() -> String {
    let papers = crate::report_design::PAPERS;
    format!("{} or {}", papers[..papers.len() - 1].join(", "), papers[papers.len() - 1])
}

/// What is wrong in a project's reports.
pub fn check(project: &Project) -> Vec<Diagnostic> {
    let mut found = check_parts(project);
    found.extend(check_code(project));
    let default = crate::translations::default_language(project);
    let texts = project.translations.get(&default);
    for file in &project.reports {
        let root = match xml::read(&file.text) {
            Ok(root) => root,
            Err(problem) => {
                found.push(problem.in_file(&file.path));
                continue;
            }
        };
        let mut checking = Checking { project, file, data: Vec::new(), found: Vec::new() };
        if root.name != "Report" {
            checking.error(format!("a report starts with <Report>, not <{}>", root.name), root.span);
        }
        if root.get("name") != Some(file.name.as_str()) {
            checking.error(format!("the report in {}.report is named {}: write name=\"{}\"", file.name, file.name, file.name), root.span);
        }
        if let Some(other) = root.get("headerFirst").filter(|v| !["true", "false"].contains(v)) {
            checking.error(format!("'headerFirst' of Report is true or false, not '{other}'"), root.span);
        }
        // The papers the Reports tab offers (crate::report_design::PAPERS), written as it writes them, turned or not.
        if let Some(paper) = root.get("paper") {
            let words: Vec<&str> = paper.split_whitespace().collect();
            let fits = match words.as_slice() {
                [name] => crate::report_design::PAPERS.contains(name),
                [name, turn] => crate::report_design::PAPERS.contains(name) && ["landscape", "portrait"].contains(turn),
                _ => false,
            };
            if !fits {
                checking.error(format!("'paper' of Report is {}, with landscape or portrait after it, not '{paper}'", papers_said()), root.span);
            }
        }
        let named_sheet = root.get("stylesheet");
        if named_sheet.is_some() && file.stylesheet.is_none() {
            checking.error(format!("the stylesheet {} is not in the reports folder", named_sheet.unwrap_or_default()), root.span);
        }
        let classes = file.stylesheet.as_deref().map(classes).unwrap_or_default();
        for data in root.children.iter().filter(|c| c.name == "Data") {
            match (data.get("name"), data.get("type")) {
                (Some(name), Some(ty)) if project.entity(ty).is_some() => checking.data.push((name.to_string(), ty.to_string())),
                (Some(_), Some(ty)) => checking.error(format!("unknown entity '{ty}': a report's data is a record of an entity"), data.span),
                _ => checking.error("Data needs 'name' and 'type', as in <Data name=\"order\" type=\"Order\" />".into(), data.span),
            }
        }
        for band in root.children.iter().filter(|c| c.name != "Data") {
            if !BANDS.contains(&band.name.as_str()) {
                checking.error(format!("a report has no '{}': its parts are Data, {}", band.name, BANDS.join(", ")), band.span);
                continue;
            }
            if band.get("height").is_none() {
                checking.error(format!("{} needs 'height', as in height=\"10mm\"", band.name), band.span);
            }
            // A group's bands speak of the detail's items: their binds start with its item.
            let group_item = if matches!(band.name.as_str(), "GroupHeader" | "GroupFooter") {
                let detail = checking.detail_item();
                match (band.get("by"), detail) {
                    (None, _) => {
                        checking.error(format!("a {} needs 'by', the detail's field it groups by, as in line.category", band.name), band.span);
                        None
                    }
                    (Some(_), None) => {
                        checking.error(format!("a {} groups the items of a Detail band, and the report has none", band.name), band.span);
                        None
                    }
                    (Some(by), Some(entity)) => {
                        if let Some(Named::Records(_)) = checking.named(by, Some(("", entity)), band.span) {
                            checking.error(format!("a group goes by one value; '{by}' holds records"), band.span);
                        }
                        if band.name == "GroupFooter" && !root.children.iter().any(|b| b.name == "GroupHeader" && b.get("by") == Some(by)) {
                            checking.error(format!("the GroupFooter by {by} has no GroupHeader by {by}"), band.span);
                        }
                        Some(("", entity))
                    }
                }
            } else {
                None
            };
            let item = match (band.name.as_str(), band.get("list")) {
                ("Detail", None) => {
                    checking.error("a Detail needs 'list', the records it repeats for, as in order.lines".into(), band.span);
                    None
                }
                ("Detail", Some(list)) => match checking.named(list, None, band.span) {
                    Some(Named::Records(entity)) if list.contains('.') => Some(("", entity)),
                    Some(_) => {
                        checking.error(format!("a Detail repeats for a list of records; '{list}' is none"), band.span);
                        None
                    }
                    None => None,
                },
                _ => group_item,
            };
            for part in &band.children {
                checking.part(part, item, texts, &classes);
            }
        }
        found.append(&mut checking.found);
    }
    found
}
