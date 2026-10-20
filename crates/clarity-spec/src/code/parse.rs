//! Parses `code` blocks: handlers, their statements (one per line, blocks
//! by indentation) and expressions (precedence climbing on one line).

use super::{EngineState, 
    Arg, BinaryOp, Branch, CodeBlock, DataEvent, EngineDecl, EngineFunction, EventDecl, Expr, TextPart, ExprKind, FunctionDecl, FunctionKind, HandlerDecl, MessageLevel, OrderKey, Param,
    Stmt, StmtKind, TypeRef, TypeRefKind, UnaryOp,
};
use crate::diagnostic::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::model::Ident;
use crate::parser::{Cursor, Node, ParseResult, first_word, no_children};
use crate::span::Span;

pub(crate) fn parse_code(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> CodeBlock {
    let mut handlers = Vec::new();
    let mut events = Vec::new();
    for child in &node.children {
        let parsed = match first_word(child) {
            Some("before" | "after") => parse_event(child, diagnostics).map(|e| events.push(e)),
            _ => parse_handler(child, diagnostics).map(|h| handlers.push(h)),
        };
        if let Err(diag) = parsed {
            diagnostics.push(diag);
        }
    }
    CodeBlock {
        name,
        handlers,
        events,
        span: node.span(),
    }
}

/// `before save`, `after save`, `before delete` or `after delete`, with
/// its statements.
fn parse_event(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<EventDecl> {
    let mut cursor = Cursor::new(node.line);
    let when = cursor.expect_ident("'before' or 'after'")?;
    let what = cursor.expect_ident("'save' or 'delete'")?;
    cursor.expect_end()?;
    let text = format!("{} {}", when.name, what.name);
    let Some(event) = DataEvent::ALL.into_iter().find(|e| e.text() == text) else {
        return Err(Diagnostic::error(
            format!("unknown event '{text}'; the events are before save, after save, before delete and after delete"),
            node.line.span,
        ));
    };
    Ok(EventDecl {
        event,
        body: indented_body(node, event.text(), diagnostics)?,
        head: node.line.span,
        span: node.span(),
    })
}

/// `function name(a: T, b: T) -> T` or `action name(...)`, then its
/// `require`, `ensure` and `effects` lines, then the statements.
/// `engine Name from "file.wasm"` with its functions below it, each a
/// signature: `function total(lines: list<Line>) -> money`.
pub(crate) fn parse_engine(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<EngineDecl> {
    let mut cursor = Cursor::new(node.line);
    cursor.next();
    let name = cursor.expect_ident("an engine name")?;
    if !cursor.eat_keyword("from") {
        return Err(cursor.error_here("expected 'from' and its file, as in 'engine Pricing from \"pricing.wasm\"'"));
    }
    let file = cursor.expect_string("the engine's file in the engines folder")?;
    cursor.expect_end()?;
    let mut functions = Vec::new();
    let (mut seconds, mut megabytes, mut limited) = (None, None, false);
    let mut uses = Vec::new();
    let (mut state, mut stated, mut opens) = (EngineState::None, false, false);
    for child in &node.children {
        let mut line = Cursor::new(child.line);
        if line.eat_keyword("state") {
            let parsed = (|| {
                if stated {
                    return Err(line.error_here("an engine has one state line, as in 'state per app'"));
                }
                stated = true;
                if !line.eat_keyword("per") {
                    return Err(line.error_here("expected 'per app' or 'per window', as in 'state per window'"));
                }
                state = if line.eat_keyword("app") {
                    EngineState::App
                } else if line.eat_keyword("window") {
                    EngineState::Window
                } else {
                    return Err(line.error_here("expected 'per app' or 'per window', as in 'state per window'"));
                };
                line.expect_end()?;
                no_children(child)
            })();
            if let Err(diag) = parsed {
                diagnostics.push(diag);
            }
            continue;
        }
        if line.eat_keyword("on") {
            let parsed = (|| {
                if !line.eat_keyword("open") {
                    return Err(line.error_here("an engine has on open only: it cannot save or show anything at close, so a window's own 'on close' asks it what to keep"));
                }
                if opens {
                    return Err(line.error_here("an engine has one on open line"));
                }
                opens = true;
                line.expect_end()?;
                no_children(child)
            })();
            if let Err(diag) = parsed {
                diagnostics.push(diag);
            }
            continue;
        }
        if line.eat_keyword("uses") {
            let parsed = (|| {
                uses.extend(line.ident_list("the services it uses, as in 'uses clock, random, log'")?);
                line.expect_end()?;
                no_children(child)
            })();
            if let Err(diag) = parsed {
                diagnostics.push(diag);
            }
            continue;
        }
        if line.eat_keyword("limit") {
            let parsed = (|| {
                if limited {
                    return Err(line.error_here("an engine has one limit line, as in 'limit 2 seconds, 64 MB'"));
                }
                limited = true;
                loop {
                    let n = line.expect_number("a number of seconds or megabytes, as in 'limit 2 seconds, 64 MB'")?;
                    if line.eat_keyword("seconds") || line.eat_keyword("second") {
                        seconds = Some(n);
                    } else if line.eat_keyword("MB") {
                        megabytes = Some(n);
                    } else {
                        return Err(line.error_here("expected 'seconds' or 'MB', as in 'limit 2 seconds, 64 MB'"));
                    }
                    if !line.eat(TokenKind::Comma) {
                        break;
                    }
                }
                line.expect_end()?;
                no_children(child)
            })();
            if let Err(diag) = parsed {
                diagnostics.push(diag);
            }
            continue;
        }
        let parsed = (|| {
            if !line.eat_keyword("function") {
                return Err(line.error_here("expected 'function' and its signature, as in 'function total(lines: list<Line>) -> money'"));
            }
            let name = line.expect_ident("a function name")?;
            line.expect(TokenKind::LParen, "'(' and the parameters")?;
            let params = parse_params(&mut line)?;
            let returns = if line.eat(TokenKind::Arrow) { Some(parse_type_ref(&mut line)?) } else { None };
            line.expect_end()?;
            no_children(child)?;
            Ok(EngineFunction { name, params, returns, span: child.span() })
        })();
        match parsed {
            Ok(function) => functions.push(function),
            Err(diag) => diagnostics.push(diag),
        }
    }
    Ok(EngineDecl { name, file, uses, state, opens, seconds, megabytes, functions, span: node.span() })
}

pub(crate) fn parse_function(node: &Node, kind: FunctionKind, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<FunctionDecl> {
    let mut cursor = Cursor::new(node.line);
    cursor.next();
    let name = cursor.expect_ident(match kind {
        FunctionKind::Function => "a function name",
        FunctionKind::Action => "an action name",
        FunctionKind::Event => "an event",
    })?;
    cursor.expect(TokenKind::LParen, "'(' and the parameters")?;
    let params = parse_params(&mut cursor)?;
    let returns = if cursor.eat(TokenKind::Arrow) {
        Some(parse_type_ref(&mut cursor)?)
    } else {
        None
    };
    cursor.expect_end()?;

    let mut decl = FunctionDecl {
        kind,
        name,
        params,
        returns,
        requires: Vec::new(),
        ensures: Vec::new(),
        effects: Vec::new(),
        body: Vec::new(),
        span: node.span(),
    };
    let clauses = node
        .children
        .iter()
        .take_while(|c| matches!(first_word(c), Some("require" | "ensure" | "effects")))
        .count();
    for child in &node.children[..clauses] {
        if let Err(diag) = parse_clause(child, &mut decl) {
            diagnostics.push(diag);
        }
    }
    for child in &node.children[clauses..] {
        if let Some(word @ ("require" | "ensure" | "effects")) = first_word(child) {
            diagnostics.push(Diagnostic::error(
                format!("'{word}' goes before the statements"),
                child.line.span,
            ));
        }
    }
    decl.body = parse_block(&node.children[clauses..], diagnostics);
    Ok(decl)
}

fn parse_clause(node: &Node, decl: &mut FunctionDecl) -> ParseResult<()> {
    let mut cursor = Cursor::new(node.line);
    let word = cursor.next().map(|t| t.text.as_str());
    match word {
        Some("require") => decl.requires.push(parse_expr(&mut cursor)?),
        Some("ensure") => decl.ensures.push(parse_expr(&mut cursor)?),
        _ => loop {
            // `db.write`: names joined by dots.
            let first = cursor.expect_ident("an effect, like db.write")?;
            let mut effect = first.clone();
            while cursor.eat(TokenKind::Dot) {
                let part = cursor.expect_ident("a name after '.'")?;
                effect.name = format!("{}.{}", effect.name, part.name);
                effect.span.end = part.span.end;
            }
            decl.effects.push(effect);
            if !cursor.eat(TokenKind::Comma) {
                break;
            }
        },
    }
    cursor.expect_end()?;
    no_children(node)
}

/// Whether `word` is a type of the language rather than a name:
/// `decimal`, not `Order`.
pub(crate) fn is_type_word(word: &str) -> bool {
    matches!(word, "text" | "integer" | "decimal" | "money" | "bool" | "date" | "datetime" | "duration" | "file")
}

/// `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`,
/// `list<Entity>`, or the name of an entity or enum.
pub(crate) fn parse_type_ref(cursor: &mut Cursor) -> ParseResult<TypeRef> {
    let ident = cursor.expect_ident("a type")?;
    let mut span = ident.span;
    let kind = match ident.name.as_str() {
        "text" => TypeRefKind::Text,
        "integer" => TypeRefKind::Integer,
        "decimal" => TypeRefKind::Decimal,
        "money" => TypeRefKind::Money,
        "bool" => TypeRefKind::Bool,
        "date" => TypeRefKind::Date,
        "datetime" => TypeRefKind::DateTime,
        "duration" => TypeRefKind::Duration,
        "file" => TypeRefKind::File,
        "list" => {
            cursor.expect(TokenKind::Lt, "'<' after 'list'")?;
            let item = cursor.expect_ident("an entity name")?;
            let close = cursor.expect(TokenKind::Gt, "'>'")?;
            span.end = close.span.end;
            TypeRefKind::List { item: item.name }
        }
        _ => TypeRefKind::Named { name: ident.name },
    };
    Ok(TypeRef { kind, span })
}

/// The parameters after `(`, to and with `)`: `a: T, b: T)`.
fn parse_params(cursor: &mut Cursor) -> ParseResult<Vec<Param>> {
    let mut params = Vec::new();
    if cursor.eat(TokenKind::RParen) {
        return Ok(params);
    }
    loop {
        let name = cursor.expect_ident("a parameter name")?;
        cursor.expect(TokenKind::Colon, "':' and a type")?;
        params.push(Param { name, ty: parse_type_ref(cursor)? });
        if cursor.eat(TokenKind::RParen) {
            return Ok(params);
        }
        cursor.expect(TokenKind::Comma, "',' or ')'")?;
    }
}

/// `handler name`, or with the values a click gives: `handler digit(d: integer)`.
fn parse_handler(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<HandlerDecl> {
    let mut cursor = Cursor::new(node.line);
    if !cursor.eat_keyword("handler") {
        return Err(cursor.error_here("expected 'handler <name>'"));
    }
    let name = cursor.expect_ident("a handler name")?;
    let params = if cursor.eat(TokenKind::LParen) { parse_params(&mut cursor)? } else { Vec::new() };
    cursor.expect_end()?;
    Ok(HandlerDecl {
        name,
        params,
        body: parse_block(&node.children, diagnostics),
        span: node.span(),
    })
}

fn parse_block(nodes: &[Node], diagnostics: &mut Vec<Diagnostic>) -> Vec<Stmt> {
    let mut body: Vec<Stmt> = Vec::new();
    for node in nodes {
        if first_word(node) == Some("else") {
            if let Err(diag) = attach_else(node, body.last_mut(), diagnostics) {
                diagnostics.push(diag);
            }
            continue;
        }
        match parse_stmt(node, diagnostics) {
            Ok(stmt) => body.push(stmt),
            Err(diag) => diagnostics.push(diag),
        }
    }
    body
}

/// Adds an `else if` branch or the `else` block to the `if` just before it.
fn attach_else(node: &Node, previous: Option<&mut Stmt>, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<()> {
    let open_if = match previous {
        Some(Stmt {
            kind: StmtKind::If { branches, otherwise },
            span,
        }) if otherwise.is_none() => Some((branches, otherwise, span)),
        _ => None,
    };
    let Some((branches, otherwise, span)) = open_if else {
        return Err(Diagnostic::error("'else' without 'if'", node.line.span));
    };
    let mut cursor = Cursor::new(node.line);
    cursor.next();
    let body = indented_body(node, "else", diagnostics)?;
    if cursor.eat_keyword("if") {
        let condition = parse_expr(&mut cursor)?;
        cursor.expect_end()?;
        branches.push(Branch { condition, body });
    } else {
        cursor.expect_end()?;
        *otherwise = Some(body);
    }
    span.end = node.span().end;
    Ok(())
}

fn indented_body(node: &Node, keyword: &str, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<Vec<Stmt>> {
    if node.children.is_empty() {
        return Err(Diagnostic::error(format!("'{keyword}' needs an indented block"), node.line.span));
    }
    Ok(parse_block(&node.children, diagnostics))
}

const STATEMENT_WORDS: [&str; 12] = ["let", "if", "for", "return", "message", "open", "close", "save", "delete", "reload", "refuse", "write"];

fn parse_stmt(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<Stmt> {
    let tokens = &node.line.tokens;
    // `save.enabled = ...` sets a part called save; `save customer` stores.
    // `let`, `if` and `for` are always statements.
    let keyword = first_word(node)
        .filter(|w| STATEMENT_WORDS.contains(w))
        .filter(|w| matches!(*w, "let" | "if" | "for") || !matches!(tokens.get(1).map(|t| t.kind), Some(TokenKind::Dot | TokenKind::Assign)));
    let mut cursor = Cursor::new(node.line);
    let span = node.line.span;
    let kind = match keyword {
        Some("if") => {
            cursor.next();
            let condition = parse_expr(&mut cursor)?;
            cursor.expect_end()?;
            let body = indented_body(node, "if", diagnostics)?;
            return Ok(Stmt {
                kind: StmtKind::If {
                    branches: vec![Branch { condition, body }],
                    otherwise: None,
                },
                span: node.span(),
            });
        }
        Some("let") => {
            cursor.next();
            let name = cursor.expect_ident("a name")?;
            let ty = if cursor.eat(TokenKind::Colon) {
                Some(parse_type_ref(&mut cursor)?)
            } else {
                None
            };
            cursor.expect(TokenKind::Assign, "'='")?;
            let value = parse_expr(&mut cursor)?;
            StmtKind::Let { name, ty, value }
        }
        Some("return") => {
            cursor.next();
            let value = if cursor.peek().is_some() {
                Some(parse_expr(&mut cursor)?)
            } else {
                None
            };
            StmtKind::Return { value }
        }
        Some("for") => {
            cursor.next();
            let var = cursor.expect_ident("a name for each item")?;
            if !cursor.eat_keyword("in") {
                return Err(cursor.error_here("expected 'in' and a list"));
            }
            let list = parse_expr(&mut cursor)?;
            cursor.expect_end()?;
            let body = indented_body(node, "for", diagnostics)?;
            return Ok(Stmt {
                kind: StmtKind::For { var, list, body },
                span: node.span(),
            });
        }
        Some("close") => {
            cursor.next();
            StmtKind::Close
        }
        Some("message") => {
            cursor.next();
            // A level word counts only when a value follows it.
            let level = match (cursor.peek(), node.line.tokens.get(2)) {
                (Some(t), Some(_)) if t.kind == TokenKind::Ident => MessageLevel::by_keyword(&t.text),
                _ => None,
            };
            if level.is_some() {
                cursor.next();
            }
            let text = parse_expr(&mut cursor)?;
            StmtKind::Message {
                level: level.unwrap_or(MessageLevel::Info),
                text,
            }
        }
        Some("open") => {
            cursor.next();
            let window = cursor.expect_ident("a window name")?;
            let mut with = Vec::new();
            if cursor.eat_keyword("with") {
                loop {
                    let name = cursor.expect_ident("the name of the window's data")?;
                    cursor.expect(TokenKind::Assign, "'='")?;
                    let value = parse_expr(&mut cursor)?;
                    with.push(Arg { name, value });
                    if !cursor.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            StmtKind::Open { window, with }
        }
        Some("save") => {
            cursor.next();
            StmtKind::Save {
                value: parse_expr(&mut cursor)?,
            }
        }
        Some("reload") => {
            cursor.next();
            StmtKind::Reload {
                data: cursor.expect_ident("the name of the window's data")?,
            }
        }
        Some("refuse") => {
            cursor.next();
            StmtKind::Refuse {
                text: parse_expr(&mut cursor)?,
            }
        }
        Some("write") => {
            cursor.next();
            let text = parse_expr(&mut cursor)?;
            if !cursor.eat_keyword("to") {
                return Err(cursor.error_here("expected 'to', a file store and a path, as in 'write lines to photos \"list.txt\"'"));
            }
            let store = cursor.expect_ident("a file store")?;
            let path = parse_expr(&mut cursor)?;
            StmtKind::Write { text, store, path }
        }
        Some("delete") => {
            cursor.next();
            StmtKind::Delete {
                value: parse_expr(&mut cursor)?,
            }
        }
        // `dial.line(...)`: a shape on a canvas.
        _ if cursor.peek_at(1).is_some_and(|t| t.kind == TokenKind::Dot)
            && cursor.peek_at(2).is_some_and(|t| t.kind == TokenKind::Ident)
            && cursor.peek_at(3).is_some_and(|t| t.kind == TokenKind::LParen) =>
        {
            let canvas = cursor.expect_ident("a canvas")?;
            cursor.expect(TokenKind::Dot, "'.'")?;
            let shape = cursor.expect_ident("a shape")?;
            cursor.expect(TokenKind::LParen, "'('")?;
            let mut args = Vec::new();
            if cursor.peek_kind() != Some(TokenKind::RParen) {
                args.push(parse_expr(&mut cursor)?);
                while cursor.eat(TokenKind::Comma) {
                    args.push(parse_expr(&mut cursor)?);
                }
            }
            cursor.expect(TokenKind::RParen, "')'")?;
            cursor.expect_end()?;
            no_children(node)?;
            StmtKind::Draw { canvas, shape, args }
        }
        _ => {
            let target = parse_expr(&mut cursor)?;
            if matches!(target.kind, ExprKind::Call { .. }) && cursor.peek().is_none() {
                no_children(node)?;
                return Ok(Stmt {
                    kind: StmtKind::Call { call: target },
                    span,
                });
            }
            if !cursor.eat(TokenKind::Assign) {
                return Err(Diagnostic::error("expected a statement, like 'x = value'", span));
            }
            if !is_place(&target) {
                return Err(Diagnostic::error(
                    "only a local, a data field or a part property can be set",
                    target.span,
                ));
            }
            let value = parse_expr(&mut cursor)?;
            StmtKind::Assign { target, value }
        }
    };
    cursor.expect_end()?;
    no_children(node)?;
    Ok(Stmt { kind, span })
}

/// A name or a member path: what an assignment can set.
fn is_place(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Name { .. } => true,
        ExprKind::Member { object, .. } => is_place(object),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Expressions, loosest first: or, and, not, comparison and in, + -, * /,
// unary minus, member access.

pub(crate) fn parse_expr(cursor: &mut Cursor) -> ParseResult<Expr> {
    parse_or(cursor)
}

fn join(a: Span, b: Span) -> Span {
    Span { end: b.end, ..a }
}

fn binary(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    Expr {
        span: join(left.span, right.span),
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

fn parse_or(cursor: &mut Cursor) -> ParseResult<Expr> {
    let mut left = parse_and(cursor)?;
    while cursor.eat_keyword("or") {
        let right = parse_and(cursor)?;
        left = binary(BinaryOp::Or, left, right);
    }
    Ok(left)
}

fn parse_and(cursor: &mut Cursor) -> ParseResult<Expr> {
    let mut left = parse_not(cursor)?;
    while cursor.eat_keyword("and") {
        let right = parse_not(cursor)?;
        left = binary(BinaryOp::And, left, right);
    }
    Ok(left)
}

fn parse_not(cursor: &mut Cursor) -> ParseResult<Expr> {
    let start = cursor.peek().map(|t| t.span);
    if cursor.eat_keyword("not") {
        let operand = parse_not(cursor)?;
        return Ok(Expr {
            span: join(start.unwrap_or(operand.span), operand.span),
            kind: ExprKind::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            },
        });
    }
    parse_comparison(cursor)
}

fn comparison_op(kind: TokenKind) -> Option<BinaryOp> {
    match kind {
        TokenKind::EqEq => Some(BinaryOp::Eq),
        TokenKind::NotEq => Some(BinaryOp::NotEq),
        TokenKind::Lt => Some(BinaryOp::Lt),
        TokenKind::LtEq => Some(BinaryOp::LtEq),
        TokenKind::Gt => Some(BinaryOp::Gt),
        TokenKind::GtEq => Some(BinaryOp::GtEq),
        _ => None,
    }
}

fn parse_comparison(cursor: &mut Cursor) -> ParseResult<Expr> {
    let left = parse_additive(cursor)?;
    if cursor.eat_keyword("contains") {
        let right = parse_additive(cursor)?;
        return Ok(binary(BinaryOp::Contains, left, right));
    }
    if let Some(op) = cursor.peek_kind().and_then(comparison_op) {
        cursor.next();
        let right = parse_additive(cursor)?;
        return Ok(binary(op, left, right));
    }
    if cursor.eat_keyword("in") {
        cursor.expect(TokenKind::LBracket, "'['")?;
        let mut options = vec![parse_expr(cursor)?];
        while cursor.eat(TokenKind::Comma) {
            options.push(parse_expr(cursor)?);
        }
        let close = cursor.expect(TokenKind::RBracket, "']'")?;
        return Ok(Expr {
            span: join(left.span, close.span),
            kind: ExprKind::In {
                value: Box::new(left),
                options,
            },
        });
    }
    Ok(left)
}

fn parse_additive(cursor: &mut Cursor) -> ParseResult<Expr> {
    let mut left = parse_multiplicative(cursor)?;
    loop {
        match cursor.peek() {
            Some(t) if t.kind == TokenKind::Plus => {
                cursor.next();
                let right = parse_multiplicative(cursor)?;
                left = binary(BinaryOp::Add, left, right);
            }
            Some(t) if t.kind == TokenKind::Minus => {
                cursor.next();
                let right = parse_multiplicative(cursor)?;
                left = binary(BinaryOp::Sub, left, right);
            }
            // `5 -1` lexes as 5 and the number -1: after a value it subtracts.
            Some(t) if t.kind == TokenKind::Number && t.text.starts_with('-') => {
                cursor.next();
                let first = number_without_sign(t);
                let right = continue_multiplicative(cursor, first)?;
                left = binary(BinaryOp::Sub, left, right);
            }
            _ => return Ok(left),
        }
    }
}

fn number_without_sign(token: &Token) -> Expr {
    let span = Span {
        start: token.span.start + 1,
        col: token.span.col + 1,
        ..token.span
    };
    Expr {
        kind: ExprKind::Number {
            value: token.text[1..].to_string(),
        },
        span,
    }
}

fn parse_multiplicative(cursor: &mut Cursor) -> ParseResult<Expr> {
    let first = parse_unary(cursor)?;
    continue_multiplicative(cursor, first)
}

fn continue_multiplicative(cursor: &mut Cursor, mut left: Expr) -> ParseResult<Expr> {
    loop {
        let op = match cursor.peek_kind() {
            Some(TokenKind::Star) => BinaryOp::Mul,
            Some(TokenKind::Slash) => BinaryOp::Div,
            _ => return Ok(left),
        };
        cursor.next();
        let right = parse_unary(cursor)?;
        left = binary(op, left, right);
    }
}

fn parse_unary(cursor: &mut Cursor) -> ParseResult<Expr> {
    if let Some(minus) = cursor.peek().filter(|t| t.kind == TokenKind::Minus) {
        cursor.next();
        let operand = parse_unary(cursor)?;
        return Ok(Expr {
            span: join(minus.span, operand.span),
            kind: ExprKind::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
            },
        });
    }
    parse_postfix(cursor)
}

fn parse_postfix(cursor: &mut Cursor) -> ParseResult<Expr> {
    // `Pricing.total(lines)`: a function of an engine.
    if cursor.peek().is_some_and(|t| t.kind == TokenKind::Ident && !RESERVED.contains(&t.text.as_str()))
        && cursor.peek_at(1).is_some_and(|t| t.kind == TokenKind::Dot)
        && cursor.peek_at(2).is_some_and(|t| t.kind == TokenKind::Ident)
        && cursor.peek_at(3).is_some_and(|t| t.kind == TokenKind::LParen)
    {
        let engine = cursor.expect_ident("an engine")?;
        cursor.expect(TokenKind::Dot, "'.'")?;
        let function = cursor.expect_ident("a function of the engine")?;
        cursor.expect(TokenKind::LParen, "'('")?;
        let mut args = Vec::new();
        if cursor.peek_kind() != Some(TokenKind::RParen) {
            args.push(parse_expr(cursor)?);
            while cursor.eat(TokenKind::Comma) {
                args.push(parse_expr(cursor)?);
            }
        }
        let close = cursor.expect(TokenKind::RParen, "')'")?;
        return Ok(Expr { span: join(engine.span, close.span), kind: ExprKind::EngineCall { engine, function, args } });
    }
    let mut expr = parse_primary(cursor)?;
    while cursor.eat(TokenKind::Dot) {
        let member = cursor.expect_ident("a name after '.'")?;
        expr = Expr {
            span: join(expr.span, member.span),
            kind: ExprKind::Member {
                object: Box::new(expr),
                member,
            },
        };
    }
    Ok(expr)
}

const RESERVED: [&str; 5] = ["and", "or", "not", "in", "contains"];

/// The empty value of a date, a date and time or a reference.
pub const NONE: &str = "none";

fn parse_primary(cursor: &mut Cursor) -> ParseResult<Expr> {
    let Some(token) = cursor.peek() else {
        return Err(cursor.error_here("expected a value"));
    };
    let kind = match token.kind {
        TokenKind::Number => ExprKind::Number {
            value: token.text.clone(),
        },
        TokenKind::Str => text_kind(token)?,
        TokenKind::Ident if token.text == "true" || token.text == "false" => ExprKind::Bool {
            value: token.text == "true",
        },
        TokenKind::Ident if token.text == NONE => ExprKind::Empty,
        TokenKind::Ident if token.text == "find" && cursor.peek_at(2).is_some_and(|t| t.text == "in") => {
            return parse_find(cursor);
        }
        TokenKind::Ident if token.text == "files" && cursor.peek_at(2).is_some_and(|t| t.text == "in") => {
            return parse_files(cursor);
        }
        TokenKind::Ident
            if token.text == "read" && cursor.peek_at(1).is_some_and(|t| t.text == "text") && cursor.peek_at(2).is_some_and(|t| t.text == "of") =>
        {
            cursor.next();
            cursor.next();
            cursor.next();
            let file = parse_postfix(cursor)?;
            return Ok(Expr { span: join(token.span, file.span), kind: ExprKind::ReadText { file: Box::new(file) } });
        }
        TokenKind::Ident if token.text == "new" && cursor.peek_at(1).is_some_and(|t| t.kind == TokenKind::Ident) => {
            cursor.next();
            let entity = cursor.expect_ident("an entity")?;
            return Ok(Expr { span: join(token.span, entity.span), kind: ExprKind::New { entity } });
        }
        TokenKind::Ident if !RESERVED.contains(&token.text.as_str()) => {
            cursor.next();
            if cursor.eat(TokenKind::LParen) {
                let mut args = Vec::new();
                if cursor.peek_kind() != Some(TokenKind::RParen) {
                    args.push(parse_expr(cursor)?);
                    while cursor.eat(TokenKind::Comma) {
                        args.push(parse_expr(cursor)?);
                    }
                }
                let close = cursor.expect(TokenKind::RParen, "')'")?;
                return Ok(Expr {
                    span: join(token.span, close.span),
                    kind: ExprKind::Call {
                        function: Ident {
                            name: token.text.clone(),
                            span: token.span,
                        },
                        args,
                    },
                });
            }
            return Ok(Expr {
                kind: ExprKind::Name {
                    name: token.text.clone(),
                },
                span: token.span,
            });
        }
        TokenKind::LParen => {
            cursor.next();
            let inner = parse_expr(cursor)?;
            let close = cursor.expect(TokenKind::RParen, "')'")?;
            return Ok(Expr {
                span: join(token.span, close.span),
                kind: inner.kind,
            });
        }
        _ => return Err(cursor.error_here("expected a value")),
    };
    cursor.next();
    Ok(Expr {
        kind,
        span: token.span,
    })
}

/// `files f in photos [where condition] [order by f.field [desc], ...]`
fn parse_files(cursor: &mut Cursor) -> ParseResult<Expr> {
    let start = cursor.next().map(|t| t.span).unwrap_or_else(|| cursor.end_span());
    let var = cursor.expect_ident("a name for each file")?;
    if !cursor.eat_keyword("in") {
        return Err(cursor.error_here("expected 'in' and a file store"));
    }
    let store = cursor.expect_ident("a file store")?;
    let mut end = store.span;
    let filter = if cursor.eat_keyword("where") {
        let condition = parse_expr(cursor)?;
        end = condition.span;
        Some(Box::new(condition))
    } else {
        None
    };
    let order = parse_order(cursor, &mut end)?;
    Ok(Expr {
        span: join(start, end),
        kind: ExprKind::Files { var, store, filter, order },
    })
}

/// `order by c.field [desc], ...`, if it is there; `end` moves past it.
fn parse_order(cursor: &mut Cursor, end: &mut Span) -> ParseResult<Vec<OrderKey>> {
    let mut order = Vec::new();
    if cursor.eat_keyword("order") {
        if !cursor.eat_keyword("by") {
            return Err(cursor.error_here("expected 'by', as in 'order by c.name'"));
        }
        loop {
            let key_var = cursor.expect_ident("a field, as in 'c.name'")?;
            cursor.expect(TokenKind::Dot, "'.' and a field, as in 'c.name'")?;
            let mut field = cursor.expect_ident("a field")?;
            // Through references: `a.artist.name`.
            let mut through = Vec::new();
            while cursor.eat(TokenKind::Dot) {
                let next = cursor.expect_ident("a field")?;
                through.push(std::mem::replace(&mut field, next));
            }
            *end = field.span;
            let descending = cursor.peek().is_some_and(|t| t.text == "desc");
            if descending {
                *end = cursor.next().map(|t| t.span).unwrap_or(*end);
            }
            order.push(OrderKey { var: key_var, through, field, descending });
            if !cursor.eat(TokenKind::Comma) {
                break;
            }
        }
    }
    Ok(order)
}

/// `find c in Customer [where condition] [order by c.field [desc], ...]`
fn parse_find(cursor: &mut Cursor) -> ParseResult<Expr> {
    let start = cursor.next().map(|t| t.span).unwrap_or_else(|| cursor.end_span());
    let var = cursor.expect_ident("a name for each record")?;
    if !cursor.eat_keyword("in") {
        return Err(cursor.error_here("expected 'in' and an entity"));
    }
    let entity = cursor.expect_ident("an entity")?;
    let mut end = entity.span;
    let filter = if cursor.eat_keyword("where") {
        let condition = parse_expr(cursor)?;
        end = condition.span;
        Some(Box::new(condition))
    } else {
        None
    };
    let order = parse_order(cursor, &mut end)?;
    Ok(Expr {
        span: join(start, end),
        kind: ExprKind::Find { var, entity, filter, order },
    })
}

/// A string read for slots: `{customer.name}` is a slot, `{{` and `}}` are
/// braces.
/// Text without slots stays a plain string. A slot's span counts from the
/// string's start as if nothing before it were escaped.
fn text_kind(token: &Token) -> ParseResult<ExprKind> {
    let text = &token.text;
    let mut parts = Vec::new();
    let mut plain = String::new();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        if rest.starts_with("{{") {
            plain.push('{');
            i += 2;
            continue;
        }
        if rest.starts_with("}}") {
            plain.push('}');
            i += 2;
            continue;
        }
        if rest.starts_with('}') {
            return Err(Diagnostic::error("'}' ends a slot; write '}}' for a brace", token.span));
        }
        if !rest.starts_with('{') {
            let c = rest.chars().next().unwrap_or_default();
            plain.push(c);
            i += c.len_utf8();
            continue;
        }
        let Some(end) = rest.find('}') else {
            return Err(Diagnostic::error(
                "'{' starts a slot; close it with '}', or write '{{' for a brace",
                token.span,
            ));
        };
        let offset = token.span.start + 1 + i as u32 + 1;
        let value = slot_path(&rest[1..end], offset, token.span)?;
        if !plain.is_empty() {
            parts.push(TextPart::Plain { text: std::mem::take(&mut plain) });
        }
        parts.push(TextPart::Slot { value });
        i += end + 1;
    }
    if parts.is_empty() {
        return Ok(ExprKind::Str { value: plain });
    }
    if !plain.is_empty() {
        parts.push(TextPart::Plain { text: plain });
    }
    Ok(ExprKind::Text { parts })
}

/// The path in a slot, `customer.name`, as a name and its members. Spaces
/// around it are allowed.
fn slot_path(inner: &str, offset: u32, string: Span) -> ParseResult<Expr> {
    let lead = (inner.len() - inner.trim_start().len()) as u32;
    let path = inner.trim();
    let wrong = || {
        Diagnostic::error(
            format!("a slot names a value or a field, like {{customer.name}}, not {{{path}}}"),
            string,
        )
    };
    let mut expr: Option<Expr> = None;
    let mut at = offset + lead;
    for word in path.split('.') {
        let valid = word.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid {
            return Err(wrong());
        }
        let span = Span {
            start: at,
            end: at + word.len() as u32,
            line: string.line,
            col: string.col + (at - string.start),
        };
        expr = Some(match expr {
            None => Expr {
                kind: ExprKind::Name { name: word.to_string() },
                span,
            },
            Some(object) => Expr {
                span: join(object.span, span),
                kind: ExprKind::Member {
                    object: Box::new(object),
                    member: Ident {
                        name: word.to_string(),
                        span,
                    },
                },
            },
        });
        at += word.len() as u32 + 1;
    }
    expr.ok_or_else(wrong)
}
