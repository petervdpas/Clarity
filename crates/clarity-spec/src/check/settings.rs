//! The checker's rules of the app: one app, its settings, its splash and its
//! main window.

use super::*;

impl<'a> Checker<'a> {
    pub(super) fn check_single_app(&mut self) {
        let files = self.files;
        let mut first: Option<&str> = None;
        for (file, f) in files.iter().enumerate() {
            for app in &f.apps {
                match first {
                    None => first = Some(&app.name.name),
                    Some(name) => self.error(
                        file,
                        format!("a project has one app; '{name}' is already declared"),
                        app.name.span,
                    ),
                }
            }
        }
    }

    /// The settings (manual/data.settings.md): one block, of plain values, and the name
    /// `Settings` theirs.
    pub(super) fn check_settings(&mut self) {
        let files = self.files;
        // Clarity's own settings for AI agents belong to an app whose commands agents call.
        let offers = files.iter().any(|f| f.commands.iter().any(|c| c.agent));
        for (file, f) in files.iter().enumerate() {
            for field in f.entities.iter().filter(|e| e.settings).flat_map(|e| &e.fields) {
                if !offers && matches!(field.name.name.as_str(), "agentReadsWhileClosed" | "agentChangesWhileClosed") && field.ty == FieldType::Bool {
                    self.error(file, format!("{} is a setting of an app whose commands agents call; no command says agent", field.name.name), field.span);
                }
            }
        }
        // A secret is the person's, kept in the system's store: never a field of a record (manual/data.settings.md).
        for (file, f) in files.iter().enumerate() {
            for entity in f.entities.iter().filter(|e| !e.settings) {
                for field in entity.fields.iter().filter(|f| f.ty == FieldType::Secret) {
                    self.error(
                        file,
                        format!("a secret is a setting of the person, kept in the system's protected store; entity {} cannot keep '{}' as one", entity.name.name, field.name.name),
                        field.span,
                    );
                }
            }
        }
        let blocks: Vec<(usize, &Entity)> = files.iter().enumerate().flat_map(|(i, f)| f.entities.iter().filter(|e| e.settings).map(move |e| (i, e))).collect();
        for (file, extra) in blocks.iter().skip(1) {
            self.error(*file, "a project has one 'settings' block", extra.name.span);
        }
        if blocks.is_empty() {
            return;
        }
        for (file, f) in files.iter().enumerate() {
            for entity in f.entities.iter().filter(|e| !e.settings && e.name.name == crate::parser::SETTINGS_ENTITY) {
                self.error(file, "'Settings' is the name of the project's settings; give this entity another name", entity.name.span);
            }
        }
        // Every setting is one the person can reach (docs/ledger/settings-secrets.md): a part
        // binds it, a picker opens for language and theme, or a handler sets it. Commands alone set theirs in code.
        let windows = files.iter().any(|f| !f.pages.is_empty());
        let bound = |name: &str| {
            files.iter().flat_map(|f| &f.pages).any(|p| crate::model::shows_setting(&p.widgets, name))
                || files.iter().flat_map(|f| f.pages.iter().map(|p| &p.frame).chain(f.apps.iter().map(|a| &a.frame))).any(|frame| {
                    frame.sidebars.iter().any(|s| crate::model::shows_setting(&s.widgets, name)) || frame.footer.as_ref().is_some_and(|ft| crate::model::shows_setting(&ft.widgets, name))
                })
        };
        // A handler of a window or the app that sets it, as a menu item per language does.
        let mut set: Vec<String> = Vec::new();
        for handler in files.iter().flat_map(|f| &f.code).flat_map(|c| &c.handlers) {
            crate::code::check::each_stmt(&handler.body, &mut |stmt| {
                if let crate::code::StmtKind::Assign { target, .. } = &stmt.kind
                    && let crate::code::ExprKind::Member { object, member } = &target.kind
                    && matches!(&object.kind, crate::code::ExprKind::Name { name } if name == crate::parser::SETTINGS_DATA)
                {
                    set.push(member.name.clone());
                }
            });
        }
        let picked = |target: &str| crate::frame::any_click_in(files, &|w| w.on_click.as_deref() == Some(target), &|c| c.on_click.as_deref() == Some(target));
        for (file, settings) in blocks.iter().filter(|_| windows) {
            for field in &settings.fields {
                let name = field.name.name.as_str();
                let picker = matches!(field.ty, FieldType::Language | FieldType::Theme) && picked(name);
                if !bound(name) && !picker && !set.iter().any(|s| s == name) {
                    self.warning(
                        *file,
                        format!("no window shows setting '{name}', so the person using the app cannot set it; bind a part to settings.{name}, such as in a settings window (New window, Settings window) opened from a menu"),
                        field.span,
                    );
                }
            }
        }
        for (file, settings) in blocks {
            for field in &settings.fields {
                if field.ty == FieldType::Secret && field.modifiers.default.is_some() {
                    self.error(file, format!("a secret has no default: it is the person's own ('{}')", field.name.name), field.span);
                }
                // A reference keeps the record's id, as a `ref` field does.
                let plain = !matches!(field.ty, FieldType::List { .. } | FieldType::Image | FieldType::Blob | FieldType::Id);
                if !plain {
                    self.error(file, format!("a setting is a plain value or a reference; '{}' cannot be {}", field.name.name, field.type_text), field.span);
                }
            }
        }
    }

