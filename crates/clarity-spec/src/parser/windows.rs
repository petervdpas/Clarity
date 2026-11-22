//! The parser of windows and views (pages): their settings and events,
//! their data and values, and what a click opens.

use super::*;

// ---------------------------------------------------------------------------
// Pages

/// A click target after `->`: a window or a command word, and the strategy
/// after `signin` (`signin password`).
pub(crate) fn click_target(cursor: &mut Cursor, expected: &str) -> ParseResult<String> {
    let target = cursor.expect_ident(expected)?.name;
    if target == frame::SIGNIN_TARGET {
        let strategy = cursor.expect_ident("a strategy of the project file's signin, as in 'signin password'")?;
        return Ok(format!("{target} {}", strategy.name));
    }
    Ok(target)
}

/// An `allow when <condition>` to the end of the line: the condition as
/// written again and as read.
pub(crate) fn allow_clause(cursor: &mut Cursor) -> ParseResult<(String, crate::code::Expr)> {
    if !cursor.eat_keyword("when") {
        return Err(cursor.error_here("expected 'when' and a condition, as in 'allow when me.role == Admin'".to_string()));
    }
    let condition = crate::code::parse::parse_expr(cursor)?;
    Ok((crate::code::print::expr(&condition), condition))
}

/// A condition written in an attribute: `me.role == Admin`.
pub(crate) fn condition_value(text: &str, at: Span) -> ParseResult<crate::code::Expr> {
    let line = value_line(text, at)?;
    let mut cursor = Cursor::new(&line);
    let condition = crate::code::parse::parse_expr(&mut cursor)?;
    cursor.expect_end()?;
    Ok(condition)
}

/// Page settings (`title`, `size`, `style`, `data`, `grid`) as opposed to widget lines.
pub(crate) fn is_page_setting(tokens: &[Token]) -> bool {
    starts_with_keyword(tokens, "title", Some(TokenKind::Str))
        || starts_with_keyword(tokens, "size", Some(TokenKind::Number))
        || starts_with_keyword(tokens, "style", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "data", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "grid", Some(TokenKind::Number))
        || starts_with_keyword(tokens, "on", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "tells", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "allow", Some(TokenKind::Ident))
}

