//! The checker's rules of a window's parts: each kind's options, ranges,
//! dropdowns, charts, graphs, trees, canvas modes, reordering and edits.

use super::*;

impl<'a> Checker<'a> {
    pub(super) fn check_widgets(
        &mut self,
        file: usize,
        page: &PageContext<'a>,
        widgets: &[Widget],
        columns: u32,
    ) {
        for (index, widget) in widgets.iter().enumerate() {
            let p = &widget.props;
            let name = widgets::keyword(p.kind);
            if let Some(icon) = p.icon.as_deref().filter(|i| crate::icons::path(i).is_none()) {
                self.error(file, crate::icons::unknown(icon), widget.span);
            }
            self.check_small_part(file, widget);
            self.check_tree_levels(file, widget);
            // A popover is outside the grid: only its own parts are placed, on its own grid.
            // A table's Add button and trash buttons run their handlers, which add and delete the row.
            if p.add && !p.handlers.iter().any(|h| h.event == crate::widgets::Event::Add) {
                self.error(file, "'add' shows an Add button above the table: write 'on add: handler' that adds the row", widget.span);
            }
            self.check_toolbox_of(file, widget);
            // A toolbox holds the tools of a canvas with a mode (a diagram's, a painting's, a drawing's), or a memo's coded ones.
            if p.toolbox.is_some() && p.kind != WidgetKind::Memo && p.mode.is_none() {
                self.error(file, "a toolbox holds the tools of a canvas in mode objects, paint or vector", widget.span);
            }
            // A link column is one of the table's columns.
            for link in &p.links {
                if !p.columns.contains(&link.column) {
                    self.error(file, format!("'{}' is no column of this table; a link is a column it shows", link.column), widget.span);
                }
            }
            if p.remove && !p.handlers.iter().any(|h| h.event == crate::widgets::Event::Remove) {
                self.error(file, "'remove' shows a trash button on every row: write 'on remove: handler' that deletes the row", widget.span);
            }
            let popover = p.kind == WidgetKind::Popover;
            if popover && p.for_part.is_none() {
                self.error(file, "a popover opens next to a part: write 'for' and the part's name", widget.span);
            }
            let inside = widget.children.iter().chain(widget.pages.iter().flat_map(|page| &page.children));
            for nested in inside {
                match nested.props.kind {
                    WidgetKind::Popover => self.error(file, "a popover stands on the window, not in another part", nested.span),
                    WidgetKind::Slideout if nested.props.beside.is_none() => {
                        self.error(file, "a slide-out stands on the window, or beside a part of the container it is in: write 'beside' and the part", nested.span)
                    }
                    WidgetKind::Sound => self.error(file, "a sound stands on the window, not in another part", nested.span),
                    WidgetKind::Timer => self.error(file, "a timer stands on the window, not in another part", nested.span),
                    _ => {}
                }
            }
            // A sound and a timer have no place at all (manual/design.sound.md, manual/parts.time.md).
            let popover = popover || widgets::off_grid(p.kind);
            self.check_time(file, widget);
            if let Some(menu) = &widget.menu {
                // A context menu is named only when code hides or disables it as a whole.
                if let Some(menu_name) = menu.name.as_deref().filter(|n| !crate::names::is_camel_case(n)) {
                    self.error(file, crate::names::camel_case_problem("context menu", menu_name), menu.span);
                }
                if menu.items.is_empty() {
                    self.warning(file, "the context menu has no items yet", menu.span);
                }
                self.check_commands(file, &menu.items);
            }
            if !popover && (p.col == 0 || p.row == 0 || p.width == 0 || p.height == 0) {
                self.error(
                    file,
                    format!(
                        "{name} at {},{}: position and size start at 1",
                        p.col, p.row
                    ),
                    widget.span,
                );
            } else if !popover && p.col + p.width - 1 > columns {
                self.error(
                    file,
                    format!(
                        "{name} at {},{} span {} runs past column {columns}",
                        p.col, p.row, p.width
                    ),
                    widget.span,
                );
            }
            for other in widgets[..index].iter().filter(|o| !popover && !widgets::off_grid(o.props.kind)) {
                if overlaps(&other.props, p) {
                    self.error(
                        file,
                        format!(
                            "{name} at {},{} overlaps the {} at {},{}",
                            p.col,
                            p.row,
                            widgets::keyword(other.props.kind),
                            other.props.col,
                            other.props.row
                        ),
                        widget.span,
                    );
                }
            }
            // Every part has a name, which code and its events go by (as in an IDE).
            if let Some(part) = p.name.as_deref().filter(|n| !crate::names::is_camel_case(n)) {
                self.error(file, crate::names::camel_case_problem("part", part), widget.span);
            }
            if p.name.is_none() {
                self.error(
                    file,
                    format!("{name} at {},{} has no name; every part has one (Name them, in Problems, names them all)", p.col, p.row),
                    widget.span,
                );
            }
            self.check_binding(file, page, widget);
            if p.kind == WidgetKind::Signin {
                self.check_signin_part(file, widget);
            }
            if let Some(target) = &p.on_click {
                if p.new_window && frame::BUILT_IN_COMMANDS.contains(&target.as_str()) {
                    self.error(file, "only a window opens in a new window", widget.span);
                }
                if frame::is_signin_target(target) {
                    self.check_signin_target(file, target, p.kind == WidgetKind::Button, p.new_window, widget.span);
                } else if frame::BUTTON_COMMANDS.contains(&target.as_str()) {
                    // A button closes its window or quits like a menu item.
                } else if frame::BUILT_IN_COMMANDS.contains(&target.as_str()) {
                    self.error(file, format!("'{target}' picks in a menu or a toolbar; a button can close its window or quit"), widget.span);
                } else if page.popovers.contains(target.as_str()) {
                    // A click opens a popover of its window next to its part, or a slide-out at its side.
                } else {
                    self.check_click_target(file, target, &p.open_with, Some(page), widget.span);
                }
            }

            match widgets::container(p.kind) {
                None => {}
                // A tool group's tools lie in its flyout, not on a grid.
                Some(ContainerKind::Plain) if p.kind == WidgetKind::ToolGroup => {}
                Some(ContainerKind::Plain) => {
                    self.check_widgets(file, page, &widget.children, p.width)
                }
                Some(container) => {
                    // The dropdown of closed tabs and a permanent tab only mean something among tabs the person closes.
                    if container == ContainerKind::Tabs && p.available && !p.closing {
                        self.error(file, "'available' lists the closed tabs: write 'closing'", widget.span);
                    }
                    if container == ContainerKind::Tabs && !p.closing {
                        for tab in widget.pages.iter().filter(|t| t.permanent) {
                            let name = p.name.as_deref().unwrap_or("the tabs");
                            self.error(file, format!("a permanent tab stays among tabs that close: write 'closing' on {name}"), tab.span);
                        }
                    }
                    match container {
                        ContainerKind::Tabs if widget.pages.is_empty() => {
                            self.error(file, "tabs need at least one 'tab'", widget.span);
                        }
                        ContainerKind::Content if widget.pages.is_empty() => {
                            self.error(file, "content needs at least one 'view', as in view OrderEditor", widget.span);
                        }
                        ContainerKind::Accordion if widget.pages.is_empty() => {
                            self.error(file, "an accordion needs at least one 'section'", widget.span);
                        }
                        ContainerKind::Steps if widget.pages.len() < 2 => {
                            self.error(file, "steps need at least two 'step' blocks: a wizard of one step is a panel", widget.span);
                        }
                        ContainerKind::Splitter if widget.pages.len() != 2 => {
                            self.error(
                                file,
                                "a splitter has exactly two 'pane' blocks",
                                widget.span,
                            );
                        }
                        _ => {}
                    }
                    if container == ContainerKind::Splitter {
                        let side_by_side = p.orientation != Some(crate::model::Orientation::Vertical);
                        for (i, pane) in widget.pages.iter().enumerate() {
                            // Its start along the split: columns side by side, rows one above the other.
                            let (length, unit) = if side_by_side { (p.width, "columns") } else { (p.height, "rows") };
                            match pane.columns {
                                None => {}
                                Some(_) if i > 0 => self.error(file, "only the first pane has a size; the second has the rest", pane.span),
                                Some(n) if !(1..length).contains(&n) => {
                                    self.error(file, format!("the first pane spans 1 to {} of the splitter's {length} {unit}", length.saturating_sub(1)), pane.span)
                                }
                                Some(_) => {}
                            }
                        }
                    }
                    let columns = if container == ContainerKind::Splitter { widgets::pane_columns(widget) } else { widget.pages.iter().map(|_| p.width).collect() };
                    for (inner, columns) in widget.pages.iter().zip(columns) {
                        self.check_widgets(file, page, &inner.children, columns);
                    }
                }
            }

            let draws = p.handlers.iter().any(|h| h.event == widgets::Event::Draw);
            if let Some(engine) = &p.runs {
                let canvas = p.name.clone().unwrap_or_else(|| format!("at {},{}", p.col, p.row));
                match self.files.iter().flat_map(|f| &f.engines).find(|e| &e.name.name == engine) {
                    None => self.error(file, format!("canvas {canvas} runs {engine}, which is no engine"), widget.span),
                    Some(e) if !e.beside => self.error(
                        file,
                        format!("canvas {canvas} runs {engine}, which runs on the backend; a canvas runs an engine declared 'beside the screen'"),
                        widget.span,
                    ),
                    Some(_) if draws => self.error(file, format!("canvas {canvas} is drawn by engine {engine}, so it has no draw handler"), widget.span),
                    Some(e) => {
                        for tell in p.tells.iter().filter(|t| !e.tells.iter().any(|own| own.name.name == t.tell)) {
                            self.error(file, format!("engine {engine} tells no '{}'", tell.tell), widget.span);
                        }
                    }
                }
            } else if p.kind == WidgetKind::Canvas && !p.tells.is_empty() {
                let canvas = p.name.clone().unwrap_or_else(|| format!("at {},{}", p.col, p.row));
                self.error(file, format!("canvas {canvas} runs no engine, so it tells nothing"), widget.span);
            // A painted or vector canvas shows what it binds (manual/parts.graphics.md).
            } else if p.kind == WidgetKind::Canvas && !draws && !matches!(p.mode, Some(crate::model::CanvasMode::Paint | crate::model::CanvasMode::Vector)) {
                self.warning(
                    file,
                    format!("canvas at {},{} draws nothing yet; wire 'on draw: handler'", p.col, p.row),
                    widget.span,
                );
            }
            if let Err(problem) = widgets::canvas_options(p) {
                self.error(file, problem, widget.span);
            }
            if p.kind == WidgetKind::Image && p.source.is_none() && p.bind.is_none() && p.caption.is_none() {
                self.warning(
                    file,
                    format!("image at {},{} has no source yet", p.col, p.row),
                    widget.span,
                );
            }
        }
    }

