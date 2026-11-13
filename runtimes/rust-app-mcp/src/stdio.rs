//! The lines of standard input and output, as an agent that starts
//! `<app> mcp` speaks to it.

use std::io::{BufRead, Write};

use crate::server::Lines;

pub struct Stdio;

impl Lines for Stdio {
    fn read(&mut self) -> Option<String> {
        let mut line = String::new();
        match std::io::stdin().lock().read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim_end_matches(['\r', '\n']).to_string()),
        }
    }

    fn write(&mut self, line: &str) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}
