//! The parser of the app and its frame: the app's settings, its regions
//! (menus, toolbars, ribbons, sidebars, footers) and its commands.

use super::*;

// ---------------------------------------------------------------------------
// Apps and frame regions

/// App settings (`title`, `main`, `data`) as opposed to regions.
pub(crate) fn is_app_setting(tokens: &[Token]) -> bool {
    starts_with_keyword(tokens, "title", Some(TokenKind::Str))
        || starts_with_keyword(tokens, "main", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "splash", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "on", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "data", Some(TokenKind::Ident))
}

/// A region block: `menu`, `toolbar`, `ribbon`, `footer` or
/// `sidebar left span 3`.
pub(crate) fn is_region_line(tokens: &[Token]) -> bool {
    tokens
        .first()
        .is_some_and(|t| t.kind == TokenKind::Ident && frame::by_keyword(&t.text).is_some())
}

pub(crate) fn parse_app(
    node: &Node,
    mut cursor: Cursor,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<App> {
    let name = cursor.expect_ident("an app name")?;
    cursor.expect_end()?;
    let mut app = App {
        name,
        title: None,
        main: None,
        splash: None,
        splash_for: None,
        data: Vec::new(),
        frame: Frame::default(),
        handlers: Vec::new(),
        span: node.span(),
    };
    for child in &node.children {
        let result = if is_app_setting(&child.line.tokens) {
            parse_app_setting(child, &mut app)
        } else if is_region_line(&child.line.tokens) {
            parse_region(child, &mut app.frame, diagnostics)
        } else {
            let c = Cursor::new(child.line);
            Err(c.error_here(
                "expected 'title', 'main', 'data', 'menu', 'toolbar', 'ribbon', 'sidebar' or 'footer'",
            ))
        };
        if let Err(diag) = result {
            diagnostics.push(diag);
        }
    }
    Ok(app)
}

pub(crate) fn parse_app_setting(node: &Node, app: &mut App) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    let keyword = cursor.expect_ident("an app setting")?;
    match keyword.name.as_str() {
        "title" => {
            if app.title.is_some() {
                return Err(Diagnostic::error("duplicate 'title'", node.line.span));
            }
            app.title = Some(cursor.expect_text("a title string")?);
        }
        "main" => {
            if app.main.is_some() {
                return Err(Diagnostic::error("duplicate 'main'", node.line.span));
            }
            app.main = Some(cursor.expect_ident("the main window")?);
        }
        // `splash Welcome for 2 seconds`: the splash page, and how long it shows at least.
        "splash" => {
            if app.splash.is_some() {
                return Err(Diagnostic::error("duplicate 'splash'", node.line.span));
            }
            app.splash = Some(cursor.expect_ident("the splash page")?);
            if cursor.eat_keyword("for") {
                let wrong = || Diagnostic::error("write 'for' and how many seconds the splash shows at least, from 0.1 to 30: splash Welcome for 2 seconds", node.line.span);
                let number = cursor.expect(TokenKind::Number, "a number of seconds").map_err(|_| wrong())?;
                let seconds: f64 = number.text.parse().map_err(|_| wrong())?;
                if !(cursor.eat_keyword("seconds") || cursor.eat_keyword("second")) || !(0.1..=30.0).contains(&seconds) {
                    return Err(wrong());
                }
                app.splash_for = Some((seconds * 1000.0).round() as u32);
            }
            cursor.expect_end()?;
        }
        "on" => {
            let owner = app.name.name.clone();
            parse_owner_event(
                &mut cursor,
                &mut app.handlers,
                &APP_EVENTS,
                |e| format!("the app has the event start, not {e}"),
                &owner,
            )?;
        }
        _ => app.data.push(parse_data(&mut cursor, node.line.span)?),
    }
    cursor.expect_end()
}