    /// The rules of a separator, a link and a progress bar (U1b).
    pub(super) fn check_small_part(&mut self, file: usize, widget: &Widget) {
        let p = &widget.props;
        match p.kind {
            WidgetKind::Form if p.labels.is_some() => {
                let columns = p.labels.unwrap_or_default();
                if columns == 0 || columns >= p.width {
                    self.error(
                        file,
                        format!("a form's label column leaves room for its inputs: 'labels' is 1 to {} here", p.width.saturating_sub(1)),
                        widget.span,
                    );
                }
                let labelled: Vec<&str> = widget.children.iter().filter(|w| w.props.made).filter_map(|w| w.props.for_part.as_deref()).collect();
                for inner in widget.children.iter().filter(|w| !w.props.made) {
                    let q = &inner.props;
                    if let Some(name) = q.name.as_deref().filter(|n| labelled.contains(n))
                        && q.col <= columns
                    {
                        self.error(file, format!("'{name}' starts at column {} or later: its label takes the columns before", columns + 1), inner.span);
                    }
                    if q.kind == WidgetKind::Label
                        && let Some(target) = q.for_part.as_deref().filter(|t| labelled.contains(t))
                    {
                        self.error(file, format!("the form labels '{target}' itself; remove this label"), inner.span);
                    }
                }
            }
            WidgetKind::Buttons => {
                for inner in &widget.children {
                    if !matches!(inner.props.kind, WidgetKind::Button | WidgetKind::Toggle) {
                        self.error(file, "a button group holds buttons and toggles", inner.span);
                    } else if inner.props.row != 1 || inner.props.height != 1 {
                        self.error(file, "the buttons of a group stand in one row: at row 1, one row high", inner.span);
                    }
                }
            }
            WidgetKind::Button if p.split => {
                if widget.menu.is_none() {
                    self.error(file, "a split button opens its menu: write a 'menu' block under it", widget.span);
                }
                let clicks = p.on_click.is_some() || p.handlers.iter().any(|h| h.event == widgets::Event::Click);
                if !clicks {
                    self.error(file, "a split button runs its click too: wire 'on click'", widget.span);
                }
            }
            WidgetKind::Separator if p.caption.is_some() => {
                self.error(file, "a separator has no caption: it is only a line", widget.span);
            }
            WidgetKind::Link => {
                let clicks = p.on_click.is_some() || p.handlers.iter().any(|h| h.event == widgets::Event::Click);
                let ways = [p.to.is_some(), p.bind.is_some(), clicks].iter().filter(|w| **w).count();
                if ways == 0 {
                    self.error(file, "a link opens an address ('to'), what it binds or runs a handler ('on click')", widget.span);
                } else if ways > 1 {
                    self.error(file, "a link opens an address, what it binds or runs a handler: one of them", widget.span);
                }
                if let Some(to) = p.to.as_deref().filter(|t| !is_address(t)) {
                    self.error(file, format!("'{to}' is no address; write it whole, as in \"https://{to}\""), widget.span);
                }
            }
            WidgetKind::Progress => {
                if p.bind.is_none() {
                    self.error(file, "a progress bar shows a number: bind one, as in 'bind done'", widget.span);
                }
                if p.max.as_deref().is_some_and(|m| m.parse::<f64>().map_or(true, |n| n <= 0.0)) {
                    self.error(file, "'max' is a number above 0: the number the bar is full at", widget.span);
                }
            }
            // A status item's gauge, when it binds a number, is full at its max.
            WidgetKind::StatusItem => {
                if p.max.as_deref().is_some_and(|m| m.parse::<f64>().map_or(true, |n| n <= 0.0)) {
                    self.error(file, "'max' is a number above 0: the number its gauge is full at", widget.span);
                }
            }
            _ => {}
        }
    }

