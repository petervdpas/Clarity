//! The checker's rules of toolboxes (docs/ledger/canvas-toolbox.md): a
//! canvas's tools, its toolbox page and what it drops.

use super::*;

impl<'a> Checker<'a> {
    /// The parts of a toolbox (docs/ledger/canvas-toolbox.md): they stand in a toolbox page, a group holds tools, a
    /// tool does a built-in action or runs a handler; a toolbox holds its parts, buttons, labels and lines.
    pub(super) fn check_tools(&mut self, file: usize, widgets: &[Widget], toolbox: bool, in_group: bool) {
        for w in widgets {
            let kind = w.props.kind;
            let is_tool_part = matches!(kind, WidgetKind::Tool | WidgetKind::ToolGroup | WidgetKind::ToolSize | WidgetKind::ToolOpacity | WidgetKind::ToolColors | WidgetKind::ToolLayers);
            if is_tool_part && !toolbox {
                self.error(file, format!("a {} belongs in a toolbox, a page written 'page Tools: toolbox'", widgets::info(kind).name.to_lowercase()), w.span);
            } else if toolbox && !is_tool_part && !matches!(kind, WidgetKind::Button | WidgetKind::Label | WidgetKind::Heading | WidgetKind::Separator) {
                self.error(file, "a toolbox holds tools, tool groups, size, opacity and colours, and buttons, labels, headings and lines", w.span);
            }
            if in_group && kind != WidgetKind::Tool {
                self.error(file, "a tool group holds tools", w.span);
            }
            if kind == WidgetKind::Tool {
                let runs = w.props.handlers.iter().any(|h| h.event == crate::widgets::Event::Click);
                match &w.props.does {
                    Some(does) if !crate::model::TOOL_ACTIONS.contains(&does.as_str()) => {
                        self.error(file, format!("a tool does one of {}, not '{does}'", crate::model::TOOL_ACTIONS.join(", ")), w.span);
                    }
                    Some(_) if runs => self.error(file, "a tool does a built-in action or runs a handler, not both", w.span),
                    None if !runs => self.error(file, "a tool does a built-in action (does pencil) or runs a handler (on click: handler)", w.span),
                    _ => {}
                }
            }
            self.check_tools(file, &w.children, toolbox, kind == WidgetKind::ToolGroup);
        }
    }

    /// A canvas's toolbox page: there is one of that name, and its tools are those of the canvas's mode.
    pub(super) fn check_toolbox_of(&mut self, file: usize, widget: &Widget) {
        let Some(name) = widget.props.toolbox_page.as_deref() else { return };
        let Some(page) = self.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == name) else {
            self.error(file, format!("there is no toolbox '{name}'; write one as 'page {name}: toolbox'"), widget.span);
            return;
        };
        if page.kind != PageKind::Toolbox {
            self.error(file, format!("{name} is no toolbox; a toolbox is written 'page {name}: toolbox'"), widget.span);
            return;
        }
        // A memo's toolbox works on its text: coded tools, buttons and labels, no canvas tools.
        if widget.props.kind == WidgetKind::Memo {
            fn canvas_only<'w>(widgets: &'w [Widget], out: &mut Vec<&'w Widget>) {
                for w in widgets {
                    if w.props.does.is_some() || matches!(w.props.kind, WidgetKind::ToolSize | WidgetKind::ToolOpacity | WidgetKind::ToolColors | WidgetKind::ToolLayers) {
                        out.push(w);
                    }
                    canvas_only(&w.children, out);
                }
            }
            let mut found = Vec::new();
            canvas_only(&page.widgets, &mut found);
            if !found.is_empty() {
                self.error(file, format!("{name} works on a canvas's picture; a memo's toolbox holds tools with 'on click', buttons and labels"), widget.span);
            }
            return;
        }
        let Some(mode) = widget.props.mode else { return };
        let tools = mode.tools();
        fn does<'w>(widgets: &'w [Widget], out: &mut Vec<&'w str>) {
            for w in widgets {
                if let Some(d) = w.props.does.as_deref() {
                    out.push(d);
                }
                does(&w.children, out);
            }
        }
        let mut done = Vec::new();
        does(&page.widgets, &mut done);
        // Layers are a picture's.
        fn has_layers(widgets: &[Widget]) -> bool {
            widgets.iter().any(|w| w.props.kind == WidgetKind::ToolLayers || has_layers(&w.children))
        }
        if mode != crate::model::CanvasMode::Paint && has_layers(&page.widgets) {
            self.error(file, format!("{name}'s layers are a picture's; a canvas in mode {} has none", mode.keyword()), widget.span);
        }
        // The canvas tools of the other mode are not this canvas's; the actions done at once are every painting canvas's.
        let all_tools: Vec<&str> = [crate::model::CanvasMode::Objects, crate::model::CanvasMode::Paint, crate::model::CanvasMode::Vector].iter().flat_map(|m| m.tools().iter().copied()).collect();
        for d in done {
            let picture_only = matches!(d, "rotate" | "flipAcross" | "flipDown" | "crop" | "import" | "export" | "adjust");
            if (all_tools.contains(&d) && !tools.contains(&d)) || (picture_only && mode != crate::model::CanvasMode::Paint) {
                self.error(file, format!("{name}'s tool '{d}' is no tool of a canvas in mode {}", mode.keyword()), widget.span);
            }
        }
    }

    /// `on drop from fragmentList`: that part is a list, table or tree of the same window, not the part itself.
    pub(super) fn check_drops(&mut self, file: usize, widgets: &[Widget]) {
        fn all<'w>(widgets: &'w [Widget], out: &mut Vec<&'w Widget>) {
            for w in widgets {
                out.push(w);
                all(&w.children, out);
                for page in &w.pages {
                    all(&page.children, out);
                }
            }
        }
        let mut parts = Vec::new();
        all(widgets, &mut parts);
        for w in &parts {
            for d in &w.props.drops {
                let from = parts.iter().find(|o| o.props.name.as_deref() == Some(d.from.as_str()));
                match from {
                    None => self.error(file, format!("there is no part '{}' in this window to drag rows from", d.from), w.span),
                    Some(o) if !matches!(o.props.kind, WidgetKind::List | WidgetKind::Table | WidgetKind::Tree | WidgetKind::TreeTable) => {
                        self.error(file, format!("'{}' has no rows to drag; rows come from a list, table or tree", d.from), w.span)
                    }
                    Some(o) if o.props.name == w.props.name => self.error(file, "a part's own rows move with `reorder by`, not `on drop from` itself", w.span),
                    Some(_) => {}
                }
            }
        }
    }
}
