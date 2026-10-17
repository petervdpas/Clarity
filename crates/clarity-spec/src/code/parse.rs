//! Parses `code` blocks: handlers, their statements (one per line, blocks
//! by indentation) and expressions (precedence climbing on one line).

use super::{
    Arg, BinaryOp, Branch, CodeBlock, Expr, ExprKind, FunctionDecl, FunctionKind, HandlerDecl, MessageLevel, OrderKey, Param,
    Stmt, StmtKind, TypeRef, TypeRefKind, UnaryOp,
};
use crate::diagnostic::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::model::Ident;
use crate::parser::{Cursor, Node, ParseResult, first_word, no_children};
use crate::span::Span;

pub(crate) fn parse_code(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> CodeBlock {
    let mut handlers = Vec::new();
    for child in &node.children {
        match parse_handler(child, diagnostics) {
            Ok(handler) => handlers.push(handler),
            Err(diag) => diagnostics.push(diag),
        }
    }
    CodeBlock {
        name,
        handlers,
        span: node.span(),
    }
}

/// `function name(a: T, b: T) -> T` or `action name(...)`, then its
/// `require`, `ensure` and `effects` lines, then the statements.
pub(crate) fn parse_function(node: &Node, kind: FunctionKind, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<FunctionDecl> {
    let mut cursor = Cursor::new(node.line);
    cursor.next();
    let name = cursor.expect_ident(match kind {
        FunctionKind::Function => "a function name",
        FunctionKind::Action => "an action name",
    })?;
    cursor.expect(TokenKind::LParen, "'(' and the parameters")?;
    let mut params = Vec::new();
    if !cursor.eat(TokenKind::RParen) {
        loop {
            let name = cursor.expect_ident("a parameter name")?;
            cursor.expect(TokenKind::Colon, "':' and a type")?;
            params.push(Param {
                name,
                ty: parse_type_ref(&mut cursor)?,
            });
            if cursor.eat(TokenKind::RParen) {
                break;
            }
            cursor.expect(TokenKind::Comma, "',' or ')'")?;
        }
    }
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

fn parse_handler(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<HandlerDecl> {
    let mut cursor = Cursor::new(node.line);
    if !cursor.eat_keyword("handler") {
        return Err(cursor.error_here("expected 'handler <name>'"));
    }
    let name = cursor.expect_ident("a handler name")?;
    cursor.expect_end()?;
    Ok(HandlerDecl {
        name,
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

const STATEMENT_WORDS: [&str; 10] = ["let", "if", "for", "return", "message", "open", "close", "save", "delete", "reload"];

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
        Some("delete") => {
            cursor.next();
            StmtKind::Delete {
                value: parse_expr(&mut cursor)?,
            }
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

fn parse_primary(cursor: &mut Cursor) -> ParseResult<Expr> {
    let Some(token) = cursor.peek() else {
        return Err(cursor.error_here("expected a value"));
    };
    let kind = match token.kind {
        TokenKind::Number => ExprKind::Number {
            value: token.text.clone(),
        },
        TokenKind::Str => ExprKind::Str {
            value: token.text.clone(),
        },
        TokenKind::Ident if token.text == "true" || token.text == "false" => ExprKind::Bool {
            value: token.text == "true",
        },
        TokenKind::Ident if token.text == "find" && cursor.peek_at(2).is_some_and(|t| t.text == "in") => {
            return parse_find(cursor);
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
    let mut order = Vec::new();
    if cursor.eat_keyword("order") {
        if !cursor.eat_keyword("by") {
            return Err(cursor.error_here("expected 'by', as in 'order by c.name'"));
        }
        loop {
            let key_var = cursor.expect_ident("a field, as in 'c.name'")?;
            cursor.expect(TokenKind::Dot, "'.' and a field, as in 'c.name'")?;
            let field = cursor.expect_ident("a field")?;
            end = field.span;
            let descending = cursor.peek().is_some_and(|t| t.text == "desc");
            if descending {
                end = cursor.next().map(|t| t.span).unwrap_or(end);
            }
            order.push(OrderKey { var: key_var, field, descending });
            if !cursor.eat(TokenKind::Comma) {
                break;
            }
        }
    }
    Ok(Expr {
        span: join(start, end),
        kind: ExprKind::Find { var, entity, filter, order },
    })
}
