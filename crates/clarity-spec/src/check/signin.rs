//! The checker's rules of signing in (docs/APP-CAPABILITIES-PLAN.md step 4):
//! the sign-in window and its sign-in views, guided (window, content, views
//! holding only the sign-in parts and decoration, each part fitting the
//! project's signin block), the sign-in part on the app's splash, and the
//! click targets `signin <strategy>` and `signout`. The block itself is
//! checked against the entity in `crate::signin`.

use super::*;

impl<'a> Checker<'a> {
    /// The guided sign-in (docs/APP-CAPABILITIES-PLAN.md, "The sign-in
    /// window is designed, and guided"): the sign-in window holds its
    /// content and decoration, a sign-in view the sign-in parts and
    /// decoration, and a sign-in part stands on a sign-in view only, fitting
    /// the project's signin block.
    pub(super) fn check_signin_guided(&mut self, file: usize, page: &'a Page) {
        let window = crate::signin::window(self.files);
        let views = window.map(crate::signin::views_of).unwrap_or_default();
        let on_view = page.kind == PageKind::View && views.contains(&page.name.name);
        if page.kind == PageKind::Signin {
            for w in &page.widgets {
                // The sign-in part says where it stands itself.
                if !widgets::on_signin_window(w.props.kind) && w.props.kind != WidgetKind::Signin {
                    let what = widgets::info(w.props.kind).name.to_lowercase();
                    self.error(file, format!("the sign-in window {} holds its content, which shows the sign-in views, and a heading, label, picture, line or link; put the {what} on a sign-in view", page.name.name), w.span);
                }
            }
        }
        let mut all = Vec::new();
        crate::resources::all_widgets(&page.widgets, &mut all);
        for w in &all {
            let info = widgets::info(w.props.kind);
            let what = info.name.to_lowercase();
            if info.category == widgets::Category::Signin && !on_view {
                self.error(file, format!("a {what} belongs on a sign-in view, a view the sign-in window's content shows"), w.span);
            } else if on_view && !info.signin_view {
                self.error(file, format!("a sign-in view holds the sign-in parts and a heading, label, picture, line or link; a {what} has no place on {}", page.name.name), w.span);
            }
        }
        if !on_view {
            return;
        }
        let Some(signin) = self.signin else { return };
        let or_list = |names: &[String]| match names.split_last() {
            Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
            Some((last, _)) => last.clone(),
            None => String::new(),
        };
        let strategies: Vec<String> = signin.strategies.iter().map(|s| s.name.clone()).collect();
        let mut inputs: Vec<String> = Vec::new();
        for s in &signin.strategies {
            for input in crate::signin::strategy_info(&s.name).map(|i| i.inputs).unwrap_or_default() {
                if !inputs.iter().any(|i| i == input) {
                    inputs.push(input.to_string());
                }
            }
        }
        let name_of = |w: &Widget| w.props.name.clone().unwrap_or_else(|| widgets::keyword(w.props.kind).to_string());
        // Each part once on a view: a field per input, a button per strategy, one of the others.
        let mut seen: Vec<(String, String)> = Vec::new();
        for w in &all {
            let p = &w.props;
            let once = match p.kind {
                WidgetKind::SigninField => p.input.as_ref().map(|i| (format!("a field for {i}"), format!("field:{i}"))),
                WidgetKind::SigninButton => p.strategy.as_ref().map(|s| (format!("a button for {s}"), format!("button:{s}"))),
                WidgetKind::SigninOther => p.shows.as_ref().map(|v| (format!("a way to {v}"), format!("other:{v}"))),
                WidgetKind::SigninRemember | WidgetKind::SigninLanguage | WidgetKind::SigninRefusal => {
                    let what = widgets::info(p.kind).name.to_lowercase();
                    Some((format!("a {what}"), what))
                }
                _ => None,
            };
            if let Some((said, key)) = once {
                match seen.iter().find(|(k, _)| *k == key) {
                    Some((_, first)) => self.error(file, format!("{} has {said} already, {first}; a sign-in view has one", page.name.name), w.span),
                    None => seen.push((key, name_of(w))),
                }
            }
            match p.kind {
                WidgetKind::SigninField => match &p.input {
                    None => self.error(file, format!("{} says which input it takes: input {}", name_of(w), or_list(&inputs)), w.span),
                    Some(input) if !inputs.contains(input) => self.error(file, format!("the project's strategies take {}; not {input}", inputs.join(", ")), w.span),
                    Some(_) => {}
                },
                WidgetKind::SigninButton => match &p.strategy {
                    None => self.error(file, format!("{} says which strategy it signs in with: strategy {}", name_of(w), or_list(&strategies)), w.span),
                    Some(strategy) if !strategies.contains(strategy) => self.error(file, format!("the project signs in with {}; not {strategy}", strategies.join(", ")), w.span),
                    Some(strategy) => {
                        for input in crate::signin::strategy_info(strategy).map(|i| i.inputs).unwrap_or_default() {
                            let there = all.iter().any(|f| f.props.kind == WidgetKind::SigninField && f.props.input.as_deref() == Some(*input));
                            if !there {
                                self.error(file, format!("{} signs in with {strategy}, which takes the {input} from a field on {} (input {input}); it has none", name_of(w), page.name.name), w.span);
                            }
                        }
                    }
                },
                WidgetKind::SigninRemember if !signin.remember_asks => {
                    let days = signin.remember_days.unwrap_or(30);
                    self.error(file, format!("keep me signed in asks whether to remember, which needs remember {days} days when asked in the signin block"), w.span);
                }
                WidgetKind::SigninLanguage if !signin.language_picker => {
                    self.error(file, "the language dropdown needs language picker in the signin block", w.span);
                }
                WidgetKind::SigninOther => {
                    let others: Vec<String> = views.iter().filter(|v| **v != page.name.name).cloned().collect();
                    match &p.shows {
                        None => self.error(file, format!("{} says which sign-in view it shows: shows {}", name_of(w), or_list(&others)), w.span),
                        Some(view) if *view == page.name.name => self.error(file, format!("{} shows {view}, the view it stands on", name_of(w)), w.span),
                        Some(view) if !views.contains(view) => {
                            let window = window.map(|p| p.name.name.clone()).unwrap_or_default();
                            self.error(file, format!("{} shows a sign-in view, one the content of {window} shows: {}; not {view}", name_of(w), views.join(", ")), w.span);
                        }
                        Some(_) => {}
                    }
                }
                _ => {}
            }
        }
    }

