//! The checker's rules of data and bindings: a window's data, its commands
//! and data, what each part binds, fields, defaults and hashed fields.

use super::*;

impl<'a> Checker<'a> {
    pub(super) fn data_context(&mut self, file: usize, decls: &'a [DataDecl]) -> PageContext<'a> {
        let mut data: HashMap<&str, Held<'a>> = HashMap::new();
        for decl in decls {
            // Every window's settings, made by Clarity; written data cannot take their place.
            if decl.made {
                data.insert(&decl.name.name, Held::Record(self.settings()));
                continue;
            }
            if let Some(problem) = crate::names::name_problem("data", &decl.name.name) {
                self.error(file, problem, decl.name.span);
            }
            if self.settings().is_some() && decl.name.name == crate::parser::SETTINGS_DATA {
                self.error(file, "every window has 'settings' of its own; give this data another name", decl.name.span);
                continue;
            }
            let held = match &decl.of {
                DataKind::Record { entity } | DataKind::List { entity } if self.settings().is_some() && entity.name == crate::parser::SETTINGS_ENTITY => {
                    self.error(file, "every window has the settings as 'settings'; bind that", entity.span);
                    continue;
                }
                DataKind::Record { entity } | DataKind::List { entity } => {
                    let record = self.decls.get(entity.name.as_str()).is_some_and(|ds| ds.iter().any(|d| d.kind == DeclKind::Record));
                    let found = if record {
                        // A record (manual/code.engines.md) is window data too, made in code and never stored.
                        self.resolve(file, entity, DeclKind::Record).map(|(f, i)| &self.files[f].records[i])
                    } else if self.decls.contains_key(entity.name.as_str()) {
                        self.resolve(file, entity, DeclKind::Entity).map(|(f, i)| &self.files[f].entities[i])
                    } else {
                        self.error(file, format!("unknown entity or enum '{}'", entity.name), entity.span);
                        None
                    };
                    if decl.is_list() { Held::List(found) } else { Held::Record(found) }
                }
                DataKind::Value { ty, default } => {
                    if let crate::code::TypeRefKind::Named { name } = &ty.kind {
                        let ident = Ident { name: name.clone(), span: ty.span };
                        let is_entity = self.decls.get(name.as_str()).is_some_and(|ds| ds.iter().any(|d| d.kind == DeclKind::Entity));
                        if is_entity && default.is_some() {
                            self.error(
                                file,
                                format!("'{}' is a record, which starts new; only a plain value has a default", decl.name.name),
                                ty.span,
                            );
                        } else if self.decls.contains_key(name.as_str()) {
                            self.resolve(file, &ident, DeclKind::Enum);
                        } else {
                            self.error(file, format!("unknown entity or enum '{name}'"), ty.span);
                        }
                    }
                    match shown_as(&ty.kind) {
                        Some(shown) => Held::Value(shown),
                        None => continue,
                    }
                }
            };
            if data.insert(&decl.name.name, held).is_some() {
                self.error(
                    file,
                    format!("duplicate data '{}'", decl.name.name),
                    decl.name.span,
                );
            }
        }
        PageContext { data, popovers: HashSet::new() }
    }

    /// The menu items and buttons a window shows (its own, and the app's) are
    /// members of it beside its data, so they cannot share a name with it.
    pub(super) fn check_commands_and_data(&mut self, file: usize, page: &'a Page) {
        fn named<'c>(items: &'c [Command], out: &mut Vec<(&'c str, Span)>) {
            for c in items {
                if let Some(name) = &c.props.name {
                    out.push((name, c.span));
                }
                named(&c.items, out);
            }
        }
        let mut commands: Vec<(&str, Span, Option<usize>)> = Vec::new();
        let mut collect = |frame: &'a Frame, from: Option<usize>| {
            let mut found = Vec::new();
            if let Some(menu) = &frame.menu {
                named(&menu.items, &mut found);
            }
            if let Some(toolbar) = &frame.toolbar {
                named(&toolbar.items, &mut found);
            }
            if let Some(ribbon) = &frame.ribbon {
                for tab in &ribbon.tabs {
                    for group in &tab.groups {
                        named(&group.items, &mut found);
                    }
                }
            }
            commands.extend(found.into_iter().map(|(n, s)| (n, s, from)));
        };
        collect(&page.frame, None);
        if page.kind == PageKind::Window {
            for (index, f) in self.files.iter().enumerate() {
                for app in &f.apps {
                    collect(&app.frame, Some(index));
                }
            }
        }
        for (name, span, from) in commands {
            if page.data.iter().any(|d| d.name.name == name) {
                // Reported where the item is written.
                self.error(
                    from.unwrap_or(file),
                    format!("item or button '{name}' has the name of data of {}; give one of them another name", page.name.name),
                    span,
                );
            }
        }
    }

