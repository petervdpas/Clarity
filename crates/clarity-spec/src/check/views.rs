//! The checker's rules of content and views (docs/ledger/ui-content.md):
//! the views a window shows, forms, parts beside others, and part names.

use super::*;

impl<'a> Checker<'a> {
    /// The views a window's slide-outs, tabs, steps and sections show (`view
    /// ClassEditor`, manual/design.windows.md): each a view of the project, shown once, in
    /// a window, its parts and data named apart from the window's, and the
    /// place holding none of its own.
    pub(super) fn check_views(&mut self, file: usize, page: &'a Page) {
        fn names<'w>(widgets: &'w [Widget], out: &mut Vec<&'w str>) {
            for w in widgets {
                out.extend(w.props.name.as_deref());
                names(&w.children, out);
                for inner in &w.pages {
                    names(&inner.children, out);
                }
            }
        }
        if page.kind != PageKind::View
            && let Some(first) = page.tells.first()
        {
            self.error(file, format!("a {} tells nothing; a view tells the window it shows in", page_kind_word(page.kind)), first.span);
        }
        // A slide-out hears what its view tells; one without a view has nothing to hear.
        fn deaf<'w>(widgets: &'w [Widget], out: &mut Vec<&'w Widget>) {
            for w in widgets {
                if w.props.kind == WidgetKind::Slideout && w.props.view.is_none() && !w.props.tells.is_empty() {
                    out.push(w);
                }
                deaf(&w.children, out);
            }
        }
        let mut unheard = Vec::new();
        deaf(&page.widgets, &mut unheard);
        for w in unheard {
            self.error(file, format!("a slide-out hears what the view it shows tells; it shows no view, so 'on {}' has nothing to hear", w.props.tells[0].tell), w.span);
        }
        let mut own = Vec::new();
        names(&page.widgets, &mut own);
        own.extend(page.data.iter().map(|d| d.name.name.as_str()));
        // Each name the window holds, and whose it is: its own, then each view's it places, whose handlers join its code.
        let window = page.name.name.as_str();
        let handlers_of = |page_name: &str| -> Vec<String> {
            self.files.iter().flat_map(|f| &f.code).filter(|c| c.name.name == page_name).flat_map(|c| c.handlers.iter().map(|h| h.name.name.clone())).collect()
        };
        let mut taken: Vec<(String, String)> = own.iter().map(|n| (n.to_string(), window.to_string())).collect();
        let mut handlers_taken: Vec<(String, String)> = handlers_of(window).into_iter().map(|h| (h, window.to_string())).collect();
        let mut shown: Vec<&str> = Vec::new();
        for place in crate::views::places(&page.widgets) {
            let (name, what, span) = (place.view, place.what, place.span);
            // A view shows only in a window's content; without content nothing shows it.
            if what != "content" {
                self.error(file, format!("a {what} does not show a view: a view shows in content; put a content in the {what} with view {name}"), span);
                continue;
            }
            if place.holds_parts {
                self.error(file, format!("a view line of content holds no parts of its own; they are {name}'s"), span);
            }
            // A view may hold content of its own (views nest), but never shows itself.
            if page.kind == PageKind::View && name == page.name.name {
                self.error(file, format!("{name} shows itself"), span);
                continue;
            }
            let Some(found) = self.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == name) else {
                self.error(file, format!("there is no view '{name}'"), span);
                continue;
            };
            if found.kind != PageKind::View {
                self.error(file, format!("{name} is a {}; content shows a view: write page {name}: view", page_kind_word(found.kind)), span);
                continue;
            }
            if shown.contains(&name) {
                self.error(file, format!("{name} shows once in a window"), span);
                continue;
            }
            shown.push(name);
            for heard in place.tells {
                if !found.tells.iter().any(|t| t.name.name == heard.tell) {
                    self.error(file, format!("{name} tells no '{}'; its page says what it tells", heard.tell), span);
                }
            }
            // A slide-out stands on the window, so a view that has one shows nowhere.
            if found.widgets.iter().any(|w| w.props.kind == WidgetKind::Slideout && w.props.beside.is_none()) {
                self.error(file, format!("{name} has a slide-out of its own, so it cannot show in a {what}"), span);
            }
            let mut theirs = Vec::new();
            names(&found.widgets, &mut theirs);
            theirs.extend(found.data.iter().map(|d| d.name.name.as_str()));
            // The project's settings are every page's: the same data, no clash.
            let mut clashes: Vec<(String, String)> = Vec::new();
            for n in theirs.iter().filter(|n| **n != crate::parser::SETTINGS_DATA) {
                match taken.iter().find(|(t, _)| t == n) {
                    Some((_, whose)) => clashes.push((n.to_string(), whose.clone())),
                    None => taken.push((n.to_string(), name.to_string())),
                }
            }
            // Its handlers join the window's code, beside the window's own and those of the other views it places.
            for h in handlers_of(name) {
                match handlers_taken.iter().find(|(t, _)| *t == h) {
                    Some((_, whose)) => clashes.push((format!("handler {h}"), whose.clone())),
                    None => handlers_taken.push((h, name.to_string())),
                }
            }
            for (clash, whose) in clashes {
                let clash = clash.strip_prefix("handler ").map(|h| format!("handler '{h}'")).unwrap_or(format!("'{clash}'"));
                self.error(file, format!("{name} has {clash}, as {whose} has; name one of them apart"), span);
            }
        }
    }

    /// A form holds the inputs of the one record it binds, and no other
    /// form.
    pub(super) fn check_forms(&mut self, file: usize, widgets: &[Widget], form: Option<&Widget>) {
        for widget in widgets {
            let p = &widget.props;
            let at = format!("{} at {},{}", widgets::keyword(p.kind), p.col, p.row);
            let mut inner = form;
            if p.kind == WidgetKind::Form {
                if form.is_some() {
                    self.error(file, format!("{at}: a form cannot hold another form"), widget.span);
                }
                inner = Some(widget);
            } else if let (Some(form), Some(bind)) = (form, &p.bind)
                && widgets::info(p.kind).input
            {
                let data = bind.split('.').next().unwrap_or_default();
                let held = form.props.bind.as_deref().unwrap_or_default();
                let held = held.split('.').next().unwrap_or_default();
                if data != held {
                    let which = match &form.props.name {
                        Some(name) => format!("'{name}'"),
                        None => format!("at {},{}", form.props.col, form.props.row),
                    };
                    self.error(
                        file,
                        format!("{at}: binds to '{data}', but its form {which} holds '{held}'"),
                        widget.span,
                    );
                }
            }
            self.check_forms(file, &widget.children, inner);
            for page in &widget.pages {
                self.check_forms(file, &page.children, inner);
            }
        }
    }

    /// `beside part`: the part stands in the same container as the slide-out
    /// and has a place there, so the slide-out lies against its edge.
    pub(super) fn check_beside(&mut self, file: usize, widgets: &[Widget]) {
        for w in widgets {
            if let Some(part) = &w.props.beside {
                match widgets.iter().find(|o| o.props.name.as_deref() == Some(part.as_str())) {
                    None => self.error(file, format!("there is no part '{part}' beside this slide-out; it lies against a part of the container it is in"), w.span),
                    Some(o) if matches!(o.props.kind, WidgetKind::Popover | WidgetKind::Slideout) || widgets::off_grid(o.props.kind) => {
                        self.error(file, format!("'{part}' has no place of its own to lie against"), w.span)
                    }
                    Some(_) => {}
                }
            }
            self.check_beside(file, &w.children);
            for page in &w.pages {
                self.check_beside(file, &page.children);
            }
        }
    }

    pub(super) fn check_names(&mut self, file: usize, widgets: &[Widget], frame: &Frame) {
        fn collect<'w>(widgets: &'w [Widget], out: &mut Vec<&'w Widget>) {
            for w in widgets {
                out.push(w);
                collect(&w.children, out);
                for inner in &w.pages {
                    collect(&inner.children, out);
                }
            }
        }
        let mut all = Vec::new();
        collect(widgets, &mut all);
        for sidebar in &frame.sidebars {
            collect(&sidebar.widgets, &mut all);
        }
        if let Some(footer) = &frame.footer {
            collect(&footer.widgets, &mut all);
        }
        fn commands<'c>(items: &'c [Command], out: &mut Vec<(&'c str, Span)>) {
            for c in items {
                if let Some(name) = &c.props.name {
                    out.push((name, c.span));
                }
                commands(&c.items, out);
            }
        }
        let mut named: Vec<(&str, Span)> = all
            .iter()
            .filter_map(|w| w.props.name.as_deref().map(|n| (n, w.span)))
            .collect();
        for menu in all.iter().filter_map(|w| w.menu.as_ref()) {
            named.extend(menu.name.as_deref().map(|n| (n, menu.span)));
            commands(&menu.items, &mut named);
        }
        if let Some(menu) = &frame.menu {
            named.extend(menu.name.as_deref().map(|n| (n, menu.span)));
            commands(&menu.items, &mut named);
        }
        if let Some(toolbar) = &frame.toolbar {
            named.extend(toolbar.name.as_deref().map(|n| (n, toolbar.span)));
            commands(&toolbar.items, &mut named);
        }
        if let Some(ribbon) = &frame.ribbon {
            for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
                commands(&group.items, &mut named);
            }
        }
        let mut seen = HashSet::new();
        for (name, span) in named {
            if !seen.insert(name) {
                self.error(file, format!("duplicate part name '{name}'"), span);
            }
        }

        // A label or help text belongs to an input of the same window.
        for widget in &all {
            let p = &widget.props;
            let Some(target) = &p.for_part else { continue };
            let at = format!("{} at {},{}", widgets::keyword(p.kind), p.col, p.row);
            let found = all.iter().find(|w| w.props.name.as_deref() == Some(target.as_str()));
            match found {
                None => self.error(
                    file,
                    format!("{at}: there is no part '{target}' in this window"),
                    widget.span,
                ),
                // A popover opens next to any part.
                Some(_) if p.kind == WidgetKind::Popover => {}
                Some(other) if !widgets::info(other.props.kind).input => self.error(
                    file,
                    format!(
                        "{at}: '{target}' is {} {}, not an input",
                        article(widgets::keyword(other.props.kind)),
                        widgets::keyword(other.props.kind)
                    ),
                    widget.span,
                ),
                Some(_) => {}
            }
        }

        // Enter clicks the default button and Escape the cancel button, so
        // a window has at most one of each.
        let defaults: Vec<&&Widget> = all.iter().filter(|w| w.props.default_button).collect();
        if let Some(extra) = defaults.get(1) {
            self.error(file, "more than one default button in this window", extra.span);
        }
        let cancels: Vec<&&Widget> = all.iter().filter(|w| w.props.cancel).collect();
        if let Some(extra) = cancels.get(1) {
            self.error(file, "more than one cancel button in this window", extra.span);
        }
    }
}