    /// The rules of a slider's or a spin box's range (U1c): its lowest value
    /// below its highest, a step above 0, whole for a field of whole numbers.
    /// `field` is the bound field's name, type and modifiers, when it binds one.
    pub(super) fn check_range(&mut self, file: usize, widget: &Widget, field: Option<(&str, &FieldType, &crate::model::Modifiers)>) {
        let p = &widget.props;
        if !matches!(p.kind, WidgetKind::Slider | WidgetKind::Spinbox) {
            return;
        }
        let range = widgets::number_range(p, field.map(|(_, _, m)| m));
        let number = |n: &Option<String>| n.as_deref().and_then(|n| n.parse::<f64>().ok());
        if let (Some(min), Some(max)) = (number(&range.min), number(&range.max))
            && min >= max
        {
            let (low, high) = (range.min.as_deref().unwrap_or_default(), range.max.as_deref().unwrap_or_default());
            self.error(file, format!("'min' is below 'max': {low} is not below {high}"), widget.span);
        }
        let step = range.step.parse::<f64>().unwrap_or(0.0);
        if step <= 0.0 {
            self.error(file, "'step' is a number above 0", widget.span);
        } else if let Some((name, FieldType::Integer, _)) = field
            && step.fract() != 0.0
        {
            self.error(file, format!("'{name}' holds whole numbers: the step is a whole number"), widget.span);
        }
    }

