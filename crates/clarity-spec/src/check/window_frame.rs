//! The checker's rules of a window's frame: its keys, sidebars, regions and
//! bars, its frame and commands, and what a click opens.

use super::*;

impl<'a> Checker<'a> {
    /// One key runs one command where it is active (manual/app.keys.md): in the
    /// window's frame, when it has regions of its own (the app's alone are
    /// checked with the app), and among the window's own buttons, which may
    /// take a frame's key, since what is innermost goes first.
    pub(super) fn check_page_keys(&mut self, file: usize, page: &'a Page) {
        let place = format!("{} {}", page_kind_word(page.kind), page.name.name);
        let own = page.frame.menu.is_some() || page.frame.toolbar.is_some() || page.frame.ribbon.is_some();
        if own && page.kind != PageKind::Dialog {
            let found = self.files.iter().enumerate().find_map(|(i, f)| f.apps.first().map(|a| (i, a)));
            let mut keyed = Vec::new();
            let pick = |mine: bool| if mine { file } else { found.map_or(file, |(i, _)| i) };
            let app_frame = found.map(|(_, a)| &a.frame);
            let menu = if page.frame.menu.is_some() { (&page.frame.menu, pick(true)) } else { (app_frame.map_or(&page.frame.menu, |f| &f.menu), pick(false)) };
            let toolbar = if page.frame.toolbar.is_some() { (&page.frame.toolbar, pick(true)) } else { (app_frame.map_or(&page.frame.toolbar, |f| &f.toolbar), pick(false)) };
            let ribbon = if page.frame.ribbon.is_some() { (&page.frame.ribbon, pick(true)) } else { (app_frame.map_or(&page.frame.ribbon, |f| &f.ribbon), pick(false)) };
            frame_keys(menu.1, menu.0, &None, &None, &mut keyed);
            frame_keys(toolbar.1, &None, toolbar.0, &None, &mut keyed);
            frame_keys(ribbon.1, &None, &None, ribbon.0, &mut keyed);
            self.key_clashes(keyed, &format!("{place}'s frame"));
        }
        fn buttons<'w>(file: usize, widgets: &'w [Widget], out: &mut Vec<Keyed<'w>>) {
            for w in widgets {
                if let Some(key) = &w.props.shortcut {
                    let click = w.props.handlers.iter().find(|h| h.event == crate::widgets::Event::Click);
                    let does = match (&w.props.on_click, click) {
                        (Some(target), _) => Does::Open(target, w.props.new_window),
                        (None, Some(h)) => Does::Run(&h.handler, &h.args),
                        (None, None) => Does::Only(w.span),
                    };
                    out.push(Keyed { key, name: w.props.name.clone().unwrap_or_else(|| widgets::keyword(w.props.kind).to_string()), does, file, span: w.span });
                }
                buttons(file, &w.children, out);
                for p in &w.pages {
                    buttons(file, &p.children, out);
                }
            }
        }
        let mut keyed = Vec::new();
        buttons(file, &page.widgets, &mut keyed);
        for sidebar in &page.frame.sidebars {
            buttons(file, &sidebar.widgets, &mut keyed);
        }
        if let Some(footer) = &page.frame.footer {
            buttons(file, &footer.widgets, &mut keyed);
        }
        self.key_clashes(keyed, &place);
    }

    /// Reports a key that runs two different commands in one place.
    pub(super) fn key_clashes(&mut self, keyed: Vec<Keyed<'_>>, place: &str) {
        for (i, later) in keyed.iter().enumerate() {
            if let Some(first) = keyed[..i].iter().find(|k| k.key == later.key && k.does != later.does) {
                self.error(later.file, format!("key '{}' runs both {} and {} in {place}; give one of them another key", later.key, first.name, later.name), later.span);
            }
        }
    }

    /// Part names are unique in a window, its sidebars and its footer
    /// included: the code behind refers to parts by name.
    /// A sidebar's name, its width and limits, and what the person may do with it.
    pub(super) fn check_sidebar(&mut self, file: usize, sidebar: &crate::model::Sidebar) {
        let p = &sidebar.props;
        let side = crate::frame::side_keyword(p.side);
        let shown = p.name.clone().unwrap_or_else(|| side.to_string());
        let mut problems = Vec::new();
        match p.name.as_deref() {
            None => problems.push(format!("sidebar {side} has no name; every part has one (Name them, in Problems, names them all)")),
            Some(name) if !crate::names::is_camel_case(name) => problems.push(crate::names::camel_case_problem("part", name)),
            Some(_) => {}
        }
        let (least, most) = crate::model::SIDEBAR_WIDTHS;
        for width in [Some(p.width), p.min, p.max].into_iter().flatten() {
            if !(least..=most).contains(&width) {
                problems.push(format!("a sidebar is {least} to {most} pixels wide"));
                break;
            }
        }
        if (p.min.is_some() || p.max.is_some()) && !p.resizable {
            problems.push(format!("sidebar {shown}: 'min' and 'max' are for a resizable sidebar; add 'resizable'"));
        }
        if let Some(min) = p.min.filter(|m| *m > p.width) {
            problems.push(format!("sidebar {shown}: min {min} is more than its width {}", p.width));
        }
        if let Some(max) = p.max.filter(|m| *m < p.width) {
            problems.push(format!("sidebar {shown}: max {max} is less than its width {}", p.width));
        }
        if p.collapsed && !p.collapsible {
            problems.push(format!("sidebar {shown}: 'collapsed' needs 'collapsible': write 'collapsible collapsed'"));
        }
        for problem in problems {
            self.error(file, problem, sidebar.span);
        }
    }

    /// A menu and a toolbar are parts, with camelCase names.
    pub(super) fn check_region_name(&mut self, file: usize, name: &Option<String>, what: &str, span: Span) {
        match name.as_deref() {
            None => self.error(file, format!("{what} has no name; every part has one (Name them, in Problems, names them all)"), span),
            Some(n) if !crate::names::is_camel_case(n) => self.error(file, crate::names::camel_case_problem("part", n), span),
            Some(_) => {}
        }
    }

    /// A menu bar's or toolbar's width, when it has one, in the widths a bar may have.
    pub(super) fn check_bar_width(&mut self, file: usize, width: Option<u32>, what: &str, span: Span) {
        let (least, most) = crate::model::BAR_WIDTHS;
        if width.is_some_and(|w| !(least..=most).contains(&w)) {
            self.error(file, format!("{what} is {least} to {most} pixels wide"), span);
        }
    }

    pub(super) fn check_frame(&mut self, file: usize, context: &PageContext<'a>, frame: &'a Frame) {
        if let Some(menu) = &frame.menu {
            if menu.items.is_empty() {
                self.warning(file, "the menu has no items yet", menu.span);
            }
            self.check_region_name(file, &menu.name, "the menu", menu.span);
            self.check_bar_width(file, menu.width, "a menu bar", menu.span);
            self.check_commands(file, &menu.items);
        }
        if let Some(toolbar) = &frame.toolbar {
            if toolbar.items.is_empty() {
                self.warning(file, "the toolbar has no buttons yet", toolbar.span);
            }
            self.check_region_name(file, &toolbar.name, "the toolbar", toolbar.span);
            self.check_bar_width(file, toolbar.width, "a toolbar", toolbar.span);
            self.check_commands(file, &toolbar.items);
        }
        if let Some(ribbon) = &frame.ribbon {
            if ribbon.tabs.is_empty() {
                self.warning(file, "the ribbon has no tabs yet", ribbon.span);
            }
            // The ribbon, its tabs and its groups are parts, with names.
            let named = |name: &Option<String>, what: String| match name.as_deref() {
                None => Some(format!("{what} has no name; every part has one (Name them, in Problems, names them all)")),
                Some(n) if !crate::names::is_camel_case(n) => Some(crate::names::camel_case_problem("part", n)),
                Some(_) => None,
            };
            let mut problems = Vec::new();
            problems.extend(named(&ribbon.name, "the ribbon".to_string()).map(|p| (p, ribbon.span)));
            if ribbon.collapsed && !ribbon.collapsible {
                problems.push(("the ribbon: 'collapsed' needs 'collapsible': write 'collapsible collapsed'".to_string(), ribbon.span));
            }
            for tab in &ribbon.tabs {
                problems.extend(named(&tab.name, format!("tab \"{}\"", tab.title)).map(|p| (p, tab.span)));
                for group in &tab.groups {
                    problems.extend(named(&group.name, format!("group \"{}\"", group.title)).map(|p| (p, group.span)));
                }
            }
            for (problem, span) in problems {
                self.error(file, problem, span);
            }
            for tab in &ribbon.tabs {
                for group in &tab.groups {
                    self.check_commands(file, &group.items);
                }
            }
        }
        for sidebar in &frame.sidebars {
            self.check_sidebar(file, sidebar);
            if sidebar.props.columns == 0 || sidebar.props.columns > MAX_PAGE_COLUMNS {
                self.error(file, format!("a sidebar's grid has 1 to {MAX_PAGE_COLUMNS} columns"), sidebar.span);
            } else {
                self.check_widgets(file, context, &sidebar.widgets, sidebar.props.columns);
            }
        }
        if let Some(footer) = &frame.footer {
            self.check_widgets(file, context, &footer.widgets, footer.columns);
            for widget in &footer.widgets {
                let p = &widget.props;
                if p.row != 1 || p.height != 1 {
                    self.error(
                        file,
                        format!(
                            "{} at {},{}: a footer has one row",
                            widgets::keyword(p.kind),
                            p.col,
                            p.row
                        ),
                        widget.span,
                    );
                }
            }
        }
    }

    pub(super) fn check_commands(&mut self, file: usize, commands: &[Command]) {
        for command in commands {
            let p = &command.props;
            // Every menu item and button has a name, in camelCase, as a part has.
            if p.kind != CommandKind::Separator {
                match p.name.as_deref() {
                    None => self.error(
                        file,
                        format!(
                            "{} \"{}\" has no name; every menu item and button has one (Name them, in Problems, names them all)",
                            frame::command_keyword(p.kind),
                            p.caption.as_deref().unwrap_or_default()
                        ),
                        command.span,
                    ),
                    Some(name) if !crate::names::is_camel_case(name) => {
                        self.error(file, crate::names::camel_case_problem("item or button", name), command.span)
                    }
                    Some(_) => {}
                }
            }
            if let Some(icon) = p.icon.as_deref().filter(|i| crate::icons::path(i).is_none()) {
                self.error(file, crate::icons::unknown(icon), command.span);
            }
            if p.kind == CommandKind::Item && !command.items.is_empty() && p.on_click.is_some() {
                self.error(
                    file,
                    "an item with a submenu opens the submenu; remove its 'on click'",
                    command.span,
                );
            }
            if let Some(target) = &p.on_click
                && p.new_window
                && frame::BUILT_IN_COMMANDS.contains(&target.as_str())
            {
                self.error(file, "only a window opens in a new window", command.span);
            }
            if let Some(target) = p.on_click.as_deref().filter(|t| frame::is_signin_target(t)) {
                self.check_signin_target(file, target, false, p.new_window, command.span);
            } else if let Some(target) = &p.on_click
                && !frame::BUILT_IN_COMMANDS.contains(&target.as_str())
            {
                self.check_click_target(file, target, &p.open_with, None, command.span);
            }
            self.check_commands(file, &command.items);
        }
    }

    /// A click opens a window. Unknown names are warnings: they become
    /// event handlers once code behind is part of the language.
    pub(super) fn check_click_target(&mut self, file: usize, target: &str, with: &[String], here: Option<&PageContext<'a>>, span: Span) {
        // A popover or slide-out of the window opens as a window does.
        if self.overlays.contains(target) {
            if !with.is_empty() {
                self.error(file, format!("'{target}' is part of this window; only a window opens with data"), span);
            }
            return;
        }
        // `with book`: the opened window has data `book`, and a part's own window holds the same.
        let opened = self.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == target);
        for name in with {
            let Some(page) = opened else { break };
            let Some(theirs) = crate::views::page_data(self.files.iter(), page).into_iter().find(|d| d.name.name == *name) else {
                self.error(file, format!("{target} has no data '{name}' to open it with"), span);
                continue;
            };
            let Some(here) = here else { continue };
            let wanted = match &theirs.of {
                DataKind::Record { entity } => Some((false, entity.name.as_str())),
                DataKind::List { entity } => Some((true, entity.name.as_str())),
                _ => None,
            };
            match (here.data.get(name.as_str()), wanted) {
                (None, _) => self.error(file, format!("this window has no data '{name}' to open {target} with"), span),
                (Some(Held::Record(Some(e))), Some((false, want))) | (Some(Held::List(Some(e))), Some((true, want))) if e.name.name != want => {
                    self.error(file, format!("'{name}' here is {}, and {target}'s '{name}' is {want}", e.name.name), span);
                }
                (Some(Held::Record(_)), Some((true, _))) | (Some(Held::List(_)), Some((false, _))) | (Some(Held::Value(_)), Some(_)) | (Some(Held::Record(_) | Held::List(_)), None) => {
                    self.error(file, format!("'{name}' here and {target}'s '{name}' hold different kinds of data"), span);
                }
                _ => {}
            }
        }
        // The splash screen is the application's to show while it starts.
        if self.files.iter().flat_map(|f| &f.pages).any(|p| p.name.name == target && p.kind == PageKind::Splash) {
            self.error(file, format!("{target} is the splash screen, which the application shows itself as it starts; nothing opens it"), span);
            return;
        }
        // So is the sign-in window, before anyone signs in and after a sign-out.
        if self.files.iter().flat_map(|f| &f.pages).any(|p| p.name.name == target && p.kind == PageKind::Signin) {
            self.error(file, format!("{target} is the sign-in window, which the program shows itself; nothing opens it"), span);
            return;
        }
        let visible = self.visible_modules(file);
        let kinds: Vec<DeclKind> = self
            .decls
            .get(target)
            .map(|ds| {
                ds.iter()
                    .filter(|d| visible.contains(d.module))
                    .map(|d| d.kind)
                    .collect()
            })
            .unwrap_or_default();
        if kinds.is_empty() {
            self.warning(
                file,
                format!("'{target}' is not a window; handlers come with code behind"),
                span,
            );
        } else if !kinds.contains(&DeclKind::Page) {
            self.error(
                file,
                format!(
                    "'{target}' is {} {}, a click opens a window",
                    article(kinds[0].noun()),
                    kinds[0].noun()
                ),
                span,
            );
        }
    }
}