    pub(super) fn check_binding(&mut self, file: usize, page: &PageContext<'a>, widget: &Widget) {
        let p = &widget.props;
        let name = widgets::keyword(p.kind);
        let info = widgets::info(p.kind);
        let at = format!("{name} at {},{}", p.col, p.row);
        if p.kind == WidgetKind::Canvas {
            self.check_canvas_mode(file, page, widget, &at);
        }
        if p.kind == WidgetKind::Graph {
            self.check_graph_options(file, page, widget, &at);
            if !p.lists.is_empty() {
                return;
            }
        }

        // A markdown or an image shows one thing: a key, a file of the resources (source), or what it binds.
        if matches!(p.kind, WidgetKind::Markdown | WidgetKind::Image)
            && [p.caption.is_some(), p.source.is_some(), p.bind.is_some()].iter().filter(|given| **given).count() > 1
        {
            self.error(
                file,
                format!("a {name} shows a key, a file of the resources (source) or what it binds: one of them"),
                widget.span,
            );
        }

        let Some(bind) = &p.bind else {
            self.check_range(file, widget, None);
            if !p.columns.is_empty() || !p.edit.is_empty() || !p.show.is_empty() {
                self.error(
                    file,
                    format!("{at} lists fields but is not bound to data"),
                    widget.span,
                );
            }
            return;
        };

        let (data_name, field_name) = match bind.split_once('.') {
            Some((d, f)) => (d, Some(f)),
            None => (bind.as_str(), None),
        };
        let Some(held) = page.data.get(data_name).cloned() else {
            self.error(
                file,
                format!("{at}: unknown data '{data_name}'"),
                widget.span,
            );
            return;
        };
        let (entity, list) = match held {
            Held::Record(entity) => (entity, false),
            Held::List(entity) => (entity, true),
            Held::Value(shown) => {
                // A chart of a list of numbers draws them as they are (manual/parts.data.md).
                if p.kind == WidgetKind::Chart && field_name.is_none() && matches!(shown, FieldType::Values { .. }) {
                    self.check_number_chart(file, &at, widget, &shown);
                    return;
                }
                let problem = match info.bind {
                    _ if field_name.is_some() => Some(format!("'{data_name}' is a plain value; bind it as '{data_name}'")),
                    BindTarget::Nothing => None,
                    BindTarget::Record => Some(format!("a {name} binds to one record, as in 'bind customer'")),
                    BindTarget::Collection => Some(format!("a {name} binds to a list, as in 'data orders: list<Order>'")),
                    BindTarget::Field if !widgets::accepts(p.kind, &shown) => Some(format!(
                        "'{data_name}' is {}; a {name} shows {}",
                        widgets::field_type_tag(&shown),
                        widgets::accepted_types(p.kind).join(", ")
                    )),
                    BindTarget::Field => None,
                };
                if let Some(problem) = problem {
                    self.error(file, format!("{at}: {problem}"), widget.span);
                } else {
                    self.check_range(file, widget, None);
                }
                return;
            }
        };
        let Some(entity) = entity else { return };
        // A hashed field keeps a hash, which no one reads (docs/ledger/signin.md).
        let named = field_name.into_iter().chain(p.columns.iter().chain(&p.edit).chain(&p.show).map(String::as_str));
        if let Some(hashed) = named.filter(|f| entity.field(f).is_some_and(|f| f.modifiers.hashed)).next() {
            self.error(file, format!("{at}: {hashed} is hashed, so no part shows it"), widget.span);
            return;
        }

        match info.bind {
            BindTarget::Nothing => {}
            BindTarget::Record => {
                if list || field_name.is_some() {
                    self.error(
                        file,
                        format!("{at}: a {name} binds to one record, as in 'bind customer'"),
                        widget.span,
                    );
                }
            }
            BindTarget::Field => {
                if list {
                    self.error(
                        file,
                        format!("{at}: '{data_name}' is a list; a {name} binds to a field of a single record"),
                        widget.span,
                    );
                    return;
                }
                let Some(field_name) = field_name else {
                    self.error(
                        file,
                        format!("{at}: bind a field, as in '{data_name}.name'"),
                        widget.span,
                    );
                    return;
                };
                match entity.field(field_name) {
                    None => self.error(
                        file,
                        format!("{at}: '{}' has no field '{field_name}'", entity.name.name),
                        widget.span,
                    ),
                    Some(field) if !widgets::accepts(p.kind, &field.ty) => self.error(
                        file,
                        format!(
                            "{at}: '{field_name}' is {}; a {name} shows {}",
                            widgets::field_type_tag(&field.ty),
                            widgets::accepted_types(p.kind).join(", ")
                        ),
                        widget.span,
                    ),
                    // Only a password input shows a secret, as dots (manual/data.settings.md).
                    Some(field) if field.ty == FieldType::Secret && p.kind != WidgetKind::Password => self.error(
                        file,
                        format!("{at}: '{field_name}' is secret; only a password input shows it, as dots"),
                        widget.span,
                    ),
                    // A blob reads the bytes of a file that is there; only a file field may name one that is not yet.
                    Some(field) if p.new_file && field.ty == FieldType::Blob => self.error(
                        file,
                        format!("{at}: '{field_name}' is a blob, which keeps a file that is there; 'new file' is for a file field"),
                        widget.span,
                    ),
                    Some(field) if p.kind == WidgetKind::Dropdown => self.check_dropdown(file, &at, widget, &field.ty),
                    Some(field) => self.check_range(file, widget, Some((field_name, &field.ty, &field.modifiers))),
                }
            }
            // A chart of a record's list of numbers (manual/parts.data.md).
            BindTarget::Collection
                if p.kind == WidgetKind::Chart
                    && !list
                    && let Some(field) = field_name.and_then(|f| entity.field(f))
                    && matches!(field.ty, FieldType::Values { .. }) =>
            {
                let ty = field.ty.clone();
                self.check_number_chart(file, &at, widget, &ty);
            }
            BindTarget::Collection => {
                if !list || field_name.is_some() {
                    self.error(
                        file,
                        format!("{at}: a {name} binds to a list, as in 'data orders: list<Order>'"),
                        widget.span,
                    );
                    return;
                }
                if p.cards && p.show.is_empty() {
                    self.error(file, format!("{at}: 'cards' shows the fields of 'show': write 'show title, kind cards'"), widget.span);
                }
                if let Some(chip) = p.card_chip.as_ref().filter(|c| !p.show.contains(c)) {
                    self.error(file, format!("{at}: the chip is one of the shown fields ({}), not '{chip}'", p.show.join(", ")), widget.span);
                }
                if let Some(field) = &p.card_color
                    && !p.cards
                {
                    self.error(file, format!("{at}: 'colored by' colours a list's cards: write 'cards colored by {field}'"), widget.span);
                }
                if let Some(field) = &p.card_color {
                    match entity.field(field) {
                        None => self.error(file, format!("{at}: '{}' has no field '{field}'", entity.name.name), widget.span),
                        Some(f) if f.ty != FieldType::Color => self.error(file, format!("{at}: '{field}' is {}; the cards are colored by a color field", widgets::field_type_tag(&f.ty)), widget.span),
                        Some(_) => {}
                    }
                }
                for field in p.columns.iter().chain(&p.show) {
                    match entity.field(field) {
                        None => self.error(
                            file,
                            format!("{at}: '{}' has no field '{field}'", entity.name.name),
                            widget.span,
                        ),
                        Some(f) if matches!(f.ty, FieldType::List { .. }) => self.error(
                            file,
                            format!("{at}: list field '{field}' cannot be shown in a {name}"),
                            widget.span,
                        ),
                        Some(f) if f.ty == FieldType::Image => self.error(
                            file,
                            format!("{at}: '{field}' is a picture; show it in an image part"),
                            widget.span,
                        ),
                        Some(f) if f.ty == FieldType::Blob => self.error(
                            file,
                            format!("{at}: '{field}' is a blob; show it in a file part"),
                            widget.span,
                        ),
                        Some(_) => {}
                    }
                }
                if let Some(order) = p.order.as_ref().filter(|o| !p.columns.contains(&o.field)) {
                    self.error(file, format!("{at}: '{}' is not a column of the table; order by one it shows", order.field), widget.span);
                }
                self.check_edits(file, &at, entity, widget);
                self.check_reorder(file, &at, entity, widget);
                if matches!(p.kind, WidgetKind::Tree | WidgetKind::TreeTable) {
                    self.check_tree(file, &at, entity, widget);
                }
                if p.kind == WidgetKind::Chart {
                    self.check_chart(file, &at, entity, widget);
                }
                if p.kind == WidgetKind::Graph {
                    self.check_graph(file, page, &at, entity, widget);
                }
            }
        }
    }