    /// A sign-in window: one at most, in a project whose file has `signin`,
    /// shown by the program before anyone signs in.
    pub(super) fn check_signin_page(&mut self, file: usize, page: &'a Page) {
        let first = self.files.iter().flat_map(|f| &f.pages).find(|p| p.kind == PageKind::Signin);
        if let Some(first) = first.filter(|f| !std::ptr::eq(*f, page)) {
            self.error(file, format!("there is already a sign-in window, {}; a project has one", first.name.name), page.name.span);
        }
        if self.signin.is_none() {
            self.error(file, format!("{} is a sign-in window, which needs signin in the project file, like: signin Member", page.name.name), page.name.span);
        }
        if page.allow.is_some() {
            self.error(file, format!("{} shows before anyone signs in, so it has no 'allow'", page.name.name), page.name.span);
        }
        if let Some(splash) = crate::signin::app_splash(self.files).filter(|s| crate::frame::page_signs_in(s)) {
            self.error(file, format!("both {} and the splash {} sign in; keep one: the window of the kind signin, or the sign-in parts on the splash", page.name.name, splash.name.name), page.name.span);
        }
    }

    /// A click that signs in (`signin password`) or out (`signout`): the
    /// project file has `signin`; signing in is a button of the sign-in
    /// window, with a strategy the project names and a part per input.
    pub(super) fn check_signin_target(&mut self, file: usize, target: &str, button: bool, new_window: bool, span: Span) {
        if new_window {
            self.error(file, "only a window opens in a new window", span);
        }
        let Some(strategy) = crate::frame::signin_strategy(target) else {
            if self.signin.is_none() {
                self.error(file, "signout signs out who signed in, which needs signin in the project file, like: signin Member", span);
            }
            return;
        };
        let Some(signin) = self.signin else {
            self.error(file, format!("{target} signs in, which needs signin in the project file, like: signin Member"), span);
            return;
        };
        // A hand-built sign-in stands on the app's splash; the sign-in window signs in with its sign-in buttons.
        let splash = crate::signin::app_splash(self.files);
        let page = self.page.filter(|p| splash.is_some_and(|s| std::ptr::eq(s, *p)));
        let Some(page) = page.filter(|_| button) else {
            self.error(file, format!("{target} signs in from a button on the app's splash; the sign-in window signs in with its sign-in buttons"), span);
            return;
        };
        if signin.strategy(strategy).is_none() {
            let names: Vec<&str> = signin.strategies.iter().map(|s| s.name.as_str()).collect();
            self.error(file, format!("the project signs in with {}, not {strategy}", names.join(", ")), span);
            return;
        }
        let Some(info) = crate::signin::strategy_info(strategy) else { return };
        let mut names = Vec::new();
        crate::resources::each_widget(&page.widgets, &mut |w| names.extend(w.props.name.clone()));
        for input in info.inputs {
            if !names.iter().any(|n| n == input) {
                self.error(file, format!("signin {strategy} takes the {input} from a part named {input}; {} has none", page.name.name), span);
            }
        }
    }

    /// A sign-in part: the project file has `signin`; the part is on the
    /// sign-in window or the app's splash, as a sign-in button is; one in the
    /// project. What it offers is the block's (`shows`), checked with it.
    pub(super) fn check_signin_part(&mut self, file: usize, widget: &Widget) {
        if self.signin.is_none() {
            self.error(file, "a signin part signs in, which needs signin in the project file, like: signin Member", widget.span);
            return;
        }
        // On the app's splash; the sign-in window is designed from its sign-in views instead.
        let splash = crate::signin::app_splash(self.files);
        if self.page.filter(|page| splash.is_some_and(|s| std::ptr::eq(s, *page))).is_none() {
            self.error(file, "a signin part stands on the app's splash; the sign-in window is designed from its views", widget.span);
        }
        // The first in the project, in the order of the files and their pages.
        let mut first: Option<(&Page, *const Widget)> = None;
        for page in self.files.iter().flat_map(|f| &f.pages) {
            crate::resources::each_widget(&page.widgets, &mut |w| {
                if first.is_none() && w.props.kind == WidgetKind::Signin {
                    first = Some((page, std::ptr::from_ref(w)));
                }
            });
        }
        if let Some((page, _)) = first.filter(|(_, w)| !std::ptr::eq(*w, widget)) {
            self.error(file, format!("there is already a signin part, on {}; a project has one", page.name.name), widget.span);
        }
    }
}
