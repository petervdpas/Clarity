//! The C interface of Clarity.AppMcp (include/clarity_app_mcp.h): one table.
//! `serve` runs the service with the program as its host: callbacks that
//! read and write lines (standard input and output when left out), run a
//! command and ask the person for a grant. Inside `run` the program answers
//! with `printed` or `refused`, and a command that asks calls `ask`. Texts
//! cross as UTF-8 with a length. No panic crosses the interface.

use std::cell::RefCell;
use std::ffi::{c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use mcp::discovery::{self, Running};
use mcp::{Answer, Ask, Decision, Grant, Host, Lines, Ran, Server, Tool};
use serde_json::Value;

pub const INTERFACE: u32 = 1;

pub const OK: i32 = 0;
/// The tool list, the options or a text given is not what it should be.
pub const BAD_INPUT: i32 = 1;
/// No running.json, or not a whole one.
pub const NOT_FOUND: i32 = 2;
pub const FAILED: i32 = 3;

pub const GIVEN: i32 = 0;
pub const DECLINED: i32 = 1;
pub const CANCELLED: i32 = 2;
pub const UNSUPPORTED: i32 = 3;

pub const ONCE: i32 = 0;
pub const SESSION: i32 = 1;
pub const REFUSE: i32 = 2;

type Out = *mut *mut u8;

/// A call being run; the program answers it inside `run`.
pub struct ClarityAppMcpCall<'a> {
    tool: &'a Tool,
    arguments: &'a Value,
    ask: &'a mut dyn Ask,
    ran: Option<Ran>,
}

/// The program's side, field for field as in the header.
#[repr(C)]
pub struct ClarityAppMcpHost {
    pub user: *mut c_void,
    /// Points `line` at the next line (kept by the program until the next
    /// read) and gives 0, or gives 1 at the end. Null: standard input.
    pub read: Option<extern "C" fn(*mut c_void, *mut *const u8, *mut usize) -> i32>,
    /// Null: standard output.
    pub write: Option<extern "C" fn(*mut c_void, *const u8, usize)>,
    pub run: extern "C" fn(*mut c_void, *mut ClarityAppMcpCall, *const u8, usize, *const u8, usize),
    /// The grant as JSON; gives ONCE, SESSION or REFUSE.
    pub grant: extern "C" fn(*mut c_void, *const u8, usize) -> i32,
}

#[repr(C)]
pub struct ClarityAppMcpApi {
    pub interface: u32,
    pub serve: extern "C" fn(*const u8, usize, *const u8, usize, *const ClarityAppMcpHost) -> i32,
    pub printed: extern "C" fn(*mut ClarityAppMcpCall, *const u8, usize),
    pub refused: extern "C" fn(*mut ClarityAppMcpCall, *const u8, usize),
    #[allow(clippy::type_complexity)]
    pub ask: extern "C" fn(*mut ClarityAppMcpCall, *const u8, usize, *const u8, usize, Out, *mut usize) -> i32,
    pub running_write: extern "C" fn(*const u8, usize, *const u8, usize, *const u8, usize, Out, *mut usize) -> i32,
    pub running_read: extern "C" fn(*const u8, usize, Out, *mut usize) -> i32,
    pub running_remove: extern "C" fn(*const u8, usize),
    pub free: extern "C" fn(*mut u8, usize),
    pub problem: extern "C" fn(*mut c_char, usize) -> i32,
    pub command_line: extern "C" fn(*mut ClarityAppMcpCall, Out, *mut usize) -> i32,
    #[allow(clippy::type_complexity)]
    pub ask_cli: extern "C" fn(*mut ClarityAppMcpCall, *const u8, usize, *const u8, usize, Out, *mut usize) -> i32,
    pub forward: extern "C" fn(*const u8, usize) -> i32,
}

static API: ClarityAppMcpApi = ClarityAppMcpApi { interface: INTERFACE, serve, printed, refused, ask, running_write, running_read, running_remove, free, problem, command_line, ask_cli, forward };

/// The table, the same for the library's whole life.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_app_mcp_api() -> *const ClarityAppMcpApi {
    &API
}

thread_local! {
    static LAST: RefCell<(i32, String)> = const { RefCell::new((OK, String::new())) };
}

fn keep(code: i32, detail: &str) -> i32 {
    LAST.with(|l| *l.borrow_mut() = (code, detail.to_string()));
    code
}

