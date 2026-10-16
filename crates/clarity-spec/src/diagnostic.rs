use serde::Serialize;
use ts_rs::TS;

use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// Path of the spec file; empty until the project loader fills it in.
    pub path: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            path: String::new(),
            span,
        }
    }

    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(message, span)
        }
    }

    pub fn in_file(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(
            f,
            "{}:{}:{}: {level}: {}",
            self.path, self.span.line, self.span.col, self.message
        )
    }
}
