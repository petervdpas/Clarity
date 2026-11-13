//! A toast shown by name (`toast saved(name = customer.name)`): its look,
//! as the project declares it (`toast saved` with its level, lifespan, icon
//! and corner), and its text, the toast's key with the slots the statement
//! fills. Worked out once here; each stack's generator writes the call of
//! its runtime (`ui.toast(...)`), and the screen shows it as the toast.

use clarity_spec::Project;
use clarity_spec::code::{Arg, Expr, ExprKind};
use clarity_spec::model::Ident;

/// A toast's look, as every runtime carries it to the screen.
pub struct Look {
    pub name: String,
    /// `info`, `success`, `warning` or `error`.
    pub level: &'static str,
    /// In milliseconds: the toast's own, else Clarity's default.
    pub lifespan: u32,
    pub icon: Option<String>,
    /// `topRight`, `topLeft`, `bottomRight` or `bottomLeft`.
    pub corner: &'static str,
}

/// The toast `name`'s look and its text with the slots `args` fill; why
/// not, when the project declares no such toast or it has no text.
pub fn shown(project: &Project, name: &Ident, args: &[Arg]) -> Result<(Look, Expr), String> {
    let toast = project.toast(&name.name).ok_or_else(|| format!("there is no toast {}", name.name))?;
    let key = toast.text.clone().ok_or_else(|| format!("toast {} has no text", name.name))?;
    let look = Look {
        name: toast.name.name.clone(),
        level: toast.level.keyword(),
        lifespan: toast.lifespan.unwrap_or(clarity_spec::code::DEFAULT_TOAST_LIFESPAN),
        icon: toast.icon.clone(),
        corner: toast.corner.keyword(),
    };
    let text = Expr { span: name.span, kind: ExprKind::Key { key, args: args.to_vec() } };
    Ok((look, text))
}

/// A text as a string literal of C, C++, Go, C# and Rust alike: quoted, with
/// its backslashes and quotes escaped (a toast's name, level, icon and
/// corner are plain words, so nothing more is needed).
pub fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}
