//! conformance/app-mcp.json through the C table, as Go, C, C++ and C# run
//! it: the program's callbacks read the case's messages, write what the
//! service says, run a tool as the case's host says and give the person's
//! answers.

use std::collections::VecDeque;
use std::ffi::c_void;

use clarity_app_mcp::*;
use serde_json::{Value, json};

struct Case {
    sends: VecDeque<String>,
    line: String,
    written: Vec<Value>,
    host: Value,
    grants: VecDeque<i32>,
    asked: Vec<Value>,
}

fn case<'a>(user: *mut c_void) -> &'a mut Case {
    unsafe { &mut *user.cast::<Case>() }
}

fn api() -> &'static ClarityAppMcpApi {
    unsafe { &*clarity_app_mcp_api() }
}

fn text(at: *const u8, len: usize) -> String {
    String::from_utf8(unsafe { std::slice::from_raw_parts(at, len) }.to_vec()).unwrap()
}

extern "C" fn read(user: *mut c_void, line: *mut *const u8, len: *mut usize) -> i32 {
    let case = case(user);
    let Some(next) = case.sends.pop_front() else { return 1 };
    case.line = next;
    unsafe {
        *line = case.line.as_ptr();
        *len = case.line.len();
    }
    0
}

extern "C" fn write(user: *mut c_void, line: *const u8, len: usize) {
    case(user).written.push(serde_json::from_str(&text(line, len)).unwrap());
}

extern "C" fn run(user: *mut c_void, call: *mut ClarityAppMcpCall, tool: *const u8, tool_len: usize, _args: *const u8, _args_len: usize) {
    let said = case(user).host[text(tool, tool_len)].clone();
    let say = |key: &str| said[key].as_str().unwrap().to_string();
    let answer = |done: bool, words: String| if done { (api().printed)(call, words.as_ptr(), words.len()) } else { (api().refused)(call, words.as_ptr(), words.len()) };
    if said.get("refuse").is_some() {
        return answer(false, say("refuse"));
    }
    if let Some(question) = said.get("ask") {
        let (message, schema) = (question["message"].as_str().unwrap(), question["schema"].to_string());
        let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
        let code = (api().ask)(call, message.as_ptr(), message.len(), schema.as_ptr(), schema.len(), &mut out, &mut len);
        return match code {
            GIVEN => {
                let given: Value = serde_json::from_str(&text(out, len)).unwrap();
                (api().free)(out, len);
                answer(given == json!(true), if given == json!(true) { say("print") } else { say("kept") })
            }
            UNSUPPORTED => answer(false, say("unanswered")),
            _ => answer(false, say("kept")),
        };
    }
    answer(true, say("print"))
}

extern "C" fn grant(user: *mut c_void, at: *const u8, len: usize) -> i32 {
    let case = case(user);
    let grant: Value = serde_json::from_str(&text(at, len)).unwrap();
    case.asked.push(json!({ "tool": grant["tool"], "kind": grant["kind"] }));
    case.grants.pop_front().expect("a grant asked that the case does not answer")
}

#[test]
fn every_exchange_through_the_c_table() {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/app-mcp.json")).unwrap()).unwrap();
    for each in file["cases"].as_array().unwrap() {
        let name = each["name"].as_str().unwrap();
        // A case may say what the project allows while the app is closed (the tool list's `whileClosed`).
        let mut tools = file["tools"].clone();
        if let Some(closed) = each.get("whileClosed") {
            tools["whileClosed"] = closed.clone();
        }
        let list = tools.to_string();
        let mut expected = Vec::new();
        let mut sends = VecDeque::new();
        for step in each["exchange"].as_array().unwrap() {
            match (step.get("send"), step.get("expect")) {
                (Some(Value::String(raw)), _) => sends.push_back(raw.clone()),
                (Some(message), _) => sends.push_back(message.to_string()),
                (_, Some(message)) => expected.push(message.clone()),
                _ => panic!("{name}"),
            }
        }
        let grants = each["grants"].as_array().unwrap().iter().map(|g| match g.as_str().unwrap() { "once" => ONCE, "session" => SESSION, _ => REFUSE }).collect();
        let mut case = Case { sends, line: String::new(), written: Vec::new(), host: file["host"].clone(), grants, asked: Vec::new() };
        let host = ClarityAppMcpHost { user: (&mut case as *mut Case).cast(), read: Some(read), write: Some(write), run, grant };
        let options = each["options"].to_string();
        assert_eq!((api().serve)(list.as_ptr(), list.len(), options.as_ptr(), options.len(), &host), OK, "{name}");
        assert_eq!(case.written, expected, "{name}");
        assert_eq!(Value::Array(case.asked), each["asked"], "{name}: the grants asked");
    }
}