    pub(super) fn check_field(&mut self, file: usize, field: &Field) {
        let m = &field.modifiers;
        match &field.ty {
            FieldType::Text {
                max_length: Some(0),
            } => {
                self.error(file, "text length must be at least 1", field.span);
            }
            FieldType::Decimal { precision, scale } if *precision == 0 || scale > precision => {
                self.error(
                    file,
                    "decimal precision must be at least 1 and not less than the scale",
                    field.span,
                );
            }
            FieldType::Enum { name } => {
                self.resolve(file, name, DeclKind::Enum);
            }
            FieldType::Ref { entity } | FieldType::List { item: entity } | FieldType::Record { entity } => {
                self.resolve(file, entity, DeclKind::Entity);
            }
            FieldType::Values { item: crate::model::ListItem::Enum { name } } => {
                self.resolve(file, name, DeclKind::Enum);
            }
            _ => {}
        }

        let name = &field.name.name;
        if m.owned && !matches!(field.ty, FieldType::List { .. }) {
            self.error(
                file,
                format!("'owned' only applies to lists ('{name}')"),
                field.span,
            );
        }
        if m.store.is_some() && !matches!(field.ty, FieldType::Image | FieldType::Blob) {
            self.error(file, format!("only an image or a blob field has a store ('{name}')"), field.span);
        }
        if !m.extensions.is_empty() && !matches!(field.ty, FieldType::Folder | FieldType::File | FieldType::Blob) {
            self.error(file, format!("only a folder, file or blob field has extensions ('{name}')"), field.span);
        }
        if m.hidden && !matches!(field.ty, FieldType::Folder | FieldType::File | FieldType::Blob) {
            self.error(file, format!("only a folder, file or blob field has hidden files ('{name}')"), field.span);
        }
        // A full-text index reads words: text, and lists of text.
        if m.search
            && !matches!(
                field.ty,
                FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Values { item: crate::model::ListItem::Text }
            )
        {
            self.error(file, format!("only text and lists of text can be searched ('{name}')"), field.span);
        }
        if m.cascade && !matches!(field.ty, FieldType::Ref { .. }) {
            self.error(file, format!("only a reference cascades: deleting the record it refers to deletes this one ('{name}')"), field.span);
        }
        if field.ty == FieldType::Blob {
            if m.unique {
                self.error(file, format!("a blob cannot be unique ('{name}')"), field.span);
            }
            if m.default.is_some() {
                self.error(file, format!("a blob has no default; it starts empty ('{name}')"), field.span);
            }
        }
        if matches!(field.ty, FieldType::Values { .. }) {
            if m.unique {
                self.error(file, format!("a list cannot be unique ('{name}')"), field.span);
            }
            if m.default.is_some() {
                self.error(file, format!("a list has no default; it starts empty ('{name}')"), field.span);
            }
        }
        if field.ty == FieldType::Image {
            if m.unique {
                self.error(file, format!("a picture cannot be unique ('{name}')"), field.span);
            }
            if m.default.is_some() {
                self.error(file, format!("a picture has no default; it starts empty ('{name}')"), field.span);
            }
        }
        if m.pattern.is_some() && !field.ty.is_textual() {
            self.error(
                file,
                format!("'pattern' only applies to text fields ('{name}')"),
                field.span,
            );
        }
        if let Some(problem) = m.format.as_deref().and_then(|f| crate::formats::problem(&field.ty, name, f)) {
            self.error(file, problem, field.span);
        }
        if m.computed && m.default.is_some() {
            self.error(
                file,
                format!("a computed field cannot have a default ('{name}')"),
                field.span,
            );
        }
        if m.computed && m.required {
            self.warning(
                file,
                format!("'required' has no effect on a computed field ('{name}')"),
                field.span,
            );
        }
        for (what, literal) in [("min", &m.min), ("max", &m.max)] {
            if let Some(literal) = literal
                && !self.bound_fits(&field.ty, literal)
            {
                self.error(
                    file,
                    format!("'{what}' needs a number for numeric fields or a date string for dates ('{name}')"),
                    field.span,
                );
            }
        }
        if let (Some(Literal::Number(min)), Some(Literal::Number(max))) = (&m.min, &m.max)
            && let (Ok(min), Ok(max)) = (min.parse::<f64>(), max.parse::<f64>())
            && min > max
        {
            self.error(
                file,
                format!("'min' is greater than 'max' ('{name}')"),
                field.span,
            );
        }
        if let Some(default) = m.default.as_ref().filter(|_| !matches!(field.ty, FieldType::Image | FieldType::Blob | FieldType::Values { .. })) {
            self.check_default(file, field, default);
        }
    }

