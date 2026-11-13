//! Clarity.AppMcp: the MCP service of the programs Clarity generates
//! (docs/ledger/app-mcp.md). An app offers its commands with an `agent` line
//! to AI agents as tools; the person using it grants each call. One core for
//! every stack: Rust programs use this crate, Go, C, C++ and C# programs its
//! C table (`clarity-app-mcp-ffi`). conformance/app-mcp.json pins it.
//!
//! - `tools`: the tool list clarity-gen writes, and how it is listed.
//! - `inputs`: whether a call's arguments fit its tool.
//! - `grants`: who allows a call: the person, once or for the session, or the
//!   person's settings while the app is closed.
//! - `cli`: a call as the command line the program reads, and a command's
//!   question as the agent is asked it.
//! - `rpc`: the JSON-RPC messages.
//! - `server`: the service over lines of JSON, with the app as its `Host`.
//! - `stdio`: the lines of standard input and output.
//! - `discovery`: `running.json`, where a running app is found.
//! - `relay`: the running app listening, and `<app> mcp` forwarding to it.

pub mod cli;
pub mod discovery;
pub mod grants;
pub mod inputs;
pub mod options;
pub mod relay;
pub mod rpc;
pub mod server;
pub mod stdio;
pub mod tools;

pub use grants::{Decision, Kind};
pub use options::Options;
pub use server::{Answer, Ask, Grant, Host, Lines, Ran, Server};
pub use tools::{Tool, ToolList};