fn bytes<'a>(at: *const u8, len: usize) -> &'a [u8] {
    if at.is_null() || len == 0 { &[] } else { unsafe { std::slice::from_raw_parts(at, len) } }
}

fn text<'a>(at: *const u8, len: usize) -> Result<&'a str, String> {
    std::str::from_utf8(bytes(at, len)).map_err(|_| "a text given is no UTF-8".to_string())
}

fn give(out: Out, out_len: *mut usize, made: Vec<u8>) -> bool {
    if out.is_null() || out_len.is_null() {
        return false;
    }
    let boxed = made.into_boxed_slice();
    let len = boxed.len();
    unsafe {
        *out = Box::into_raw(boxed).cast::<u8>();
        *out_len = len;
    }
    true
}

/// The program as the core's host.
struct Program<'a>(&'a ClarityAppMcpHost);

impl Lines for Program<'_> {
    fn read(&mut self) -> Option<String> {
        let Some(read) = self.0.read else { return mcp::stdio::Stdio.read() };
        let (mut at, mut len) = (std::ptr::null(), 0usize);
        if read(self.0.user, &mut at, &mut len) != 0 {
            return None;
        }
        Some(String::from_utf8_lossy(bytes(at, len)).into_owned())
    }

    fn write(&mut self, line: &str) {
        match self.0.write {
            Some(write) => write(self.0.user, line.as_ptr(), line.len()),
            None => mcp::stdio::Stdio.write(line),
        }
    }
}

impl Host for Program<'_> {
    fn run(&mut self, tool: &Tool, arguments: &Value, ask: &mut dyn Ask) -> Ran {
        let given = arguments.to_string();
        let mut call = ClarityAppMcpCall { tool, arguments, ask, ran: None };
        (self.0.run)(self.0.user, &mut call, tool.name.as_ptr(), tool.name.len(), given.as_ptr(), given.len());
        call.ran.unwrap_or_else(|| Ran::Printed(String::new()))
    }

    fn grant(&mut self, grant: &Grant) -> Decision {
        let json = grant.to_json().to_string();
        match (self.0.grant)(self.0.user, json.as_ptr(), json.len()) {
            ONCE => Decision::Once,
            SESSION => Decision::Session,
            _ => Decision::Refuse,
        }
    }
}

extern "C" fn serve(list: *const u8, list_len: usize, options: *const u8, options_len: usize, host: *const ClarityAppMcpHost) -> i32 {
    if host.is_null() {
        return keep(BAD_INPUT, "no host given");
    }
    let host = unsafe { &*host };
    let made = (|| {
        let options = text(options, options_len)?;
        Server::new(text(list, list_len)?, if options.trim().is_empty() { "{}" } else { options })
    })();
    let mut server = match made {
        Ok(server) => server,
        Err(problem) => return keep(BAD_INPUT, &problem),
    };
    let (mut lines, mut program) = (Program(host), Program(host));
    match catch_unwind(AssertUnwindSafe(|| server.serve(&mut lines, &mut program))) {
        Ok(()) => OK,
        Err(_) => keep(FAILED, "a fault inside Clarity.AppMcp"),
    }
}

fn answer(call: *mut ClarityAppMcpCall, ran: Ran) {
    if let Some(call) = unsafe { call.as_mut() } {
        call.ran = Some(ran);
    }
}

extern "C" fn printed(call: *mut ClarityAppMcpCall, at: *const u8, len: usize) {
    answer(call, Ran::Printed(String::from_utf8_lossy(bytes(at, len)).into_owned()));
}

extern "C" fn refused(call: *mut ClarityAppMcpCall, at: *const u8, len: usize) {
    answer(call, Ran::Refused(String::from_utf8_lossy(bytes(at, len)).into_owned()));
}

/// Asks the agent; a given answer goes to `out` as JSON.
extern "C" fn ask(call: *mut ClarityAppMcpCall, message: *const u8, message_len: usize, schema: *const u8, schema_len: usize, out: Out, out_len: *mut usize) -> i32 {
    let Some(call) = (unsafe { call.as_mut() }) else { return keep(BAD_INPUT, "no call given") };
    let (Ok(message), Ok(schema)) = (text(message, message_len), text(schema, schema_len)) else { return keep(BAD_INPUT, "a text given is no UTF-8") };
    let Ok(schema) = serde_json::from_str::<Value>(schema) else { return keep(BAD_INPUT, "the answer's schema is no JSON") };
    match call.ask.ask(message, &schema) {
        Answer::Given(value) => {
            give(out, out_len, value.to_string().into_bytes());
            GIVEN
        }
        Answer::Declined => DECLINED,
        Answer::Cancelled => CANCELLED,
        Answer::Unsupported => UNSUPPORTED,
    }
}