    pub(super) fn bound_fits(&self, ty: &FieldType, literal: &Literal) -> bool {
        match ty {
            t if t.is_numeric() => matches!(literal, Literal::Number(_)),
            FieldType::Date | FieldType::DateTime => matches!(literal, Literal::Str(_)),
            _ => false,
        }
    }

    pub(super) fn check_default(&mut self, file: usize, field: &Field, default: &Literal) {
        let name = &field.name.name;
        let ok = match (&field.ty, default) {
            (FieldType::Bool, Literal::Bool(_)) => true,
            (FieldType::Color, Literal::Str(text)) => {
                if !crate::colors::is_kept(text) {
                    self.error(file, format!("{} ('{name}')", crate::colors::COLOR_PROBLEM), field.span);
                }
                true
            }
            // A secret's default is refused on its own (check_settings).
            (FieldType::Secret, _) => true,
            (t, Literal::Number(_)) if t.is_numeric() => true,
            (t, Literal::Str(_)) if t.is_textual() => true,
            // Written as the backends send it, which every generator writes out (crate::dates).
            (FieldType::Date, Literal::Str(text)) => {
                if crate::dates::date(text).is_none() {
                    self.error(file, format!("'{text}' is no date: a date is written {} ('{name}')", crate::dates::DATE_SHAPE), field.span);
                }
                true
            }
            (FieldType::DateTime, Literal::Str(text)) => {
                if crate::dates::date_time(text).is_none() {
                    self.error(file, format!("'{text}' is no date and time: a date and time is written {} ('{name}')", crate::dates::DATE_TIME_SHAPE), field.span);
                }
                true
            }
            (FieldType::Enum { name: enum_name }, Literal::Ident(variant)) => {
                if let Some(decl) = self.enum_by_name(file, &enum_name.name)
                    && !decl.variants.iter().any(|v| &v.name.name == variant)
                {
                    self.error(
                        file,
                        format!(
                            "'{variant}' is not a variant of '{}' ('{name}')",
                            enum_name.name
                        ),
                        field.span,
                    );
                }
                true
            }
            _ => false,
        };
        if !ok {
            self.error(
                file,
                format!("default value does not match the field type ('{name}')"),
                field.span,
            );
        }
    }

