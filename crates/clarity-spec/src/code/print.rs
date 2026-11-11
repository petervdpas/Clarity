//! Expressions back as Clarity text, for the messages of contract checks
//! (`condition failed: order.status == Pending`), and mappings, which the
//! IDE's mapping page writes back.

use super::{Expr, ExprKind, UnaryOp};

/// A mapping declaration as the language writes it (`mapping OrderFeed of
/// Order from xml` with its lines), for writing it back into its module.
pub fn mapping(m: &super::MappingDecl) -> String {
    super::mapping::text(&super::mapping::form_of(m))
}

pub fn expr(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Number { value } => value.clone(),
        ExprKind::Str { value } => format!("{value:?}"),
        ExprKind::Text { parts } => format!("{:?}", super::template(parts)),
        ExprKind::Bool { value } => value.to_string(),
        ExprKind::Empty => "none".to_string(),
        ExprKind::Name { name } => name.clone(),
        ExprKind::Member { object, member } => format!("{}.{}", expr(object), member.name),
        ExprKind::Unary { op: UnaryOp::Not, operand } => format!("not {}", expr(operand)),
        ExprKind::Unary { op: UnaryOp::Neg, operand } => format!("-{}", expr(operand)),
        ExprKind::Binary { op, left, right } => format!("{} {} {}", expr(left), op.symbol(), expr(right)),
        ExprKind::Range { from, to } => format!("{} to {}", expr(from), expr(to)),
        ExprKind::In { value, options } => format!(
            "{} in [{}]",
            expr(value),
            options.iter().map(expr).collect::<Vec<_>>().join(", ")
        ),
        ExprKind::Call { function, args, named } => format!(
            "{}({})",
            function.name,
            args.iter().map(expr).chain(named.iter().map(|n| format!("{}: {}", n.name.name, expr(&n.value)))).collect::<Vec<_>>().join(", ")
        ),
        ExprKind::Find { var, entity, filter, order } => query(&format!("find {} in {}", var.name, entity.name), filter.as_deref(), order),
        ExprKind::Files { var, store, filter, order } => query(&format!("files {} in {}", var.name, store.name), filter.as_deref(), order),
        ExprKind::ReadText { file } => format!("read text of {}", expr(file)),
        ExprKind::ReadBytes { file } => format!("read bytes of {}", expr(file)),
        ExprKind::FormatOf { value, mapping: super::MappingUse::Inline { format, options } } => format!("{} of {}{}", format.name, expr(value), format_options(options)),
        ExprKind::FormatOf { value, mapping: super::MappingUse::Named { name } } => format!("text of {} using {}", expr(value), name.name),
        ExprKind::ReadFormat { ty, input, mapping } => {
            let read = match &ty.kind {
                super::TypeRefKind::Named { name } => name.clone(),
                super::TypeRefKind::List { item } => format!("list<{item}>"),
                _ => "...".to_string(),
            };
            match mapping {
                super::MappingUse::Inline { format, options } => format!("read {read} from {} {}{}", format.name, expr(input), format_options(options)),
                super::MappingUse::Named { name } => format!("read {read} from {} using {}", expr(input), name.name),
            }
        }
        ExprKind::Http { method, url, from, pages, body, headers, query } => {
            let mut text = format!("http {} {}", method.word(), expr(url));
            if let Some(from) = from {
                text.push_str(&format!(" from {}", from.name));
            }
            if *pages {
                text.push_str(" every page");
            }
            if let Some(body) = body {
                text.push_str(&format!(" with {}", expr(body)));
            }
            let when = |pair: &super::HttpPair| pair.when.as_ref().map(|w| format!(" when {}", expr(w))).unwrap_or_default();
            for header in headers {
                text.push_str(&format!(" header {:?}: {}{}", header.name.name, expr(&header.value), when(header)));
            }
            for pair in query {
                text.push_str(&format!(" query {:?}: {}{}", pair.name.name, expr(&pair.value), when(pair)));
            }
            text
        }
        ExprKind::New { entity } => format!("new {}", entity.name),
        ExprKind::Ask { question, .. } => format!("ask {}", expr(question)),
        ExprKind::Key { key, args } if args.is_empty() => format!("@{}", key.name),
        ExprKind::Key { key, args } => format!(
            "@{}({})",
            key.name,
            args.iter().map(|a| format!("{} = {}", a.name.name, expr(&a.value))).collect::<Vec<_>>().join(", ")
        ),
        ExprKind::ListOf { items } => format!("[{}]", items.iter().map(expr).collect::<Vec<_>>().join(", ")),
        ExprKind::Index { list, index } => format!("{}[{}]", expr(list), expr(index)),
        ExprKind::EngineCall { engine, function, args } => {
            format!("{}.{}({})", engine.name, function.name, args.iter().map(expr).collect::<Vec<_>>().join(", "))
        }
    }
}

/// A `find` or `files` with its condition and order.
fn query(head: &str, filter: Option<&Expr>, order: &[super::OrderKey]) -> String {
    let mut text = head.to_string();
    if let Some(filter) = filter {
        text.push_str(&format!(" where {}", expr(filter)));
    }
    if !order.is_empty() {
        let keys: Vec<String> = order
            .iter()
            .map(|k| {
                let path: String = k.through.iter().map(|t| format!("{}.", t.name)).collect();
                format!("{}.{path}{}{}", k.var.name, k.field.name, if k.descending { " desc" } else { "" })
            })
            .collect();
        text.push_str(&format!(" order by {}", keys.join(", ")));
    }
    text
}

/// A type as code writes it: `integer`, `list<Shape>`.
pub fn type_ref(ty: &super::TypeRef) -> String {
    use super::TypeRefKind as K;
    match &ty.kind {
        K::Text => "text".into(),
        K::Integer => "integer".into(),
        K::Decimal => "decimal".into(),
        K::Money => "money".into(),
        K::Bool => "bool".into(),
        K::Date => "date".into(),
        K::DateTime => "datetime".into(),
        K::Duration => "duration".into(),
        K::File => "file".into(),
        K::Bytes => "bytes".into(),
        K::Named { name } => name.clone(),
        K::List { item } => format!("list<{item}>"),
    }
}

/// A format's options as code writes them: ` at "data.orders"`.
fn format_options(options: &[super::FormatOption]) -> String {
    options.iter().map(|o| format!(" {} {}", o.name.name, expr(&o.value))).collect()
}