/// The call as the command line the program reads, as a JSON array of texts.
extern "C" fn command_line(call: *mut ClarityAppMcpCall, out: Out, out_len: *mut usize) -> i32 {
    let Some(call) = (unsafe { call.as_ref() }) else { return keep(BAD_INPUT, "no call given") };
    let line = Value::from(mcp::cli::command_line(call.tool, call.arguments));
    if give(out, out_len, line.to_string().into_bytes()) { OK } else { keep(BAD_INPUT, "nowhere to give the result") }
}

/// Asks the agent a command's question of the kind `form` (JSON, as
/// conformance/command-line.json writes it); the answer as the terminal
/// gives it goes to `out`, or NOT_FOUND when there is none.
extern "C" fn ask_cli(call: *mut ClarityAppMcpCall, question: *const u8, question_len: usize, form: *const u8, form_len: usize, out: Out, out_len: *mut usize) -> i32 {
    let Some(call) = (unsafe { call.as_mut() }) else { return keep(BAD_INPUT, "no call given") };
    let (Ok(question), Ok(form)) = (text(question, question_len), text(form, form_len)) else { return keep(BAD_INPUT, "a text given is no UTF-8") };
    let Ok(form) = serde_json::from_str::<Value>(form) else { return keep(BAD_INPUT, "the question's form is no JSON") };
    match mcp::cli::ask(call.ask, question, &form) {
        Some(answer) => {
            give(out, out_len, answer.into_bytes());
            OK
        }
        None => keep(NOT_FOUND, "no answer"),
    }
}

/// Writes running.json at `file` for this process; gives it as JSON.
#[allow(clippy::too_many_arguments)]
extern "C" fn running_write(file: *const u8, file_len: usize, app: *const u8, app_len: usize, address: *const u8, address_len: usize, out: Out, out_len: *mut usize) -> i32 {
    let made = (|| {
        let running = Running::new(text(app, app_len)?, text(address, address_len)?)?;
        discovery::write(Path::new(text(file, file_len)?), &running)?;
        Ok::<_, String>(running)
    })();
    match made {
        Ok(running) => {
            give(out, out_len, running.to_json().to_string().into_bytes());
            OK
        }
        Err(problem) => keep(FAILED, &problem),
    }
}

extern "C" fn running_read(file: *const u8, file_len: usize, out: Out, out_len: *mut usize) -> i32 {
    let Ok(file) = text(file, file_len) else { return keep(BAD_INPUT, "a text given is no UTF-8") };
    match discovery::read(Path::new(file)) {
        Some(running) => {
            give(out, out_len, running.to_json().to_string().into_bytes());
            OK
        }
        None => keep(NOT_FOUND, "no running app"),
    }
}

extern "C" fn running_remove(file: *const u8, file_len: usize) {
    if let Ok(file) = text(file, file_len) {
        discovery::remove(Path::new(file));
    }
}

/// Forwards standard input and output to the running app that running.json
/// at `file` names, until either ends: 1 when an app answered, 0 when none
/// does (the program then serves as the closed app).
extern "C" fn forward(file: *const u8, file_len: usize) -> i32 {
    let Ok(file) = text(file, file_len) else { return 0 };
    match catch_unwind(AssertUnwindSafe(|| mcp::relay::forward(Path::new(file)))) {
        Ok(true) => 1,
        _ => 0,
    }
}

extern "C" fn free(at: *mut u8, len: usize) {
    if !at.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(at, len)) })));
    }
}

extern "C" fn problem(into: *mut c_char, size: usize) -> i32 {
    let (code, detail) = LAST.with(|l| l.borrow().clone());
    if !into.is_null() && size > 0 {
        let mut n = detail.len().min(size - 1);
        while n > 0 && !detail.is_char_boundary(n) {
            n -= 1;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(detail.as_ptr(), into.cast::<u8>(), n);
            *into.add(n) = 0;
        }
    }
    code
}