    /// The project's settings, when it has them.
    pub(super) fn settings(&self) -> Option<&'a Entity> {
        self.files.iter().flat_map(|f| &f.entities).find(|e| e.settings)
    }

    pub(super) fn check_app(&mut self, file: usize, app: &'a App) {
        match &app.main {
            None => self.warning(
                file,
                format!("app '{}' has no 'main' window yet", app.name.name),
                app.name.span,
            ),
            Some(main) => self.check_main(file, main),
        }
        if let Some(splash) = &app.splash {
            self.check_splash(file, splash);
        }
        let context = self.data_context(file, &app.data);
        self.check_frame(file, &context, &app.frame);
        self.check_names(file, &[], &app.frame);
        let mut keyed = Vec::new();
        frame_keys(file, &app.frame.menu, &app.frame.toolbar, &app.frame.ribbon, &mut keyed);
        self.key_clashes(keyed, "the app's frame");
    }

    /// The main window is a page of kind window.
    /// The app's splash (`splash Welcome`) is a page of the kind splash.
    pub(super) fn check_splash(&mut self, file: usize, splash: &Ident) {
        let found = self.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == splash.name);
        match found {
            None => self.error(file, format!("there is no splash page '{}': write page {}: splash", splash.name, splash.name), splash.span),
            Some(page) if page.kind != PageKind::Splash => self.error(
                file,
                format!("{} is a {}; the app's splash shows a splash page: write page {}: splash", splash.name, page_kind_word(page.kind), splash.name),
                splash.span,
            ),
            Some(_) => {}
        }
    }

    pub(super) fn check_main(&mut self, file: usize, main: &Ident) {
        let visible = self.visible_modules(file);
        let found = self.decls.get(main.name.as_str()).and_then(|ds| {
            ds.iter()
                .find(|d| {
                    visible.contains(d.module)
                        && d.kind == DeclKind::Page
                })
                .map(|d| (d.kind, d.file, d.index))
        });
        match found {
            None => self.error(
                file,
                format!("'{}' is not a window", main.name),
                main.span,
            ),
            Some((DeclKind::Page, f, i)) if self.files[f].pages[i].kind != PageKind::Window => self
                .error(
                    file,
                    format!(
                        "the main window '{}' must be a page of kind window",
                        main.name
                    ),
                    main.span,
                ),
            Some(_) => {}
        }
    }
}