    /// A dropdown's options (U1c): `typed` takes any text, so it binds a
    /// text field; `search` finds a record, so it binds a reference, and
    /// shows fields of that record's entity.
    pub(super) fn check_dropdown(&mut self, file: usize, at: &str, widget: &Widget, ty: &FieldType) {
        let p = &widget.props;
        if let Err(problem) = widgets::dropdown_options(p) {
            self.error(file, format!("{at}: {problem}"), widget.span);
        }
        let text = matches!(ty, FieldType::Text { .. });
        if p.typed && !text {
            self.error(file, format!("{at}: 'typed' takes any text, so it binds a text field"), widget.span);
        } else if text && !p.typed {
            self.error(file, format!("{at}: a dropdown of text is 'typed': any text may be typed, the values it has are offered"), widget.span);
        }
        if p.search && !matches!(ty, FieldType::Ref { .. }) {
            self.error(file, format!("{at}: 'search' finds a record, so it binds a reference"), widget.span);
            return;
        }
        let FieldType::Ref { entity } = ty else { return };
        let Some(target) = self.files.iter().flat_map(|f| &f.entities).find(|e| e.name.name == entity.name) else { return };
        for shown in &p.show {
            match target.field(shown) {
                None => self.error(file, format!("{at}: {} has no field '{shown}' to show", target.name.name), widget.span),
                Some(f) if matches!(f.ty, FieldType::Image | FieldType::Blob | FieldType::List { .. }) => {
                    self.error(file, format!("{at}: a search shows text and values, not {shown}"), widget.span)
                }
                Some(_) => {}
            }
        }
    }

    /// A chart groups its rows by a field and gives each group a number:
    /// its rows counted, or a number field summed or averaged.
    /// A chart of a list of numbers (manual/parts.data.md): drawn as they are, as
    /// bars or a line, never grouped.
    pub(super) fn check_number_chart(&mut self, file: usize, at: &str, widget: &Widget, ty: &FieldType) {
        let p = &widget.props;
        if !matches!(ty, FieldType::Values { item: crate::model::ListItem::Integer | crate::model::ListItem::Decimal }) {
            self.error(file, format!("{at}: a chart of a list of values draws numbers: list<integer> or list<decimal>"), widget.span);
        }
        if p.by.is_some() {
            self.error(file, format!("{at}: a chart of numbers draws them as they are; 'by' groups the rows of a list of records"), widget.span);
        }
        if p.chart == Some(crate::model::ChartKind::Pie) {
            self.error(file, format!("{at}: a pie shares out groups; a list of numbers is drawn as a bar or a line"), widget.span);
        }
    }

    /// What keeps time (manual/parts.time.md): a timer's ticks and a trend's values
    /// come every 0.1 to 86400 seconds, and a trend or chart shows 2 to 1000 points.
    pub(super) fn check_time(&mut self, file: usize, widget: &Widget) {
        let p = &widget.props;
        if p.kind == WidgetKind::Timer && p.every.is_none() {
            let name = p.name.as_deref().unwrap_or("timer");
            self.error(file, format!("timer {name} runs every so many seconds: write every 1, or every 0.5"), widget.span);
        }
        if let Some(every) = &p.every
            && !every.parse::<f64>().is_ok_and(|s| (0.1..=86400.0).contains(&s))
        {
            let what = if p.kind == WidgetKind::Timer { "a timer runs" } else { "a trend takes a value" };
            self.error(file, format!("{what} every 0.1 to 86400 seconds, not {every}"), widget.span);
        }
        if p.points.is_some_and(|n| !(2..=1000).contains(&n)) {
            self.error(file, "a trend or chart shows 2 to 1000 points", widget.span);
        }
    }