    pub(super) fn enum_by_name(&self, file: usize, name: &str) -> Option<&'a crate::model::EnumDecl> {
        let visible = self.visible_modules(file);
        let decls = self.decls.get(name)?;
        let found: Vec<_> = decls
            .iter()
            .filter(|d| d.kind == DeclKind::Enum && visible.contains(d.module))
            .collect();
        match found.as_slice() {
            [d] => Some(&self.files[d.file].enums[d.index]),
            _ => None,
        }
    }

    /// `hashed` (docs/ledger/signin.md): text of a stored
    /// entity, long enough for a hash, and none of what reads or shows its
    /// value: a hash is never searched, has no default, pattern, bounds or format.
    pub(super) fn check_hashed(&mut self, file: usize, stored: bool, field: &Field) {
        let m = &field.modifiers;
        if !m.hashed {
            return;
        }
        let name = &field.name.name;
        if !stored {
            self.error(file, format!("{name}: only a stored entity's field is hashed; a record or a setting keeps its value as it is"), field.span);
            return;
        }
        match field.ty {
            FieldType::Text { max_length: Some(n) } if n < HASH_LENGTH => {
                self.error(file, format!("{name}: a hash is longer than {n} characters; write text without a length"), field.span);
            }
            FieldType::Text { .. } => {}
            _ => self.error(file, format!("{name}: only text is hashed, not {}", crate::dictionary::type_text(&field.ty)), field.span),
        }
        if m.search {
            self.error(file, format!("{name}: a hashed field is not searched; take 'search' away"), field.span);
        }
        if m.computed {
            self.error(file, format!("{name}: a hashed field is set by code, not computed; take 'computed' away"), field.span);
        }
        let shown = [(m.default.is_some(), "default"), (m.pattern.is_some(), "pattern"), (m.min.is_some(), "min"), (m.max.is_some(), "max"), (m.format.is_some(), "format")];
        for (_, word) in shown.iter().filter(|(given, _)| *given) {
            self.error(file, format!("{name}: a hashed field has no {word}; its value is a hash"), field.span);
        }
    }
}