#[test]
fn a_tool_list_that_is_no_json_is_refused_with_why() {
    let host = ClarityAppMcpHost { user: std::ptr::null_mut(), read: Some(read), write: Some(write), run, grant };
    assert_eq!((api().serve)(b"{x".as_ptr(), 2, b"".as_ptr(), 0, &host), BAD_INPUT);
    let mut buf = vec![0 as std::ffi::c_char; 128];
    assert_eq!((api().problem)(buf.as_mut_ptr(), buf.len()), BAD_INPUT);
    assert!(unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().starts_with("the tool list is no JSON"));
}

#[test]
fn running_json_through_the_table() {
    let dir = std::env::temp_dir().join(format!("clarity-app-mcp-ffi-{}", std::process::id()));
    let file = dir.join("running.json").to_string_lossy().into_owned();
    let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
    assert_eq!((api().running_write)(file.as_ptr(), file.len(), b"shop".as_ptr(), 4, b"sock".as_ptr(), 4, &mut out, &mut len), OK);
    let written: Value = serde_json::from_str(&text(out, len)).unwrap();
    (api().free)(out, len);
    assert_eq!((api().running_read)(file.as_ptr(), file.len(), &mut out, &mut len), OK);
    assert_eq!(serde_json::from_str::<Value>(&text(out, len)).unwrap(), written);
    (api().free)(out, len);
    (api().running_remove)(file.as_ptr(), file.len());
    assert_eq!((api().running_read)(file.as_ptr(), file.len(), &mut out, &mut len), NOT_FOUND);
    let _ = std::fs::remove_dir_all(dir);
}

/// One call served with every change and read allowed, the program's `run`
/// doing `act`; what the tool answered.
fn served(sends: Vec<Value>, act: extern "C" fn(*mut c_void, *mut ClarityAppMcpCall, *const u8, usize, *const u8, usize), form: Value) -> Vec<Value> {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/app-mcp.json")).unwrap()).unwrap();
    let list = file["tools"].to_string();
    let mut case = Case { sends: sends.iter().map(Value::to_string).collect(), line: String::new(), written: Vec::new(), host: form, grants: VecDeque::new(), asked: Vec::new() };
    let host = ClarityAppMcpHost { user: (&mut case as *mut Case).cast(), read: Some(read), write: Some(write), run: act, grant };
    let options = r#"{"open":false,"closedReads":true,"closedChanges":true}"#;
    assert_eq!((api().serve)(list.as_ptr(), list.len(), options.as_ptr(), options.len(), &host), OK);
    case.written
}

extern "C" fn print_command_line(_user: *mut c_void, call: *mut ClarityAppMcpCall, _t: *const u8, _tl: usize, _a: *const u8, _al: usize) {
    let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
    assert_eq!((api().command_line)(call, &mut out, &mut len), OK);
    (api().printed)(call, out, len);
    (api().free)(out, len);
}

extern "C" fn print_answer(user: *mut c_void, call: *mut ClarityAppMcpCall, _t: *const u8, _tl: usize, _a: *const u8, _al: usize) {
    let form = case(user).host.to_string();
    let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
    match (api().ask_cli)(call, b"?".as_ptr(), 1, form.as_ptr(), form.len(), &mut out, &mut len) {
        OK => {
            (api().printed)(call, out, len);
            (api().free)(out, len);
        }
        code => {
            assert_eq!(code, NOT_FOUND);
            (api().refused)(call, std::ptr::null(), 0);
        }
    }
}

fn calling(tool: &str, arguments: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": tool, "arguments": arguments } })
}

fn answer_text(reply: &Value) -> (String, bool) {
    let result = &reply["result"];
    (result["content"][0]["text"].as_str().unwrap().to_string(), result["isError"].as_bool().unwrap())
}

#[test]
fn command_lines_through_the_c_table() {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/app-mcp.json")).unwrap()).unwrap();
    for each in file["commandLines"].as_array().unwrap() {
        let written = served(vec![calling(each["tool"].as_str().unwrap(), &each["arguments"])], print_command_line, Value::Null);
        let (line, _) = answer_text(&written[0]);
        assert_eq!(serde_json::from_str::<Value>(&line).unwrap(), each["line"], "{}", each["what"]);
    }
}

#[test]
fn questions_through_the_c_table() {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/app-mcp.json")).unwrap()).unwrap();
    for each in file["questions"].as_array().unwrap() {
        let what = each["what"].as_str().unwrap();
        let elicits = !each["reply"].is_null();
        let mut sends = Vec::new();
        if elicits {
            sends.push(json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "capabilities": { "elicitation": {} } } }));
        }
        sends.push(calling("customer-count", &json!({})));
        if elicits {
            let mut reply = json!({ "jsonrpc": "2.0", "id": "ask-1" });
            reply["result"] = each["reply"].clone();
            sends.push(reply);
        }
        let written = served(sends, print_answer, each["form"].clone());
        let last = written.last().unwrap();
        if elicits {
            assert_eq!(written[1]["params"]["requestedSchema"]["properties"]["answer"], each["schema"], "{what}");
        }
        match each["answer"].as_str() {
            Some(answer) => assert_eq!(answer_text(last), (answer.to_string(), false), "{what}"),
            None => assert!(answer_text(last).1, "{what}: no answer"),
        }
    }
}