    pub(super) fn check_chart(&mut self, file: usize, at: &str, entity: &Entity, widget: &Widget) {
        let p = &widget.props;
        let name = &entity.name.name;
        match p.by.as_deref() {
            None => self.error(file, format!("{at}: a chart groups its rows by a field, as in 'by status'"), widget.span),
            Some(by) => match entity.field(by).map(|f| &f.ty) {
                None => self.error(file, format!("{at}: {name} has no field '{by}'"), widget.span),
                Some(FieldType::Image | FieldType::Blob | FieldType::List { .. } | FieldType::Values { .. }) => {
                    self.error(file, format!("{at}: a chart cannot group by '{by}'"), widget.span)
                }
                Some(_) => {}
            },
        }
        let field = match &p.measure {
            None => return self.error(file, format!("{at}: a chart counts its rows or sums a number, as in 'count' or 'sum total'"), widget.span),
            Some(ChartMeasure::Count) => return,
            Some(ChartMeasure::Sum { field } | ChartMeasure::Average { field }) => field,
        };
        match entity.field(field) {
            None => self.error(file, format!("{at}: {name} has no field '{field}'"), widget.span),
            Some(f) if !matches!(f.ty, FieldType::Integer | FieldType::Decimal { .. } | FieldType::Money | FieldType::Duration) => {
                self.error(file, format!("{at}: '{field}' is {}; a chart sums or averages a number", widgets::field_type_tag(&f.ty)), widget.span)
            }
            Some(_) => {}
        }
    }

    /// What every graph is given whatever its nodes come from (`hops`), and
    /// a graph of the data model (`lists`): lists of records, instead of a
    /// list it binds and its edges; the picked node's entity and id go into
    /// text and whole-number data.
    pub(super) fn check_graph_options(&mut self, file: usize, page: &PageContext<'a>, widget: &Widget, at: &str) {
        let p = &widget.props;
        if let Some(hops) = p.hops.filter(|h| !(1..=widgets::GRAPH_MAX_HOPS).contains(h)) {
            self.error(file, format!("{at}: a graph shows 1 to {} hops from the picked node, not {hops}", widgets::GRAPH_MAX_HOPS), widget.span);
        }
        if p.lists.is_empty() {
            if p.picked_kind.is_some() || p.picked_id.is_some() {
                self.error(file, format!("{at}: pickedKind and pickedId belong to a graph of lists; a graph it binds has its picked record in selected"), widget.span);
            }
            if p.bind.is_none() {
                self.error(file, format!("{at}: a graph shows the records of the list it binds, as in bind=\"things\", or of the lists it names, as in lists=\"people, places\""), widget.span);
            }
            return;
        }
        if p.bind.is_some() || p.edges.is_some() || p.node_key.is_some() || p.node_label.is_some() || p.node_kind.is_some() || p.edge_from.is_some() || p.edge_to.is_some() || p.edge_label.is_some() || p.edge_kind.is_some() {
            self.error(file, format!("{at}: a graph of lists takes its nodes, labels, kinds and edges from the data model; it binds nothing and names no edges or node fields"), widget.span);
        }
        let mut entities: Vec<&Entity> = Vec::new();
        for list in &p.lists {
            match page.data.get(list.as_str()) {
                Some(Held::List(Some(entity))) => entities.push(*entity),
                Some(_) => self.error(file, format!("{at}: '{list}' is no list of records; a graph of lists shows records, as in data people: list<Person>"), widget.span),
                None => self.error(file, format!("{at}: unknown data '{list}'"), widget.span),
            }
        }
        for field in &p.show {
            if field != "id" && !entities.iter().any(|e| e.field(field).is_some()) {
                self.error(file, format!("{at}: none of {} has a field '{field}'", entities.iter().map(|e| e.name.name.as_str()).collect::<Vec<_>>().join(", ")), widget.span);
            }
        }
        for (data, wanted) in [(&p.picked_kind, "text"), (&p.picked_id, "integer")] {
            let Some(data) = data else { continue };
            let fits = match page.data.get(data.as_str()) {
                Some(Held::Value(FieldType::Text { .. })) => wanted == "text",
                Some(Held::Value(FieldType::Integer | FieldType::Id)) => wanted == "integer",
                None => {
                    self.error(file, format!("{at}: unknown data '{data}'"), widget.span);
                    continue;
                }
                _ => false,
            };
            if !fits {
                self.error(file, format!("{at}: '{data}' is to hold the picked node's {}; declare it as {wanted}", if wanted == "text" { "entity" } else { "id" }), widget.span);
            }
        }
    }