pub(crate) fn parse_region(
    node: &Node,
    frame: &mut Frame,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<()> {
    let mut cursor = Cursor::new(node.line);
    let keyword = cursor.expect_ident("a region")?;
    let region = frame::by_keyword(&keyword.name).expect("checked by is_region_line");
    let duplicate = || Diagnostic::error(format!("duplicate '{}'", keyword.name), keyword.span);
    match region {
        RegionKind::Menu => {
            let (name, width) = region_head(&mut cursor, "menu", true)?;
            if frame.menu.is_some() {
                return Err(duplicate());
            }
            let items = parse_commands(
                &node.children,
                &[CommandKind::Item, CommandKind::Separator],
                diagnostics,
            );
            frame.menu = Some(Menu {
                name,
                width,
                items,
                span: node.span(),
            });
        }
        RegionKind::Toolbar => {
            let (name, width) = region_head(&mut cursor, "toolbar", true)?;
            if frame.toolbar.is_some() {
                return Err(duplicate());
            }
            let items = parse_commands(
                &node.children,
                &[CommandKind::Button, CommandKind::Separator],
                diagnostics,
            );
            frame.toolbar = Some(Toolbar {
                name,
                width,
                items,
                span: node.span(),
            });
        }
        RegionKind::Ribbon => {
            let (mut name, mut collapsible, mut collapsed) = (None, false, false);
            while let Some(word) = cursor.peek().map(|t| t.text.clone()) {
                let clause = cursor.expect_ident("a ribbon property")?;
                match word.as_str() {
                    "name" => name = Some(cursor.expect_ident("a part name")?.name),
                    "collapsible" => collapsible = true,
                    "collapsed" => collapsed = true,
                    other => {
                        return Err(Diagnostic::error(
                            format!("unknown ribbon property '{other}'; a ribbon has name, collapsible and collapsed"),
                            clause.span,
                        ));
                    }
                }
            }
            if frame.ribbon.is_some() {
                return Err(duplicate());
            }
            let mut tabs = Vec::new();
            for child in &node.children {
                match parse_ribbon_tab(child, diagnostics) {
                    Ok(tab) => tabs.push(tab),
                    Err(diag) => diagnostics.push(diag),
                }
            }
            frame.ribbon = Some(Ribbon {
                name,
                collapsible,
                collapsed,
                tabs,
                span: node.span(),
            });
        }
        RegionKind::Sidebar => {
            let side = cursor.expect_ident("'left' or 'right', as in 'sidebar left name nav'")?;
            let side = match side.name.as_str() {
                "left" => Side::Left,
                "right" => Side::Right,
                other => {
                    return Err(Diagnostic::error(
                        format!("unknown side '{other}', expected 'left' or 'right'"),
                        side.span,
                    ));
                }
            };
            let mut props = frame::new_sidebar(side, None);
            let mut old_columns = None;
            while let Some(word) = cursor.peek().map(|t| t.text.clone()) {
                let clause = cursor.expect_ident("a sidebar property")?;
                match word.as_str() {
                    "name" => props.name = Some(cursor.expect_ident("a part name")?.name),
                    "width" => props.width = cursor.expect_number("a width in pixels")?,
                    "min" => props.min = Some(cursor.expect_number("a width in pixels")?),
                    "max" => props.max = Some(cursor.expect_number("a width in pixels")?),
                    "resizable" => props.resizable = true,
                    "collapsible" => props.collapsible = true,
                    "collapsed" => props.collapsed = true,
                    "maximizable" => props.maximizable = true,
                    "grid" => {
                        props.columns = cursor.expect_number("a number of columns")?;
                        if !cursor.eat_keyword("columns") {
                            return Err(Diagnostic::error("expected 'columns', as in 'grid 4 columns'", clause.span));
                        }
                    }
                    // The old sidebar, `span 3` columns wide: read, for its fix.
                    "span" => old_columns = Some(cursor.expect_number("a number of columns")?),
                    other => {
                        return Err(Diagnostic::error(
                            format!("unknown sidebar property '{other}'; a sidebar has name, width, min, max, resizable, collapsible, collapsed, maximizable and grid"),
                            clause.span,
                        ));
                    }
                }
            }
            if let Some(columns) = old_columns {
                diagnostics.push(Diagnostic::error(
                    format!("a sidebar's width is in pixels now; Update the sidebars, in Problems, writes it so ('width {} grid {columns} columns')", columns * crate::layout::SIDEBAR_COLUMN),
                    keyword.span,
                ));
                props.columns = columns;
                props.width = columns * crate::layout::SIDEBAR_COLUMN;
            }
            if frame.sidebars.iter().any(|s| s.props.side == side) {
                return Err(Diagnostic::error(
                    format!("duplicate '{} {}'", keyword.name, frame::side_keyword(side)),
                    keyword.span,
                ));
            }
            frame.sidebars.push(Sidebar {
                props,
                widgets: parse_widgets(&node.children, diagnostics),
                span: node.span(),
            });
        }
        RegionKind::Footer => {
            cursor.expect_end()?;
            if frame.footer.is_some() {
                return Err(duplicate());
            }
            let mut footer = Footer {
                status: None,
                status_span: None,
                columns: FOOTER_COLUMNS,
                widgets: Vec::new(),
                span: node.span(),
            };
            for child in &node.children {
                if is_keyword_line(child, "status", Some(TokenKind::Str)) {
                    let result = (|| {
                        no_children(child)?;
                        let mut c = Cursor::new(child.line);
                        c.next();
                        if footer.status.is_some() {
                            return Err(Diagnostic::error("duplicate 'status'", child.line.span));
                        }
                        footer.status = Some(c.expect_text("a status text")?);
                        footer.status_span = Some(child.line.span);
                        c.expect_end()
                    })();
                    if let Err(diag) = result {
                        diagnostics.push(diag);
                    }
                } else {
                    match parse_widget(child, diagnostics) {
                        Ok(widget) => footer.widgets.push(widget),
                        Err(diag) => diagnostics.push(diag),
                    }
                }
            }
            frame.footer = Some(footer);
        }
    }
    Ok(())
}

pub(crate) fn parse_ribbon_tab(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<RibbonTab> {
    let mut cursor = Cursor::new(node.line);
    if !is_keyword_line(node, "tab", Some(TokenKind::Str)) {
        return Err(cursor.error_here("expected 'tab \"Title\"'"));
    }
    cursor.next();
    let title = cursor.expect_text("a tab title")?;
    let name = part_name(&mut cursor)?;
    let mut groups = Vec::new();
    for child in &node.children {
        let mut c = Cursor::new(child.line);
        if !is_keyword_line(child, "group", Some(TokenKind::Str)) {
            diagnostics.push(c.error_here("expected 'group \"Title\"'"));
            continue;
        }
        c.next();
        let result = c.expect_text("a group title").and_then(|title| {
            let name = part_name(&mut c)?;
            Ok(RibbonGroup {
                title,
                name,
                items: parse_commands(
                    &child.children,
                    &[CommandKind::Button, CommandKind::Separator],
                    diagnostics,
                ),
                span: child.span(),
            })
        });
        match result {
            Ok(group) => groups.push(group),
            Err(diag) => diagnostics.push(diag),
        }
    }
    Ok(RibbonTab {
        title,
        name,
        groups,
        span: node.span(),
    })
}

/// An optional `name x` at the end of a line, as tabs and groups of a ribbon have.
/// The name on a menu's or toolbar's line: `menu name mainMenu`.
/// The rest of a region's line: its name, and its width where it may have
/// one (the menu bar and the toolbar of a frame).
pub(crate) fn region_head(cursor: &mut Cursor, what: &str, wide: bool) -> ParseResult<(Option<String>, Option<u32>)> {
    let (mut name, mut width) = (None, None);
    while let Some(word) = cursor.peek().map(|t| t.text.clone()) {
        let clause = cursor.expect_ident(&format!("a {what} property"))?;
        match word.as_str() {
            "name" => name = Some(cursor.expect_ident("a part name")?.name),
            "width" if wide => width = Some(cursor.expect_number("a width in pixels, as in width 320")?),
            other => {
                let has = if wide { "a name and a width" } else { "a name" };
                return Err(Diagnostic::error(format!("unknown {what} property '{other}'; a {what} has {has}"), clause.span));
            }
        }
    }
    Ok((name, width))
}

pub(crate) fn part_name(cursor: &mut Cursor) -> ParseResult<Option<String>> {
    let name = if cursor.eat_keyword("name") { Some(cursor.expect_ident("a part name")?.name) } else { None };
    cursor.expect_end()?;
    Ok(name)
}

pub(crate) fn parse_commands(
    nodes: &[Node],
    allowed: &[CommandKind],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Command> {
    let mut commands = Vec::new();
    for node in nodes {
        match parse_command(node, allowed, diagnostics) {
            Ok(command) => commands.push(command),
            Err(diag) => diagnostics.push(diag),
        }
    }
    commands
}

pub(crate) fn parse_command(
    node: &Node,
    allowed: &[CommandKind],
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<Command> {
    let mut cursor = Cursor::new(node.line);
    let expected = allowed
        .iter()
        .map(|k| format!("'{}'", frame::command_keyword(*k)))
        .collect::<Vec<_>>()
        .join(" or ");
    let keyword = cursor.expect_ident(&expected)?;
    let kind = match frame::command_by_keyword(&keyword.name) {
        Some(kind) if allowed.contains(&kind) => kind,
        _ => {
            return Err(Diagnostic::error(
                format!("expected {expected}, found '{}'", keyword.name),
                keyword.span,
            ));
        }
    };
    let mut props = frame::new_command(kind, None);
    if cursor.peek_is_text() {
        props.caption = Some(cursor.expect_text("a caption")?);
    }
    let mut seen: Vec<String> = Vec::new();
    while cursor.peek().is_some() {
        let clause = cursor.expect_ident("a property")?;
        if seen.contains(&clause.name) {
            return Err(Diagnostic::error(
                format!("duplicate '{}'", clause.name),
                clause.span,
            ));
        }
        seen.push(clause.name.clone());
        match clause.name.as_str() {
            "icon" => props.icon = Some(cursor.expect_string("an icon name")?),
            // `allow when me.role == Admin`, to the end of the line.
            "allow" => {
                let (written, condition) = allow_clause(&mut cursor)?;
                props.allow = Some(written);
                props.allow_condition = Some(condition);
            }
            "key" => {
                let text = cursor.peek().map(|t| t.span).unwrap_or(clause.span);
                let shortcut = cursor.expect_string("a shortcut, as in key \"Ctrl+N\"")?;
                widgets::check_shortcut(&shortcut).map_err(|m| Diagnostic::error(m, text))?;
                props.shortcut = Some(shortcut);
            }
            "large" => props.large = true,
            "name" => props.name = Some(cursor.expect_ident("a part name")?.name),
            "on" => {
                let event = cursor.expect_ident("an event")?;
                if event.name != "click" {
                    return Err(Diagnostic::error(
                        format!("unknown event '{}', commands have 'click'", event.name),
                        event.span,
                    ));
                }
                if cursor.eat(TokenKind::Colon) {
                    props.handler = Some(cursor.expect_ident("a handler name")?.name);
                    continue;
                }
                cursor.expect(TokenKind::Arrow, "'->' and a target")?;
                props.on_click = Some(click_target(&mut cursor, "a page, screen or command")?);
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
            other => {
                return Err(Diagnostic::error(
                    format!("unknown property '{other}'"),
                    clause.span,
                ));
            }
        }
    }
    frame::validate_command(&props).map_err(|m| Diagnostic::error(m, node.line.span))?;
    let items = if kind == CommandKind::Item {
        parse_commands(
            &node.children,
            &[CommandKind::Item, CommandKind::Separator],
            diagnostics,
        )
    } else {
        no_children(node)?;
        Vec::new()
    };
    Ok(Command {
        props,
        items,
        span: node.span(),
    })
}