pub(crate) fn parse_page(
    node: &Node,
    mut cursor: Cursor,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<Page> {
    let name = cursor.expect_ident("a page name")?;
    cursor.expect(
        TokenKind::Colon,
        "':' and a page kind, as in 'page X: window'",
    )?;
    let kind_ident = cursor.expect_ident("a page kind ('window', 'dialog', 'view', 'splash', 'toolbox' or 'signin')")?;
    let kind = match kind_ident.name.as_str() {
        "window" => PageKind::Window,
        "dialog" => PageKind::Dialog,
        "view" => PageKind::View,
        "splash" => PageKind::Splash,
        "toolbox" => PageKind::Toolbox,
        "signin" => PageKind::Signin,
        other => {
            return Err(Diagnostic::error(
                format!("unknown page kind '{other}', expected 'window', 'dialog', 'view', 'splash', 'toolbox' or 'signin'"),
                kind_ident.span,
            ));
        }
    };
    // `dialog gives Friend`: what it closes with.
    let gives = if cursor.eat_keyword("gives") { Some(crate::code::parse::parse_type_ref(&mut cursor)?) } else { None };
    cursor.expect_end()?;

    let mut page = Page {
        name,
        kind,
        title: None,
        size: None,
        style: None,
        data: Vec::new(),
        columns: DEFAULT_PAGE_COLUMNS,
        cell: None,
        widgets: Vec::new(),
        frame: Frame::default(),
        handlers: Vec::new(),
        gives,
        tells: Vec::new(),
        allow: None,
        allow_condition: None,
        span: node.span(),
    };
    let mut grid_seen = false;

    for child in &node.children {
        let result = if is_page_setting(&child.line.tokens) {
            parse_page_setting(child, &mut page, &mut grid_seen)
        } else if is_region_line(&child.line.tokens) {
            parse_region(child, &mut page.frame, diagnostics)
        } else {
            parse_widget(child, diagnostics).map(|w| page.widgets.push(w))
        };
        if let Err(diag) = result {
            diagnostics.push(diag);
        }
    }
    Ok(page)
}

/// `on open: window_opened`: one of the owner's own events and its handler.
pub(crate) fn parse_owner_event(
    cursor: &mut Cursor,
    handlers: &mut Vec<Handler>,
    allowed: &[Event],
    refused: impl Fn(&str) -> String,
    owner: &str,
) -> ParseResult<()> {
    let word = cursor.expect_ident("an event")?;
    let event = Event::by_keyword(&word.name)
        .filter(|e| allowed.contains(e))
        .ok_or_else(|| Diagnostic::error(refused(&word.name), word.span))?;
    cursor.expect(TokenKind::Colon, "':' and a handler, as in 'on open: window_opened'")?;
    let handler = cursor.expect_ident("a handler name")?;
    if handlers.iter().any(|h| h.event == event) {
        return Err(Diagnostic::error(
            format!("{owner} already has 'on {}'", event.keyword()),
            word.span,
        ));
    }
    handlers.push(Handler {
        event,
        handler: handler.name,
        args: Vec::new(),
    });
    Ok(())
}

pub(crate) fn parse_page_setting(node: &Node, page: &mut Page, grid_seen: &mut bool) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    let keyword = cursor.expect_ident("a page setting")?;
    match keyword.name.as_str() {
        "title" => {
            if page.title.is_some() {
                return Err(Diagnostic::error("duplicate 'title'", node.line.span));
            }
            page.title = Some(cursor.expect_text("a title string")?);
        }
        "size" => {
            if page.size.is_some() {
                return Err(Diagnostic::error("duplicate 'size'", node.line.span));
            }
            let width = cursor.expect_number("a width in pixels")?;
            if !cursor.eat_keyword("x") {
                return Err(cursor.error_here("expected 'x' and a height, as in 'size 320 x 460'"));
            }
            let height = cursor.expect_number("a height in pixels, as in 'size 320 x 460'")?;
            page.size = Some(WindowSize { width, height });
        }
        // `style splashBack`: a style of the theme for the window itself.
        "style" => {
            if page.style.is_some() {
                return Err(Diagnostic::error("duplicate 'style'", node.line.span));
            }
            page.style = Some(cursor.expect_ident("a style of the project's theme")?.name);
            cursor.expect_end()?;
        }
        "data" => {
            page.data.push(parse_data(&mut cursor, node.line.span)?);
        }
        // `allow when me.role == Admin`: who may open it.
        "allow" => {
            if page.allow.is_some() {
                return Err(Diagnostic::error("duplicate 'allow'", node.line.span));
            }
            let (written, condition) = allow_clause(&mut cursor)?;
            page.allow = Some(written);
            page.allow_condition = Some(condition);
        }
        // What a view tells its window: `tells saved(name: text)`.
        "tells" => {
            let name = cursor.expect_ident("a tell's name, as in tells saved(name: text)")?;
            let params = if cursor.eat(TokenKind::LParen) { crate::code::parse::parse_params(&mut cursor)? } else { Vec::new() };
            page.tells.push(crate::code::PartMessage { name, params, span: node.span() });
        }
        "on" => {
            let owner = page.name.name.clone();
            parse_owner_event(
                &mut cursor,
                &mut page.handlers,
                &WINDOW_EVENTS,
                |e| format!("a window has the events open, close and return, not {e}"),
                &owner,
            )?;
        }
        _ => {
            // grid N columns
            if *grid_seen {
                return Err(Diagnostic::error("duplicate 'grid'", node.line.span));
            }
            *grid_seen = true;
            page.columns = cursor.expect_number("a number of columns")?;
            if !cursor.eat_keyword("columns") {
                return Err(cursor.error_here("expected 'columns', as in 'grid 12 columns'"));
            }
            // `rows 10`: the page's own exact grid, rows of 10 pixels.
            if cursor.eat_keyword("rows") {
                page.cell = Some(cursor.expect_number("a row height in pixels, as in 'grid 56 columns rows 10'")?);
            }
        }
    }
    cursor.expect_end()
}

/// `name: Entity`, `name: list<Entity>` or `name: type, default value`,
/// after the `data` keyword. A name is a record here; once every file is
/// read, one that names an enum becomes a plain value (`resolve_data`).
pub(crate) fn parse_data(cursor: &mut Cursor, span: Span) -> ParseResult<DataDecl> {
    let name = cursor.expect_ident("a data name")?;
    cursor.expect(
        TokenKind::Colon,
        "':' and an entity or a type, as in 'data customer: Customer'",
    )?;
    let ty = crate::code::parse::parse_type_ref(cursor)?;
    let default = if cursor.eat(TokenKind::Comma) {
        if !cursor.eat_keyword("default") {
            return Err(cursor.error_here("expected 'default' and a value, as in 'data fresh: bool, default true'"));
        }
        Some(parse_literal(cursor)?)
    } else {
        None
    };
    data_decl(name, ty, default, span)
}