    /// A graph's nodes are the records it binds, its edges the records of
    /// `edges`; every field it names is a field of those records, and its
    /// key and its edges' ends are the key or a field of their own.
    pub(super) fn check_graph(&mut self, file: usize, page: &PageContext<'a>, at: &str, nodes: &Entity, widget: &Widget) {
        let p = &widget.props;
        let has = |entity: &Entity, field: &str| field == "id" || field == entity.key_column() || entity.field(field).is_some();
        let key = p.node_key.as_deref().unwrap_or("id");
        for field in [Some(key), p.node_label.as_deref(), p.node_kind.as_deref()].into_iter().flatten().chain(p.show.iter().map(String::as_str)) {
            if !has(nodes, field) {
                self.error(file, format!("{at}: {} has no field '{field}'", nodes.name.name), widget.span);
            }
        }
        if let Some(limit) = p.limit.filter(|l| !(1..=widgets::GRAPH_MAX_LIMIT).contains(l)) {
            self.error(file, format!("{at}: a graph shows 1 to {} nodes, not {limit}", widgets::GRAPH_MAX_LIMIT), widget.span);
        }
        let Some(list) = p.edges.as_deref() else {
            if p.edge_from.is_some() || p.edge_to.is_some() || p.edge_label.is_some() || p.edge_kind.is_some() || !p.edge_looks.is_empty() {
                self.error(file, format!("{at}: name the list of its edges first, as in edges=\"links\""), widget.span);
            }
            return;
        };
        let edges = match page.data.get(list) {
            Some(Held::List(Some(entity))) => *entity,
            Some(_) => return self.error(file, format!("{at}: '{list}' is no list of records; a graph's edges are records, as in data links: list<Link>"), widget.span),
            None => return self.error(file, format!("{at}: unknown data '{list}'"), widget.span),
        };
        let ends = [p.edge_from.as_deref().unwrap_or("source"), p.edge_to.as_deref().unwrap_or("target")];
        for field in ends.into_iter().chain([p.edge_label.as_deref(), p.edge_kind.as_deref()].into_iter().flatten()) {
            if !has(edges, field) {
                self.error(file, format!("{at}: {} has no field '{field}'; an edge names the nodes it joins in its fields (edgeFrom, edgeTo)", edges.name.name), widget.span);
            }
        }
    }

    /// A tree's rows are arranged by `children`, a list field of their own
    /// kind, through the one reference to their own kind (their parent).
    pub(super) fn check_tree(&mut self, file: usize, at: &str, entity: &Entity, widget: &Widget) {
        let name = &entity.name.name;
        let p = &widget.props;
        if p.row_key.is_some() || p.parent_key.is_some() {
            return self.check_tree_by_key(file, at, entity, widget);
        }
        let Some(children) = p.children.as_deref() else {
            return self.error(
                file,
                format!("{at}: a tree names the list of its own kind its rows have, as in 'children children', or the key and parent fields of its rows, as in 'key id parent parentId'"),
                widget.span,
            );
        };
        match entity.field(children).map(|f| &f.ty) {
            None => return self.error(file, format!("{at}: {name} has no field '{children}'"), widget.span),
            Some(FieldType::List { item }) if item.name == *name => {}
            Some(_) => return self.error(file, format!("{at}: '{children}' is no list of {name} records"), widget.span),
        }
        let parents = entity.fields.iter().filter(|f| matches!(&f.ty, FieldType::Ref { entity: e } if e.name == *name)).count();
        if parents != 1 {
            self.error(file, format!("{at}: a tree of {name} needs one field 'ref {name}' to know a row's parent"), widget.span);
        }
    }

    /// A tree by key: each row is named by its `key` field and shows under
    /// the row whose key its `parent` field holds, text or whole numbers.
    pub(super) fn check_tree_by_key(&mut self, file: usize, at: &str, entity: &Entity, widget: &Widget) {
        let p = &widget.props;
        let name = &entity.name.name;
        if p.children.is_some() {
            return self.error(file, format!("{at}: a tree is arranged by 'children' or by 'key' and 'parent', not both"), widget.span);
        }
        let (Some(key), Some(parent)) = (p.row_key.as_deref(), p.parent_key.as_deref()) else {
            return self.error(file, format!("{at}: a tree by key names both fields, as in 'key id parent parentId'"), widget.span);
        };
        let mut kinds = Vec::new();
        for field in [key, parent] {
            // A stored record's own id names it too.
            if field == crate::code::check::RECORD_ID && !entity.record && entity.field(field).is_none() {
                kinds.push("integer");
                continue;
            }
            match entity.field(field).map(|f| &f.ty) {
                None => self.error(file, format!("{at}: {name} has no field '{field}'"), widget.span),
                Some(FieldType::Text { .. }) => kinds.push("text"),
                Some(FieldType::Integer) => kinds.push("integer"),
                Some(other) => self.error(file, format!("{at}: '{field}' is {}; a tree's key and parent are text or integer", widgets::field_type_tag(other)), widget.span),
            }
        }
        if let [k, q] = kinds[..]
            && k != q
        {
            self.error(file, format!("{at}: '{key}' is {k} and '{parent}' is {q}; a row's parent holds a key of the same type"), widget.span);
        }
    }

