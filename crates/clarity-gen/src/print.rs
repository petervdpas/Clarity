//! Expressions back as Clarity text, for the messages of contract checks
//! (`condition failed: order.status == Pending`).

use clarity_spec::code::{Expr, ExprKind, UnaryOp};

pub fn expr(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Number { value } => value.clone(),
        ExprKind::Str { value } => format!("{value:?}"),
        ExprKind::Bool { value } => value.to_string(),
        ExprKind::Name { name } => name.clone(),
        ExprKind::Member { object, member } => format!("{}.{}", expr(object), member.name),
        ExprKind::Unary { op: UnaryOp::Not, operand } => format!("not {}", expr(operand)),
        ExprKind::Unary { op: UnaryOp::Neg, operand } => format!("-{}", expr(operand)),
        ExprKind::Binary { op, left, right } => format!("{} {} {}", expr(left), op.symbol(), expr(right)),
        ExprKind::In { value, options } => format!(
            "{} in [{}]",
            expr(value),
            options.iter().map(expr).collect::<Vec<_>>().join(", ")
        ),
        ExprKind::Call { function, args } => format!(
            "{}({})",
            function.name,
            args.iter().map(expr).collect::<Vec<_>>().join(", ")
        ),
    }
}