/// A window's or app's data of `name`, of type `ty`, starting at `default`:
/// a record, a list of records or a plain value, by the type.
pub(crate) fn data_decl(name: Ident, ty: crate::code::TypeRef, default: Option<Literal>, span: Span) -> ParseResult<DataDecl> {
    use crate::code::TypeRefKind;
    use crate::model::DataKind;
    let of = match ty.kind {
        // A list of values is a plain value of the window, starting empty (manual/parts.data.md).
        TypeRefKind::List { ref item } if crate::code::parse::is_type_word(item) => {
            if crate::model::ListItem::of_word(item).is_none() {
                return Err(Diagnostic::error("a list of values holds text, integer, decimal or date, as in 'list<decimal>'; a list of records 'list<Order>'", ty.span));
            }
            if default.is_some() {
                return Err(Diagnostic::error(format!("'{}' is a list, which starts empty; it has no default", name.name), ty.span));
            }
            DataKind::Value { ty, default: None }
        }
        TypeRefKind::List { .. } if default.is_some() => {
            return Err(Diagnostic::error(
                format!("'{}' is a list, which starts with every record; only a plain value has a default", name.name),
                ty.span,
            ));
        }
        TypeRefKind::List { item } => {
            // The entity's name sits between the angle brackets.
            let start = ty.span.end - 1 - item.len() as u32;
            let col = ty.span.col + (start - ty.span.start);
            DataKind::List { entity: Ident { name: item, span: Span { start, end: ty.span.end - 1, line: ty.span.line, col } } }
        }
        TypeRefKind::File => {
            return Err(Diagnostic::error(
                "a window holds records, lists and plain values; a file belongs to a record",
                ty.span,
            ));
        }
        TypeRefKind::Bytes => {
            return Err(Diagnostic::error("bytes are no window data: the screen has nothing to show of them, so code keeps them while it works", ty.span));
        }
        TypeRefKind::Named { name: entity } if default.is_none() => DataKind::Record { entity: Ident { name: entity, span: ty.span } },
        _ => DataKind::Value { ty, default },
    };
    Ok(DataDecl { name, of, made: false, span })
}

// ---------------------------------------------------------------------------
// Values written in a design file's attributes (manual/design.windows.md): read by the
// rules Clarity's own lines are read by, each where it is in the markup.

/// `text`, an attribute's value at `at`, as one line of Clarity there: each
/// token where it is in the markup.
pub(crate) fn value_line(text: &str, at: Span) -> ParseResult<Line> {
    let (lines, diagnostics) = crate::lexer::lex(text);
    let place = |span: Span| {
        let start = (at.start + span.start).min(at.end);
        Span { start, end: (at.start + span.end).min(at.end), line: at.line, col: at.col + span.start }
    };
    if let Some(d) = diagnostics.into_iter().next() {
        return Err(Diagnostic::error(d.message, place(d.span)));
    }
    let mut lines = lines.into_iter();
    let mut line = lines.next().unwrap_or(Line { indent: 0, tokens: Vec::new(), span: Span::default() });
    if lines.next().is_some() {
        return Err(Diagnostic::error("a value is written on one line", at));
    }
    for token in &mut line.tokens {
        token.span = place(token.span);
    }
    line.span = at;
    Ok(line)
}

/// A type written in an attribute: `Customer`, `list<Order>`, `decimal`.
pub(crate) fn type_value(text: &str, at: Span) -> ParseResult<crate::code::TypeRef> {
    let line = value_line(text, at)?;
    let mut cursor = Cursor::new(&line);
    let ty = crate::code::parse::parse_type_ref(&mut cursor)?;
    cursor.expect_end()?;
    Ok(ty)
}

/// A value written in an attribute, as Clarity writes one: `80`, `true`, `"text"`, `Business`.
pub(crate) fn literal_value(text: &str, at: Span) -> ParseResult<Literal> {
    let line = value_line(text, at)?;
    let mut cursor = Cursor::new(&line);
    let value = parse_literal(&mut cursor)?;
    cursor.expect_end()?;
    Ok(value)
}

/// The values a handler runs with, as in `digit(7, true)`: what is between the brackets.
pub(crate) fn literals_value(text: &str, at: Span) -> ParseResult<Vec<Literal>> {
    let line = value_line(text, at)?;
    let mut cursor = Cursor::new(&line);
    let mut out = Vec::new();
    while cursor.peek().is_some() {
        out.push(parse_literal(&mut cursor)?);
        if cursor.peek().is_some() {
            cursor.expect(TokenKind::Comma, "',' between values")?;
        }
    }
    Ok(out)
}

/// A window's or app's data: `<Data name="customer" type="Customer" default="...">`.
pub(crate) fn data_value(name: Ident, ty: (&str, Span), default: Option<(&str, Span)>, span: Span) -> ParseResult<DataDecl> {
    let ty = type_value(ty.0, ty.1)?;
    let default = default.map(|(text, at)| literal_value(text, at)).transpose()?;
    data_decl(name, ty, default, span)
}

/// What a view tells its window: `<Tells message="saved(name: text)" />`.
pub(crate) fn tells_value(text: &str, at: Span, span: Span) -> ParseResult<crate::code::PartMessage> {
    let line = value_line(text, at)?;
    let mut cursor = Cursor::new(&line);
    let name = cursor.expect_ident("a tell's name, as in saved(name: text)")?;
    let params = if cursor.eat(TokenKind::LParen) { crate::code::parse::parse_params(&mut cursor)? } else { Vec::new() };
    cursor.expect_end()?;
    Ok(crate::code::PartMessage { name, params, span })
}