    /// A tree of several kinds of records (manual/design.windows.md): named, bound to
    /// nothing (its rows are its levels'), each level an entity whose
    /// `under` field refers to the level around it, showing a field or a key,
    /// ordered by a field; each `on select` an entity of its levels.
    pub(super) fn check_tree_levels(&mut self, file: usize, widget: &Widget) {
        if widget.levels.is_empty() {
            if !widget.selects.is_empty() {
                self.error(file, "'on select Entity: handler' belongs to a tree of levels; write level lines in it", widget.span);
            }
            return;
        }
        if widget.props.name.is_none() {
            self.error(file, "a tree of levels needs a name, which code reloads it by", widget.span);
        }
        if widget.props.bind.is_some() {
            self.error(file, "a tree of levels shows its levels' records; it binds nothing", widget.span);
        }
        let entity = |name: &str| self.files.iter().flat_map(|f| &f.entities).find(|e| e.name.name == name);
        let mut problems: Vec<(String, crate::span::Span)> = Vec::new();
        fn walk<'e>(levels: &[crate::model::TreeLevel], around: Option<&str>, entity: &dyn Fn(&str) -> Option<&'e crate::model::Entity>, problems: &mut Vec<(String, crate::span::Span)>) {
            for level in levels {
                let Some(name) = &level.entity else {
                    walk(&level.levels, around, entity, problems);
                    continue;
                };
                let Some(found) = entity(name) else {
                    problems.push((format!("there is no entity '{name}' for this level"), level.span));
                    continue;
                };
                match (&level.under, around) {
                    (Some(under), Some(around)) => match found.field(under).map(|f| &f.ty) {
                        Some(FieldType::Ref { entity }) if entity.name == around => {}
                        Some(_) => problems.push((format!("{name}.{under} does not refer to {around}, the level around it"), level.span)),
                        None => problems.push((format!("{name} has no field '{under}'"), level.span)),
                    },
                    (Some(_), None) => problems.push(("a top level stands under nothing; leave 'under' out".to_string(), level.span)),
                    (None, Some(around)) => problems.push((format!("say which field of {name} refers to {around}: under field"), level.span)),
                    (None, None) => {}
                }
                if let Some(show) = level.show.as_deref().filter(|s| !s.starts_with('@'))
                    && found.field(show).is_none()
                    && show != "id"
                {
                    problems.push((format!("{name} has no field '{show}' to show"), level.span));
                }
                if let Some(order) = &level.order
                    && found.field(order).is_none()
                {
                    problems.push((format!("{name} has no field '{order}' to order by"), level.span));
                }
                walk(&level.levels, Some(name), entity, problems);
            }
        }
        walk(&widget.levels, None, &entity, &mut problems);
        let shown = crate::trees::entities(&widget.levels);
        for select in &widget.selects {
            if !shown.contains(&select.entity) {
                problems.push((format!("the tree shows no {} rows; on select names an entity of its levels", select.entity), select.span));
            }
        }
        for (message, span) in problems {
            self.error(file, message, span);
        }
    }

    /// A canvas's mode (manual/parts.graphics.md): the records of a list placed by integer
    /// fields, a painted picture in an image field, or a vector drawing in a
    /// text field; its tools, brush and ink belong to its mode, and what the
    /// person changes is lost without `on change`.
    pub(super) fn check_canvas_mode(&mut self, file: usize, page: &PageContext<'a>, widget: &Widget, at: &str) {
        use crate::model::CanvasMode;
        let p = &widget.props;
        let changes = p.handlers.iter().any(|h| h.event == widgets::Event::Change);
        let Some(mode) = p.mode else {
            if p.bind.is_some() {
                self.error(file, format!("{at}: a canvas binds a picture in mode paint or a drawing in mode vector; give it its mode"), widget.span);
            }
            for (given, word) in [(!p.tools.is_empty(), "tools"), (p.brush.is_some(), "brush"), (p.ink.is_some(), "ink"), (changes, "on change")] {
                if given {
                    self.error(file, format!("{at}: '{word}' belongs to a canvas with a mode (objects, paint or vector)"), widget.span);
                }
            }
            return;
        };
        if !changes {
            self.warning(file, format!("{at} is changed by the person in mode {} but has no 'on change' handler to keep it", mode.keyword()), widget.span);
        }
        for tool in &p.tools {
            if !mode.tools().contains(&tool.as_str()) {
                let tools = mode.tools();
                let offered = if tools.is_empty() { "no tools".to_string() } else { tools.join(", ") };
                self.error(file, format!("{at}: '{tool}' is no tool of mode {}, which has {offered}", mode.keyword()), widget.span);
            }
        }
        if mode == CanvasMode::Objects && (p.brush.is_some() || p.ink.is_some()) {
            self.error(file, format!("{at}: a brush and ink are for mode paint and mode vector"), widget.span);
        }
        match mode {
            CanvasMode::Objects => {
                if p.bind.is_some() {
                    self.error(file, format!("{at}: a canvas of objects binds nothing; its objects are the records of its list"), widget.span);
                }
                let Some(objects) = &p.objects else { return };
                let entity = match page.data.get(objects.list.as_str()) {
                    Some(Held::List(Some(entity))) => *entity,
                    Some(_) => return self.error(file, format!("{at}: '{}' is no list of records; objects are records, as in data types: list<ClassType>", objects.list), widget.span),
                    None => return self.error(file, format!("{at}: unknown data '{}'", objects.list), widget.span),
                };
                for field in [&objects.x, &objects.y] {
                    match entity.field(field) {
                        None => self.error(file, format!("{at}: {} has no field '{field}'", entity.name.name), widget.span),
                        Some(f) if f.ty != FieldType::Integer => self.error(
                            file,
                            format!("{at}: '{field}' is {}; an object's place is an integer field", widgets::field_type_tag(&f.ty)),
                            widget.span,
                        ),
                        Some(_) => {}
                    }
                }
                // The canvas follows a dragged object by its id: a stored entity has one, a record gives one.
                if entity.record && entity.field("id").is_none_or(|f| f.ty != FieldType::Integer) {
                    self.error(
                        file,
                        format!("{at}: {} is a record, which has no 'id' of its own; give it `id: integer`, different for each object, which the canvas follows a dragged object by", entity.name.name),
                        widget.span,
                    );
                }
            }
            CanvasMode::Paint | CanvasMode::Vector => {
                let what = if mode == CanvasMode::Paint { "an image field, which holds the picture" } else { "a text field, which holds the drawing as SVG" };
                let bound = p.bind.as_deref().and_then(|b| b.split_once('.')).and_then(|(data, field)| match page.data.get(data) {
                    Some(Held::Record(Some(entity))) => entity.field(field).map(|f| f.ty.clone()),
                    _ => None,
                });
                let fits = matches!((&bound, mode), (Some(FieldType::Image), CanvasMode::Paint) | (Some(FieldType::Text { .. }), CanvasMode::Vector));
                if !fits {
                    self.error(file, format!("{at}: a canvas in mode {} binds {what}", mode.keyword()), widget.span);
                }
            }
        }
    }

    /// A part that reorders (manual/parts.data.md) numbers its rows in an integer
    /// field of the listed entity, follows no sort of its own, and saves the
    /// order in an `on reorder` handler.
    pub(super) fn check_reorder(&mut self, file: usize, at: &str, entity: &Entity, widget: &Widget) {
        let p = &widget.props;
        let Some(field) = p.reorder.as_deref() else { return };
        if entity.record {
            return self.error(file, format!("{at}: {} is a record, which is never stored, so the person cannot put its rows in their own order; build the list in code", entity.name.name), widget.span);
        }
        match entity.field(field) {
            None => self.error(file, format!("{at}: {} has no field '{field}' to keep the order in", entity.name.name), widget.span),
            Some(f) if f.ty != FieldType::Integer => self.error(
                file,
                format!("{at}: '{field}' is {}; the order is kept in an integer field", widgets::field_type_tag(&f.ty)),
                widget.span,
            ),
            Some(_) => {}
        }
        if p.sortable {
            self.error(file, format!("{at}: a table that the person reorders is not 'sortable' too; its rows keep their order"), widget.span);
        }
        if p.order.is_some() {
            self.error(file, format!("{at}: a table that the person reorders has no 'order by'; its rows keep their order"), widget.span);
        }
        if !p.handlers.iter().any(|h| h.event == widgets::Event::Reorder) {
            self.warning(file, format!("{at} reorders its rows but has no 'on reorder' handler to save them"), widget.span);
        }
    }

    /// The edited columns of a table: shown, of a type a cell can edit,
    /// and saved by an `on change` handler.
    pub(super) fn check_edits(&mut self, file: usize, at: &str, entity: &Entity, widget: &Widget) {
        let p = &widget.props;
        if p.edit.is_empty() {
            return;
        }
        if entity.record {
            return self.error(file, format!("{at}: {} is a record, which is never stored, so its rows are not edited in a table; edit them in parts bound to a record", entity.name.name), widget.span);
        }
        for field in &p.edit {
            if !p.columns.contains(field) {
                self.error(file, format!("{at}: '{field}' is edited but is not one of the columns"), widget.span);
                continue;
            }
            let Some(f) = entity.field(field) else { continue };
            if p.switches.contains(field) && f.ty != FieldType::Bool {
                self.error(
                    file,
                    format!("{at}: '{field}' is {}; only a bool column can be a switch", widgets::field_type_tag(&f.ty)),
                    widget.span,
                );
            }
            if !widgets::cell_edits(&f.ty) {
                self.error(
                    file,
                    format!(
                        "{at}: '{field}' is {}; a table edits text, numbers, bool, dates, colors, folders, enum and ref columns",
                        widgets::field_type_tag(&f.ty)
                    ),
                    widget.span,
                );
            }
        }
        if !p.handlers.iter().any(|h| h.event == widgets::Event::Change) {
            self.warning(file, format!("{at} edits its rows but has no 'on change' handler to save them"), widget.span);
        }
    }
}
