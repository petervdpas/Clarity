//! The parser of a window's parts: each part line, a tree's levels and
//! selection, a part's own pages, and its properties.

use super::*;

pub(crate) fn parse_widgets(nodes: &[Node], diagnostics: &mut Vec<Diagnostic>) -> Vec<Widget> {
    let mut widgets = Vec::new();
    for node in nodes {
        match parse_widget(node, diagnostics) {
            Ok(widget) => widgets.push(widget),
            Err(diag) => diagnostics.push(diag),
        }
    }
    widgets
}

pub(crate) fn parse_widget(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<Widget> {
    let mut cursor = Cursor::new(node.line);
    let keyword = cursor.expect_ident("a widget")?;
    // A word in capitals is a custom part, by the name it is declared by (manual/design.custom-parts.md).
    let custom = keyword.name.starts_with(|c: char| c.is_ascii_uppercase());
    let Some(kind) = widgets::by_keyword(&keyword.name).or(custom.then_some(WidgetKind::Custom)) else {
        return Err(Diagnostic::error(
            format!("unknown widget '{}'", keyword.name),
            keyword.span,
        ));
    };
    let mut props = parse_widget_props(&mut cursor, kind, node.line.span)?;
    if custom {
        props.part = Some(keyword.name.clone());
    }

    let mut widget = Widget {
        props,
        children: Vec::new(),
        pages: Vec::new(),
        menu: None,
        levels: Vec::new(),
        selects: Vec::new(),
        span: node.span(),
    };
    // A `menu` block under a part is its context menu (manual/design.parts.md); the rest is its content.
    let (menus, content): (Vec<&Node>, Vec<&Node>) = node.children.iter().partition(|c| Cursor::new(c.line).peek().is_some_and(|t| t.text == "menu"));
    for (i, menu) in menus.iter().enumerate() {
        if i > 0 {
            diagnostics.push(Diagnostic::error("a part has one context menu", menu.line.span));
            continue;
        }
        let mut cursor = Cursor::new(menu.line);
        cursor.eat_keyword("menu");
        match region_head(&mut cursor, "menu", false) {
            Ok((name, _)) => {
                let items = parse_commands(&menu.children, &[CommandKind::Item, CommandKind::Separator], diagnostics);
                widget.menu = Some(Menu { name, width: None, items, span: menu.span() });
            }
            Err(diag) => diagnostics.push(diag),
        }
    }
    let content: Vec<Node> = content.into_iter().cloned().collect();
    let node = &Node { children: content, ..node.clone() };
    // A tree of several kinds of records: its levels and what it runs when a row is picked.
    if kind == WidgetKind::Tree && !node.children.is_empty() {
        for child in &node.children {
            let cursor = Cursor::new(child.line);
            if cursor.peek().is_some_and(|t| t.text == "on") {
                match parse_tree_select(child) {
                    Ok(select) => widget.selects.push(select),
                    Err(diag) => diagnostics.push(diag),
                }
                continue;
            }
            match parse_tree_level(child, diagnostics) {
                Ok(level) => widget.levels.push(level),
                Err(diag) => diagnostics.push(diag),
            }
        }
        return Ok(widget);
    }
    match widgets::container(kind) {
        None => no_children(node)?,
        Some(ContainerKind::Plain) => widget.children = parse_widgets(&node.children, diagnostics),
        Some(container @ (ContainerKind::Tabs | ContainerKind::Steps | ContainerKind::Accordion | ContainerKind::Content)) => {
            let keyword = match container {
                ContainerKind::Tabs => "tab",
                ContainerKind::Steps => "step",
                ContainerKind::Content => "view",
                _ => "section",
            };
            for child in &node.children {
                match parse_widget_page(child, keyword, diagnostics) {
                    Ok(page) => widget.pages.push(page),
                    Err(diag) => diagnostics.push(diag),
                }
            }
        }
        Some(ContainerKind::Splitter) => {
            for child in &node.children {
                match parse_widget_page(child, "pane", diagnostics) {
                    Ok(page) => widget.pages.push(page),
                    Err(diag) => diagnostics.push(diag),
                }
            }
        }
    }
    Ok(widget)
}

/// `level Domain under project show name order by name`, or `group @dataObjectsNode`,
/// with the levels inside it.
pub(crate) fn parse_tree_level(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<TreeLevel> {
    let mut cursor = Cursor::new(node.line);
    let mut level = TreeLevel { entity: None, under: None, show: None, group: None, order: None, descending: false, levels: Vec::new(), span: node.span() };
    if cursor.eat_keyword("group") {
        level.group = Some(cursor.expect_text("the group's heading, as in group @dataObjectsNode")?);
    } else if cursor.eat_keyword("level") {
        level.entity = Some(cursor.expect_ident("the entity whose records the level shows")?.name);
        loop {
            if cursor.eat_keyword("under") {
                level.under = Some(cursor.expect_ident("the field that refers to the row above, as in under project")?.name);
            } else if cursor.eat_keyword("show") {
                level.show = Some(match cursor.peek().map(|t| t.kind) {
                    Some(TokenKind::Ident) => cursor.expect_ident("what a row shows")?.name,
                    _ => cursor.expect_text("a field or a text key, as in show name or show @classNode")?,
                });
            } else if cursor.eat_keyword("order") {
                if !cursor.eat_keyword("by") {
                    return Err(cursor.error_here("expected 'by' and a field, as in order by name"));
                }
                level.order = Some(cursor.expect_ident("the field the rows are ordered by")?.name);
                level.descending = cursor.eat_keyword("desc");
            } else {
                break;
            }
        }
    } else {
        return Err(cursor.error_here("a tree holds 'level' lines, as in level Project show name, 'group' lines, and 'on select Entity: handler'"));
    }
    cursor.expect_end()?;
    for child in &node.children {
        match parse_tree_level(child, diagnostics) {
            Ok(inner) => level.levels.push(inner),
            Err(diag) => diagnostics.push(diag),
        }
    }
    Ok(level)
}

/// `on select ClassDefinition: classPicked`.
pub(crate) fn parse_tree_select(node: &Node) -> ParseResult<TreeSelect> {
    let mut cursor = Cursor::new(node.line);
    cursor.eat_keyword("on");
    if !cursor.eat_keyword("select") {
        return Err(cursor.error_here("a tree of levels runs a handler when a row is picked: on select Project: projectPicked"));
    }
    let entity = cursor.expect_ident("the entity of the rows, as in on select Project: projectPicked")?.name;
    cursor.expect(TokenKind::Colon, "':' and a handler, as in on select Project: projectPicked")?;
    let handler = cursor.expect_ident("a handler name")?.name;
    cursor.expect_end()?;
    Ok(TreeSelect { entity, handler, span: node.span() })
}

/// `tab "Title"` or `pane`, each with widgets inside.
pub(crate) fn parse_widget_page(
    node: &Node,
    keyword: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<WidgetPage> {
    let mut cursor = Cursor::new(node.line);
    if !cursor.eat_keyword(keyword) {
        let expected = match keyword {
            "tab" => "'tab \"Title\"'",
            "step" => "'step \"Title\"'",
            "section" => "'section \"Title\"'",
            "view" => "'view' and the name of a view, as in view OrderEditor",
            _ => "'pane'",
        };
        return Err(cursor.error_here(format!("expected {expected}")));
    }
    let title = if matches!(keyword, "tab" | "step" | "section") {
        Some(cursor.expect_text(match keyword {
            "tab" => "a tab title",
            "step" => "a step title",
            _ => "a section title",
        })?)
    } else {
        None
    };
    // `section "Chapters" color "#2e86de"`: a section's header in a color of its own.
    let color = if keyword == "section" && cursor.eat_keyword("color") {
        let color = match cursor.peek().map(|t| t.kind) {
            Some(TokenKind::Str) => {
                let span = cursor.peek().map(|t| t.span).unwrap_or(node.span());
                let written = cursor.expect_string("a color, as in color \"#2e86de\"")?;
                if !crate::themes::is_hex(&written) {
                    return Err(Diagnostic::error(format!("'{written}' is not a color; write it as \"#rrggbb\" or \"#rgb\", or name a color of the theme"), span));
                }
                written
            }
            _ => cursor.expect_ident("a color, as in color \"#2e86de\" or color accent")?.name,
        };
        Some(color)
    } else {
        None
    };
    // `section "Class" view ClassEditor`: what it holds is the view's, and
    // `on saved: recipeSaved` hears what the view tells.
    // `view OrderEditor` in a window's content: the keyword is the view's own line.
    let view = if keyword == "view" {
        Some(cursor.expect_ident("the name of the view it shows")?.name)
    } else if keyword != "pane" && cursor.eat_keyword("view") {
        Some(cursor.expect_ident("the name of the view it shows")?.name)
    } else {
        None
    };
    let mut tells = Vec::new();
    while keyword != "pane" && cursor.eat_keyword("on") {
        let tell = cursor.expect_ident("a tell of the view it shows")?.name;
        cursor.expect(TokenKind::Colon, "':' and a handler, as in 'on saved: recipeSaved'")?;
        let handler = cursor.expect_ident("a handler name")?.name;
        tells.push(crate::model::TellHandler { tell, handler });
    }
    // `tab "Overview" permanent`: a tab of tabs that close that has no x.
    let permanent = keyword == "tab" && cursor.eat_keyword("permanent");
    let columns = if keyword == "pane" && cursor.eat_keyword("span") {
        let token = cursor.expect(TokenKind::Number, "the columns the pane starts with")?;
        Some(token.text.parse().map_err(|_| Diagnostic::error("a pane's span is a whole number", token.span))?)
    } else {
        None
    };
    cursor.expect_end()?;
    Ok(WidgetPage {
        title,
        columns,
        view,
        tells,
        color,
        permanent,
        children: parse_widgets(&node.children, diagnostics),
        span: node.span(),
    })
}

/// The words of a part's line, which every editor colors as Clarity's own (the VS Code grammar too).
pub const LINE_WORDS: [&str; 98] = [
    "name", "on", "at", "span", "rows", "bind", "columns", "resizable", "sortable", "striped", "fold", "reading", "order", "reorder", "mode", "tools", "brush", "ink", "edit", "show", "surface", "renderer", "source", "for",
    "runs", "units", "digits", "placeholder", "tooltip", "labels", "volume", "repeat", "points", "every", "split", "by", "count", "sum", "average",
    "children", "page", "typed", "cards", "colored", "band", "chip", "frame", "search", "zoom", "report", "save", "print", "pages", "grid", "snap", "background", "dots", "to", "max", "min", "step", "icon", "style", "readonly", "no",
    "foldable", "folded", "taborder", "key", "parent", "default", "cancel", "tells", "hears", "value", "values", "in", "new", "window", "side", "width", "top", "pinnable", "view", "add", "remove", "confirm", "code", "copy", "markdown", "color", "input", "strategy", "shows", "closing", "cycling", "available", "permanent",
];

pub(crate) fn parse_widget_props(
    cursor: &mut Cursor,
    kind: WidgetKind,
    line: Span,
) -> ParseResult<WidgetProps> {
    let name = widgets::keyword(kind);
    let (width, height) = widgets::default_size(kind);
    let mut props = WidgetProps {
        kind,
        caption: None,
        name: None,
        col: 0,
        row: 0,
        width,
        height,
        bind: None,
        on_click: None,
        new_window: false,
        open_with: Vec::new(),
        handlers: Vec::new(),
        columns: Vec::new(),
        edit: Vec::new(),
        links: Vec::new(),
        drops: Vec::new(),
        switches: Vec::new(),
        widths: Default::default(),
        resizable: false,
        side: None,
        beside: None,
        slide_width: None,
        slide_height: None,
        pinnable: false,
        code: None,
        copy: false,
        markdown: false,
        view: None,
        sortable: false,
        add: false,
        remove: false,
        confirm: false,
        striped: false,
        fold_buttons: false,
        reading_width: false,
        extensions: None,
        hidden: false,
        start_folder: None,
        store: None,
        new_file: false,
        reorder: None,
        reorder_from: None,
        mode: None,
        objects: None,
        tools: Vec::new(),
        toolbox: None,
        toolbox_page: None,
        toolbox_columns: None,
        does: None,
        input: None,
        strategy: None,
        shows: None,
        brush: None,
        ink: None,
        order: None,
        show: Vec::new(),
        orientation: None,
        surface: None,
        source: None,
        for_part: None,
        placeholder: None,
        digits: None,
        units: None,
        zoom: false,
        report: None,
        save: false,
        print: false,
        pages: false,
        grid: None,
        snap: false,
        background: None,
        dot_size: None,
        dot_color: None,
        runs: None,
        tooltip: None,
        icon: None,
        to: None,
        max: None,
        min: None,
        step: None,
        split: false,
        cards: false,
        card_color: None,
        card_band: false,
        card_chip: None,
        frame: false,
        typed: false,
        page: None,
        children: None,
        row_key: None,
        parent_key: None,
        chart: None,
        by: None,
        measure: None,
        edges: None,
        node_key: None,
        node_label: None,
        node_kind: None,
        edge_from: None,
        edge_to: None,
        edge_label: None,
        edge_kind: None,
        layout: None,
        node_looks: Default::default(),
        edge_looks: Default::default(),
        limit: None,
        lists: Vec::new(),
        picked_kind: None,
        picked_id: None,
        hops: None,
        legend: false,
        closing: false,
        cycling: false,
        available: false,
        graph_model: None,
        search: false,
        labels: None,
        volume: None,
        repeat: false,
        points: None,
        every: None,
        made: false,
        style: None,
        readonly: false,
        no_undo: false,
        undo_scope: None,
        foldable: false,
        folded: false,
        tab_order: None,
        shortcut: None,
        default_button: false,
        cancel: false,
        bottom: false,
        part: None,
        values: Default::default(),
        tells: Vec::new(),
        grows: false,
        description: None,
    };
    // A chart's first word is its kind; an icon part's the icon it shows, no text people read.
    if kind == WidgetKind::Chart {
        let word = cursor.expect_ident("the kind of chart, as in chart bar")?;
        props.chart = Some(crate::model::ChartKind::of_word(&word.name).ok_or_else(|| Diagnostic::error("a chart is bar, line or pie, as in 'chart bar'", word.span))?);
    } else if kind == WidgetKind::Icon {
        props.icon = Some(cursor.expect_string("the icon it shows, as in icon \"triangle-alert\"")?);
    } else if cursor.peek_is_text() {
        props.caption = Some(cursor.expect_text("a caption")?);
    }

    let mut seen: Vec<String> = Vec::new();
    let mut placed = false;
    while let Some(token) = cursor.peek() {
        let clause = cursor.expect_ident("a widget property")?;
        let word = clause.name.as_str();
        // `horizontal` and `vertical` share one property; every event is one
        // property (`on click`, `on change`).
        let event_word = cursor.peek().map(|t| t.text.as_str()).unwrap_or("");
        let on_event = format!("on {event_word}");
        let property_name = match word {
            "horizontal" | "vertical" => "orientation",
            "on" => on_event.as_str(),
            other => other,
        };
        if seen.iter().any(|s| s == property_name) {
            return Err(Diagnostic::error(
                format!("duplicate '{property_name}'"),
                clause.span,
            ));
        }
        seen.push(property_name.to_string());

        let applies = |property: WidgetProperty| {
            if widgets::has_property(kind, property) {
                Ok(())
            } else {
                let written = if property == WidgetProperty::NoUndo { "no undo" } else { property_name };
                Err(Diagnostic::error(
                    format!("'{written}' does not apply to a {name}"),
                    clause.span,
                ))
            }
        };

        // A custom part's own property, or a tell wired to a handler: its
        // declaration says which there are, and the checker holds it to them.
        if kind == WidgetKind::Custom && !widgets::CUSTOM_PART_WORDS.contains(&word) {
            // A text is shown, so it may be a key: `title @levelOne`.
            let value = if cursor.peek_is_text() { Literal::Str(cursor.expect_text("a text")?) } else { parse_literal(cursor)? };
            props.values.insert(word.to_string(), value);
            continue;
        }
        if kind == WidgetKind::Custom && word == "on" && Event::by_keyword(event_word) != Some(Event::Change) {
            let tell = cursor.expect_ident("a tell of the part")?.name;
            cursor.expect(TokenKind::Colon, "':' and a handler, as in 'on scored: arenaScored'")?;
            let handler = cursor.expect_ident("a handler name")?.name;
            props.tells.push(crate::model::TellHandler { tell, handler });
            continue;
        }
        // A tell of the view a slide-out shows (manual/design.windows.md): the view's `tells` say which there are.
        if kind == WidgetKind::Slideout && word == "on" && Event::by_keyword(event_word).is_none() {
            let tell = cursor.expect_ident("a tell of the view the slide-out shows")?.name;
            cursor.expect(TokenKind::Colon, "':' and a handler, as in 'on saved: recipeSaved'")?;
            let handler = cursor.expect_ident("a handler name")?.name;
            props.tells.push(crate::model::TellHandler { tell, handler });
            continue;
        }
        // A tell of the engine beside the screen a canvas runs (manual/code.engines.md):
        // the engine's declaration says which there are.
        if kind == WidgetKind::Canvas && word == "on" && Event::by_keyword(event_word).is_none() {
            let tell = cursor.expect_ident("a tell of the engine the canvas runs")?.name;
            cursor.expect(TokenKind::Colon, "':' and a handler, as in 'on finished: gameOver'")?;
            let handler = cursor.expect_ident("a handler name")?.name;
            props.tells.push(crate::model::TellHandler { tell, handler });
            continue;
        }
        match word {
            "at" | "span" | "rows" if kind == WidgetKind::Sound => {
                return Err(Diagnostic::error("a sound has no place in the window: it is heard, not seen, so it has no 'at', 'span' or 'rows'", clause.span));
            }
            "at" | "span" | "rows" if kind == WidgetKind::Timer => {
                return Err(Diagnostic::error("a timer has no place in the window: it keeps time, not seen, so it has no 'at', 'span' or 'rows'", clause.span));
            }
            "at" if kind == WidgetKind::Popover => {
                return Err(Diagnostic::error("a popover has no 'at': it opens next to the part named in 'for'", clause.span));
            }
            "at" if kind == WidgetKind::Slideout => {
                return Err(Diagnostic::error("a slide-out has no 'at': it comes in from the side of the window named in 'side'", clause.span));
            }
            "for" if kind == WidgetKind::Slideout => {
                return Err(Diagnostic::error("a slide-out has no 'for': it comes in from the side of the window; a part that opens next to another is a popover", clause.span));
            }
            "side" => {
                applies(WidgetProperty::Side)?;
                let word = cursor.expect_ident("left, right, top or bottom, as in side left")?;
                props.side = Some(match word.name.as_str() {
                    "left" => crate::model::SlideSide::Left,
                    "right" => crate::model::SlideSide::Right,
                    "top" => crate::model::SlideSide::Top,
                    "bottom" => crate::model::SlideSide::Bottom,
                    other => return Err(Diagnostic::error(format!("a slide-out comes in from the left, the right, the top or the bottom, not '{other}': write side left, side right, side top or side bottom"), word.span)),
                });
            }
            "does" => {
                applies(WidgetProperty::Does)?;
                props.does = Some(cursor.expect_ident("what the tool does, as in does pencil")?.name);
            }
            "input" => {
                applies(WidgetProperty::Input)?;
                props.input = Some(cursor.expect_ident("the input it takes, as in input password")?.name);
            }
            "strategy" => {
                applies(WidgetProperty::Strategy)?;
                props.strategy = Some(cursor.expect_ident("the strategy it signs in with, as in strategy password")?.name);
            }
            "shows" => {
                applies(WidgetProperty::Shows)?;
                props.shows = Some(cursor.expect_ident("the sign-in view it shows, as in shows CodeView")?.name);
            }
            "beside" => {
                applies(WidgetProperty::Beside)?;
                props.beside = Some(cursor.expect_ident("the part it lies against, as in beside chapterTable")?.name);
            }
            "width" => {
                applies(WidgetProperty::SlideWidth)?;
                props.slide_width = Some(cursor.expect_number("a width in pixels, as in width 420")?);
            }
            "height" if kind == WidgetKind::Slideout => {
                applies(WidgetProperty::SlideHeight)?;
                props.slide_height = Some(cursor.expect_number("a height in pixels, as in height 300")?);
            }
            "pinnable" => {
                applies(WidgetProperty::Pinnable)?;
                props.pinnable = true;
            }
            // Tabs the person closes, goes through, and brings back.
            "closing" => {
                applies(WidgetProperty::Closing)?;
                props.closing = true;
            }
            "cycling" => {
                applies(WidgetProperty::Cycling)?;
                props.cycling = true;
            }
            "available" => {
                applies(WidgetProperty::Available)?;
                props.available = true;
            }
            // `code csharp`: a memo colors its text as code of the language.
            "code" => {
                applies(WidgetProperty::Code)?;
                let language = cursor.expect_ident("a language, as in code csharp")?;
                if !widgets::CODE_LANGUAGES.contains(&language.name.as_str()) {
                    return Err(Diagnostic::error(format!("a memo colors code of {}, not '{}'", widgets::CODE_LANGUAGES.join(", "), language.name), language.span));
                }
                if props.markdown {
                    return Err(Diagnostic::error("a markdown memo colors its Markdown itself; leave out 'code'", language.span));
                }
                props.code = Some(language.name);
            }
            "copy" => {
                applies(WidgetProperty::Copy)?;
                props.copy = true;
            }
            // `markdown`: a memo of Markdown, with Write and Preview.
            "markdown" => {
                applies(WidgetProperty::Markdown)?;
                if props.code.is_some() {
                    return Err(Diagnostic::error("a markdown memo colors its Markdown itself; leave out 'code'", clause.span));
                }
                props.markdown = true;
            }
            "view" => {
                applies(WidgetProperty::View)?;
                props.view = Some(cursor.expect_ident("the view it shows, as in view ClassEditor")?.name);
            }
            "at" => {
                props.col = cursor.expect_number("a column")?;
                cursor.expect(TokenKind::Comma, "',' and a row, as in 'at 1,2'")?;
                props.row = cursor.expect_number("a row")?;
                placed = true;
            }
            "span" => props.width = cursor.expect_number("a number of columns")?,
            "rows" => props.height = cursor.expect_number("a number of rows")?,
            "bottom" if widgets::off_grid(kind) => {
                return Err(Diagnostic::error(format!("a {name} has no place in the window's grid, so no 'bottom'"), clause.span));
            }
            "bottom" => props.bottom = true,
            "bind" => {
                if widgets::info(kind).bind == BindTarget::Nothing {
                    return Err(Diagnostic::error(
                        format!("a {name} cannot be bound to data"),
                        clause.span,
                    ));
                }
                let mut path = cursor.expect_ident("a data name")?.name;
                if cursor.eat(TokenKind::Dot) {
                    path.push('.');
                    path.push_str(&cursor.expect_ident("a field name")?.name);
                }
                props.bind = Some(path);
            }
            "name" => props.name = Some(cursor.expect_ident("a part name")?.name),
            "on" => {
                let event_ident = cursor.expect_ident("an event")?;
                let Some(event) = Event::by_keyword(&event_ident.name) else {
                    return Err(Diagnostic::error(
                        format!("unknown event '{}'", event_ident.name),
                        event_ident.span,
                    ));
                };
                // `on drop from fragmentList: linkFragment`: a row of that part dragged onto this one.
                if event == Event::Drop && cursor.eat_keyword("from") {
                    // A memo takes them too: dropped at a place in its text, the caret goes there first.
                    if !matches!(kind, WidgetKind::List | WidgetKind::Table | WidgetKind::Tree | WidgetKind::TreeTable | WidgetKind::Memo) {
                        return Err(Diagnostic::error(format!("a {name} takes no rows dragged onto it; a list, table, tree or memo does"), event_ident.span));
                    }
                    let from = cursor.expect_ident("the part whose rows are dragged here, as in on drop from fragmentList: linkFragment")?.name;
                    cursor.expect(TokenKind::Colon, "':' and the handler the drop runs")?;
                    let handler = cursor.expect_ident("the handler the drop runs")?.name;
                    props.drops.push(crate::model::DropFrom { from, handler });
                    continue;
                }
                // `on change: name_changed` wires a handler in the code behind.
                if cursor.eat(TokenKind::Colon) {
                    if !widgets::has_event(kind, event) {
                        return Err(Diagnostic::error(
                            format!("a {name} has no {} event", event.keyword()),
                            event_ident.span,
                        ));
                    }
                    let handler = cursor.expect_ident("a handler name")?.name;
                    // `digit(7)`: the values the handler runs with.
                    let mut args = Vec::new();
                    if cursor.eat(TokenKind::LParen) && !cursor.eat(TokenKind::RParen) {
                        loop {
                            args.push(parse_literal(cursor)?);
                            if cursor.eat(TokenKind::RParen) {
                                break;
                            }
                            cursor.expect(TokenKind::Comma, "',' or ')' after a value")?;
                        }
                    }
                    props.handlers.push(Handler { event, handler, args });
                    continue;
                }
                // `on click -> Window` opens a window.
                if event != Event::Click {
                    return Err(Diagnostic::error(
                        format!("'on {} ->' is not supported; wire a handler with 'on {}: handler_name'", event.keyword(), event.keyword()),
                        event_ident.span,
                    ));
                }
                if !widgets::info(kind).clickable {
                    return Err(Diagnostic::error(
                        format!("a {name} has no click event"),
                        event_ident.span,
                    ));
                }
                cursor.expect(TokenKind::Arrow, "'->' and a target")?;
                props.on_click = Some(click_target(cursor, "a window")?);
                // `with book, chapter`: the opened window's data, from the data of the same name here.
                if cursor.eat_keyword("with") {
                    loop {
                        props.open_with.push(cursor.expect_ident("the name of data to open the window with")?.name);
                        if !cursor.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                if cursor.eat_keyword("in") {
                    if !(cursor.eat_keyword("new") && cursor.eat_keyword("window")) {
                        return Err(cursor.error_here("expected 'in new window'".to_string()));
                    }
                    props.new_window = true;
                }
            }
            "columns" => {
                applies(WidgetProperty::Columns)?;
                // `columns title 240, artist`: a width in pixels may follow a field.
                loop {
                    let field = cursor.expect_ident("a field name")?.name;
                    if cursor.peek().is_some_and(|t| t.kind == TokenKind::Number) {
                        let at = cursor.peek().map(|t| t.span).unwrap_or(clause.span);
                        let width = cursor.expect_number("a width")?;
                        if !(widgets::MIN_COLUMN_WIDTH..=widgets::MAX_COLUMN_WIDTH).contains(&width) {
                            return Err(Diagnostic::error(widgets::COLUMN_WIDTH_PROBLEM.to_string(), at));
                        }
                        props.widths.insert(field.clone(), width);
                    }
                    props.columns.push(field);
                    if !cursor.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            "resizable" => {
                applies(WidgetProperty::Resizable)?;
                props.resizable = true;
            }
            "sortable" => {
                applies(WidgetProperty::Sortable)?;
                props.sortable = true;
            }
            "add" => {
                applies(WidgetProperty::Add)?;
                props.add = true;
            }
            "remove" => {
                applies(WidgetProperty::Remove)?;
                props.remove = true;
                // `remove confirm`: it asks first.
                if cursor.peek().is_some_and(|t| t.text == "confirm") {
                    cursor.next();
                    props.confirm = true;
                }
            }
            // `fold buttons`: Expand all and Collapse all above a tree's rows.
            "fold" => {
                applies(WidgetProperty::FoldButtons)?;
                let word = cursor.expect_ident("buttons, as in fold buttons")?;
                if word.name != "buttons" {
                    return Err(Diagnostic::error(format!("expected buttons after fold, as in fold buttons, not '{}'", word.name), word.span));
                }
                props.fold_buttons = true;
            }
            // `extensions "*.pdf"`: the files a file part's dialog offers.
            "extensions" => {
                applies(WidgetProperty::Extensions)?;
                let written = cursor.expect_string("the files it offers, as in 'extensions \".png, .jpg, .gif\"', or \"*.*\" for any file")?;
                let patterns = crate::files::patterns_of(&written);
                if let Some(problem) = crate::files::patterns_problem(&patterns) {
                    return Err(Diagnostic::error(problem, clause.span));
                }
                props.extensions = Some(crate::files::written_of(&patterns));
            }
            "hidden" => {
                applies(WidgetProperty::Hidden)?;
                props.hidden = true;
            }
            // `start folder "videos"`: where the dialog opens while nothing is chosen.
            "start" => {
                applies(WidgetProperty::StartFolder)?;
                let word = cursor.expect_ident("folder, as in start folder \"videos\"")?;
                if word.name != "folder" {
                    return Err(Diagnostic::error(format!("expected folder after start, as in start folder \"videos\", not '{}'", word.name), word.span));
                }
                props.start_folder = Some(cursor.expect_string("the folder, as in start folder \"videos\"")?);
            }
            // `store uploads`: the file store a web app keeps a picked file in.
            "store" => {
                applies(WidgetProperty::Store)?;
                props.store = Some(cursor.expect_ident("a file store, as in store uploads")?.name);
            }
            // `new file`: the dialog may name a file that is not there yet.
            "new" => {
                applies(WidgetProperty::NewFile)?;
                let word = cursor.expect_ident("file, as in new file")?;
                if word.name != "file" {
                    return Err(Diagnostic::error(format!("expected file after new, as in new file, not '{}'", word.name), word.span));
                }
                props.new_file = true;
            }
            // `reading width`: Markdown in a column of a reading width.
            "reading" => {
                applies(WidgetProperty::ReadingWidth)?;
                let word = cursor.expect_ident("width, as in reading width")?;
                if word.name != "width" {
                    return Err(Diagnostic::error(format!("expected width after reading, as in reading width, not '{}'", word.name), word.span));
                }
                props.reading_width = true;
            }
            "striped" => {
                applies(WidgetProperty::Striped)?;
                props.striped = true;
            }
            "order" => {
                applies(WidgetProperty::OrderBy)?;
                if !cursor.eat_keyword("by") {
                    return Err(cursor.error_here("expected 'by', as in 'order by title'".to_string()));
                }
                let field = cursor.expect_ident("a column to order by")?.name;
                let desc = cursor.eat_keyword("desc");
                props.order = Some(crate::model::TableOrder { field, desc });
            }
            // `mode objects types at x, y`, `mode paint`, `mode vector` (manual/parts.graphics.md).
            "mode" => {
                applies(WidgetProperty::Mode)?;
                let word = cursor.expect_ident("objects, paint or vector, as in mode paint")?;
                props.mode = Some(match word.name.as_str() {
                    "objects" => {
                        let list = cursor.expect_ident("the list whose records are the objects, as in mode objects types at x, y")?.name;
                        if !cursor.eat_keyword("at") {
                            return Err(cursor.error_here("expected 'at', as in mode objects types at x, y".to_string()));
                        }
                        let x = cursor.expect_ident("the field of an object's x")?.name;
                        if !cursor.eat(TokenKind::Comma) {
                            return Err(cursor.error_here("expected ',' and the field of an object's y, as in at x, y".to_string()));
                        }
                        let y = cursor.expect_ident("the field of an object's y")?.name;
                        props.objects = Some(crate::model::CanvasObjects { list, x, y });
                        crate::model::CanvasMode::Objects
                    }
                    "paint" => crate::model::CanvasMode::Paint,
                    "vector" => crate::model::CanvasMode::Vector,
                    other => return Err(Diagnostic::error(format!("a canvas's mode is objects, paint or vector, not '{other}'"), word.span)),
                });
            }
            "toolbox" => {
                applies(WidgetProperty::Toolbox)?;
                // `toolbox` alone floats; `toolbox slideout` lies against the canvas's edge; `toolbox Painting` shows that toolbox page.
                if cursor.peek().is_some_and(|t| t.kind == TokenKind::Ident && t.text.chars().next().is_some_and(char::is_uppercase)) {
                    props.toolbox_page = Some(cursor.expect_ident("a toolbox")?.name);
                } else if kind == WidgetKind::Memo {
                    // A memo has no built-in tools: its toolbox is a page of its own, with coded tools.
                    return Err(Diagnostic::error("a memo's toolbox is a page of its own: toolbox WritingTools, written 'page WritingTools: toolbox'", clause.span));
                }
                // A memo's comes at the caret unless it is written to float.
                props.toolbox = Some(if cursor.eat_keyword("slideout") {
                    crate::model::ToolboxKind::Slideout
                } else if cursor.eat_keyword("popup") || kind != WidgetKind::Memo {
                    crate::model::ToolboxKind::Popup
                } else {
                    crate::model::ToolboxKind::Caret
                });
            }
            "tools" => {
                applies(WidgetProperty::Tools)?;
                loop {
                    props.tools.push(cursor.expect_ident("a tool, as in tools pen, eraser")?.name);
                    if !cursor.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            "brush" => {
                applies(WidgetProperty::Brush)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or(clause.span);
                let width = cursor.expect_number("how wide the brush is, in pixels, as in brush 4")?;
                if !(1..=100).contains(&width) {
                    return Err(Diagnostic::error(format!("a brush is 1 to 100 pixels wide, not {width}"), at));
                }
                props.brush = Some(width);
            }
            "ink" => {
                applies(WidgetProperty::Ink)?;
                props.ink = Some(match cursor.peek().map(|t| t.kind) {
                    Some(TokenKind::Str) => cursor.expect_string("a color")?,
                    _ => cursor.expect_ident("a color of the theme, as in ink accent")?.name,
                });
            }
            "reorder" => {
                applies(WidgetProperty::Reorder)?;
                if !cursor.eat_keyword("by") {
                    return Err(cursor.error_here("expected 'by', as in 'reorder by position'".to_string()));
                }
                props.reorder = Some(cursor.expect_ident("the integer field that keeps the order")?.name);
                // `from 0`: the first row's number, for lists that count from 0.
                if cursor.eat_keyword("from") {
                    props.reorder_from = Some(i64::from(cursor.expect_number("the number the first row gets, as in 'from 0'")?));
                }
            }
            "link" => {
                applies(WidgetProperty::Link)?;
                // `link chapter: toChapter, layer: toLayer`: columns whose cells are links.
                loop {
                    let column = cursor.expect_ident("a column, as in link chapter: toChapter")?.name;
                    cursor.expect(TokenKind::Colon, "':' and the handler a click on the link runs")?;
                    let handler = cursor.expect_ident("the handler a click on the link runs")?.name;
                    props.links.push(crate::model::ColumnLink { column, handler });
                    if !cursor.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            "edit" => {
                applies(WidgetProperty::Edit)?;
                // `edit title, done as switch`: a bool column can be a switch.
                loop {
                    let field = cursor.expect_ident("a field name")?.name;
                    if cursor.eat_keyword("as") {
                        if !cursor.eat_keyword("switch") {
                            return Err(cursor.error_here("expected 'switch', as in 'edit done as switch'".to_string()));
                        }
                        props.switches.push(field.clone());
                    }
                    props.edit.push(field);
                    if !cursor.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            "show" => {
                applies(WidgetProperty::Show)?;
                props.show = cursor
                    .ident_list("a field name")?
                    .into_iter()
                    .map(|i| i.name)
                    .collect();
            }
            "horizontal" | "vertical" => {
                applies(WidgetProperty::Orientation)?;
                props.orientation = Some(if word == "horizontal" {
                    Orientation::Horizontal
                } else {
                    Orientation::Vertical
                });
            }
            "surface" => {
                applies(WidgetProperty::Surface)?;
                let surface = cursor.expect_ident("'svg', 'bitmap' or 'gpu'")?;
                props.surface = Some(match surface.name.as_str() {
                    "svg" => Surface::Svg,
                    "bitmap" => Surface::Bitmap,
                    "gpu" => Surface::Gpu,
                    other => {
                        return Err(Diagnostic::error(
                            format!("unknown surface '{other}', expected 'svg', 'bitmap' or 'gpu'"),
                            surface.span,
                        ));
                    }
                });
            }
            // Graphics an outside library draws are a custom part now (manual/design.custom-parts.md).
            "renderer" => {
                return Err(Diagnostic::error("a canvas is drawn by its draw handler; graphics an outside library draws are a custom part (part Name with native \"folder\")", clause.span));
            }
            "source" => {
                applies(WidgetProperty::Source)?;
                props.source = Some(cursor.expect_string("a file name")?);
            }
            "for" => {
                applies(WidgetProperty::For)?;
                props.for_part = Some(cursor.expect_ident("a part name")?.name);
            }
            "runs" => {
                applies(WidgetProperty::Runs)?;
                props.runs = Some(cursor.expect_ident("the engine beside the screen that draws it, as in 'runs Screen'")?.name);
            }
            "units" => {
                applies(WidgetProperty::Units)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or_default();
                let width = cursor.expect_number("a width in units")?;
                if !cursor.eat_keyword("x") {
                    return Err(cursor.error_here("expected 'x' and a height, as in 'units 200 x 100'"));
                }
                let height = cursor.expect_number("a height in units")?;
                if !widgets::canvas_units_fit(width, height) {
                    return Err(Diagnostic::error(widgets::CANVAS_UNITS_PROBLEM, at));
                }
                props.units = Some(crate::model::CanvasUnits { width, height });
            }
            "digits" => {
                applies(WidgetProperty::Digits)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or_default();
                let digits = cursor.expect_number("a number of digits")?;
                if !(1..=widgets::MAX_DIGITS).contains(&digits) {
                    return Err(Diagnostic::error(format!("a display has 1 to {} digits", widgets::MAX_DIGITS), at));
                }
                props.digits = Some(digits);
            }
            "placeholder" => {
                applies(WidgetProperty::Placeholder)?;
                props.placeholder = Some(cursor.expect_text("a placeholder text")?);
            }
            "tooltip" => props.tooltip = Some(cursor.expect_text("a tooltip text")?),
            "labels" => {
                applies(WidgetProperty::Labels)?;
                props.labels = Some(cursor.expect_number("the columns of its label column, as in labels 3")?);
            }
            "volume" => {
                applies(WidgetProperty::Volume)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or_default();
                let volume = cursor.expect_number("a volume from 0 to 100")?;
                if volume > 100 {
                    return Err(Diagnostic::error("a sound's volume is 0 to 100", at));
                }
                props.volume = Some(volume);
            }
            "repeat" => {
                applies(WidgetProperty::Repeat)?;
                props.repeat = true;
            }
            "points" => {
                applies(WidgetProperty::Points)?;
                props.points = Some(cursor.expect_number("how many values it shows, as in points 60")?);
            }
            "every" => {
                applies(WidgetProperty::Every)?;
                props.every = Some(cursor.expect(TokenKind::Number, "a number of seconds, as in every 1")?.text.clone());
            }
            "split" => {
                applies(WidgetProperty::Split)?;
                props.split = true;
            }
            "by" => {
                applies(WidgetProperty::By)?;
                props.by = Some(cursor.expect_ident("the field a chart groups its rows by, as in by status")?.name);
            }
            "count" => {
                applies(WidgetProperty::Measure)?;
                props.measure = Some(crate::model::ChartMeasure::Count);
            }
            "sum" => {
                applies(WidgetProperty::Measure)?;
                props.measure = Some(crate::model::ChartMeasure::Sum { field: cursor.expect_ident("the number field it sums, as in sum total")?.name });
            }
            "average" => {
                applies(WidgetProperty::Measure)?;
                props.measure = Some(crate::model::ChartMeasure::Average { field: cursor.expect_ident("the number field it averages, as in average total")?.name });
            }
            "children" => {
                applies(WidgetProperty::Children)?;
                props.children = Some(cursor.expect_ident("the list field of the rows' own kind, as in children children")?.name);
            }
            "page" => {
                applies(WidgetProperty::Page)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or(clause.span);
                let size = cursor.expect_number("the rows of a page, as in page 50")?;
                if !widgets::PAGE_SIZES.contains(&size) {
                    return Err(Diagnostic::error(widgets::PAGE_PROBLEM.to_string(), at));
                }
                props.page = Some(size);
            }
            "typed" => {
                applies(WidgetProperty::Typed)?;
                props.typed = true;
            }
            "cards" => {
                applies(WidgetProperty::Cards)?;
                props.cards = true;
            }
            "colored" => {
                applies(WidgetProperty::CardColor)?;
                if !cursor.eat_keyword("by") {
                    return Err(cursor.error_here("expected 'by', as in 'colored by paint'".to_string()));
                }
                props.card_color = Some(cursor.expect_ident("the color field each card takes")?.name);
                if cursor.eat_keyword("band") {
                    props.card_band = true;
                } else if cursor.eat_keyword("chip") {
                    props.card_chip = Some(cursor.expect_ident("the shown field the chip is around, as in 'chip category'")?.name);
                }
            }
            "frame" => {
                applies(WidgetProperty::Frame)?;
                props.frame = true;
            }
            "search" => {
                applies(WidgetProperty::Search)?;
                props.search = true;
            }
            "zoom" => {
                applies(WidgetProperty::Zoom)?;
                props.zoom = true;
            }
            "report" => {
                applies(WidgetProperty::Report)?;
                props.report = Some(cursor.expect_ident("the report it shows, as in 'report Invoice'")?.name);
            }
            "save" => {
                applies(WidgetProperty::Save)?;
                props.save = true;
            }
            "print" => {
                applies(WidgetProperty::Print)?;
                props.print = true;
            }
            "pages" => {
                applies(WidgetProperty::Pages)?;
                props.pages = true;
            }
            "grid" => {
                applies(WidgetProperty::Grid)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or_default();
                let grid = cursor.expect_number("the grid's size in units, as in grid 20")?;
                if !(1..=10000).contains(&grid) {
                    return Err(Diagnostic::error(widgets::CANVAS_GRID_PROBLEM, at));
                }
                props.grid = Some(grid);
            }
            "snap" => {
                applies(WidgetProperty::Snap)?;
                props.snap = true;
            }
            "dots" => {
                applies(WidgetProperty::Dots)?;
                props.dot_size = Some(cursor.expect_number("the dots' size in pixels, as in dots 3")?);
                // Their color, when it follows: a string, or a name that is no word of the line.
                props.dot_color = match cursor.peek_kind() {
                    Some(TokenKind::Str) => Some(cursor.expect_string("a color")?),
                    Some(TokenKind::Ident) if cursor.peek().is_some_and(|t| !LINE_WORDS.contains(&t.text.as_str())) => Some(cursor.expect_ident("a color")?.name),
                    _ => None,
                };
            }
            "background" => {
                applies(WidgetProperty::Background)?;
                props.background = Some(if matches!(cursor.peek_kind(), Some(TokenKind::Str)) {
                    cursor.expect_string("a color, as in background panel or background \"#101810\"")?
                } else {
                    cursor.expect_ident("a color, as in background panel or background \"#101810\"")?.name
                });
            }
            "to" => {
                applies(WidgetProperty::To)?;
                props.to = Some(cursor.expect_string("an address, as in to \"https://lucide.dev\"")?);
            }
            "max" => {
                applies(WidgetProperty::Max)?;
                let what = match kind {
                    WidgetKind::Progress | WidgetKind::StatusItem => "the number it is full at, as in max 250",
                    WidgetKind::Slideout => "the widest it is dragged, in pixels, as in max 900",
                    _ => "the highest value, as in max 100",
                };
                props.max = Some(cursor.expect(TokenKind::Number, what)?.text.clone());
            }
            "min" => {
                applies(WidgetProperty::Min)?;
                let what = if kind == WidgetKind::Slideout { "the narrowest it is dragged, in pixels, as in min 300" } else { "the lowest value, as in min 0" };
                props.min = Some(cursor.expect(TokenKind::Number, what)?.text.clone());
            }
            "step" => {
                applies(WidgetProperty::Step)?;
                props.step = Some(cursor.expect(TokenKind::Number, "how far it moves at a time, as in step 5")?.text.clone());
            }
            "icon" => {
                applies(WidgetProperty::Icon)?;
                props.icon = Some(cursor.expect_string("an icon's name, as in icon \"save\"")?);
            }
            "style" => props.style = Some(cursor.expect_ident("a style of the project's theme")?.name),
            "readonly" => {
                applies(WidgetProperty::Readonly)?;
                props.readonly = true;
            }
            // `no undo`: the part has no undo of its own (manual/code.handlers.md).
            "no" => {
                applies(WidgetProperty::NoUndo)?;
                let word = cursor.expect_ident("undo, as in no undo")?;
                if word.name != "undo" {
                    return Err(Diagnostic::error(format!("expected undo after no, as in no undo, not '{}'", word.name), word.span));
                }
                props.no_undo = true;
            }
            "foldable" => {
                applies(WidgetProperty::Foldable)?;
                props.foldable = true;
            }
            "folded" => {
                if !props.foldable {
                    return Err(Diagnostic::error("'folded' needs 'foldable': write 'foldable folded'", clause.span));
                }
                props.folded = true;
            }
            "taborder" => {
                applies(WidgetProperty::TabOrder)?;
                let order = cursor.expect_number("a position in the tab order")?;
                if order == 0 {
                    return Err(Diagnostic::error("tab order starts at 1", clause.span));
                }
                props.tab_order = Some(order);
            }
            // A tree's key field (`key id`), beside a shortcut (`key "Ctrl+S"`).
            "key" if widgets::has_property(kind, WidgetProperty::Children) && cursor.peek().is_some_and(|t| t.kind == TokenKind::Ident) => {
                props.row_key = Some(cursor.expect_ident("the field that names a row, as in key id")?.name);
            }
            "parent" => {
                applies(WidgetProperty::Children)?;
                props.parent_key = Some(cursor.expect_ident("the field that holds the key of a row's parent, as in parent parentId")?.name);
            }
            "key" => {
                applies(WidgetProperty::Shortcut)?;
                let text = cursor.peek().map(|t| t.span).unwrap_or(clause.span);
                let shortcut = cursor.expect_string("a shortcut, as in key \"Ctrl+S\"")?;
                widgets::check_shortcut(&shortcut).map_err(|m| Diagnostic::error(m, text))?;
                props.shortcut = Some(shortcut);
            }
            "default" => {
                applies(WidgetProperty::Default)?;
                props.default_button = true;
            }
            "cancel" => {
                applies(WidgetProperty::Cancel)?;
                props.cancel = true;
            }
            _ => {
                return Err(Diagnostic::error(
                    format!("unknown widget property '{}'", token.text),
                    token.span,
                ));
            }
        }
    }

    // A popover stands outside the window's grid, on a grid of its own; a
    // sound has no place at all.
    if widgets::off_grid(kind) {
        props.col = 1;
        props.row = 1;
        placed = true;
    }
    // A tool in a tool group lies in its flyout, not on a grid: it needs no place.
    if kind == WidgetKind::Tool && !placed {
        props.col = 1;
        props.row = 1;
        placed = true;
    }
    if !placed {
        return Err(Diagnostic::error(
            format!("a {name} needs a position, as in 'at 1,1'"),
            line,
        ));
    }
    if kind == WidgetKind::Slideout {
        widgets::slide_widths_fit(&props).map_err(|m| Diagnostic::error(m, line))?;
    }
    Ok(props)
}
