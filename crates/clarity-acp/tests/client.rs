//! Clarity's ACP client against a small agent of the tests, over a channel
//! in memory, so no vendor program or login is needed: what the agent says
//! reaches the panel, Clarity's tools are handed over, a permission asks the
//! person, and files are read and written only inside the project.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use agent_client_protocol as acp;
use acp::schema::v1::*;
use clarity_acp::{Agent, Decision, Ended, Event, Failure, Host, SignIn, ToolServer, ToolStatus};
use tokio::sync::mpsc;

/// What the fake agent saw from the client.
#[derive(Default)]
struct Seen {
    servers: Vec<String>,
    cwd: Option<PathBuf>,
    signed_in: Option<String>,
    fs: Option<(bool, bool)>,
    terminal: Option<bool>,
    decision: Option<String>,
    read: Option<String>,
    outside_write: Option<bool>,
    inside_edit: Option<String>,
    outside_edit: Option<String>,
}

/// An agent that offers one sign-in, opens a session and, on a prompt,
/// talks, calls a tool, asks permission, reads and writes files through
/// the client and ends its turn.
fn fake_agent(transport: acp::Channel, seen: Arc<Mutex<Seen>>, root: PathBuf) {
    let on_init = seen.clone();
    let on_session = seen.clone();
    let on_auth = seen.clone();
    let on_prompt = seen;
    tokio::spawn(async move {
        let ended = acp::Agent
            .builder()
            .name("fake")
            .on_receive_request(
                async move |init: InitializeRequest, responder, _cx| {
                    let caps = &init.client_capabilities;
                    let mut seen = on_init.lock().unwrap();
                    seen.fs = Some((caps.fs.read_text_file, caps.fs.write_text_file));
                    seen.terminal = Some(caps.terminal);
                    drop(seen);
                    responder.respond(
                        InitializeResponse::new(init.protocol_version)
                            .agent_info(Implementation::new("fake-agent", "1.0"))
                            .auth_methods(vec![AuthMethod::Agent(AuthMethodAgent::new("chatgpt", "Sign in with ChatGPT"))]),
                    )
                },
                acp::on_receive_request!(),
            )
            .on_receive_request(
                async move |auth: AuthenticateRequest, responder, _cx| {
                    on_auth.lock().unwrap().signed_in = Some(auth.method_id.to_string());
                    responder.respond(AuthenticateResponse::new())
                },
                acp::on_receive_request!(),
            )
            .on_receive_request(
                async move |session: NewSessionRequest, responder, _cx| {
                    let mut seen = on_session.lock().unwrap();
                    // As a vendor's program does: no session before it is signed in.
                    if seen.signed_in.is_none() {
                        drop(seen);
                        return responder.respond_with_error(acp::Error::auth_required());
                    }
                    seen.cwd = Some(session.cwd.clone());
                    seen.servers = session
                        .mcp_servers
                        .iter()
                        .map(|s| match s {
                            McpServer::Stdio(s) => format!("{} {} {}", s.name, s.command.display(), s.args.join(" ")),
                            _ => "other".into(),
                        })
                        .collect();
                    drop(seen);
                    responder.respond(NewSessionResponse::new(SessionId::new("s1")))
                },
                acp::on_receive_request!(),
            )
            .on_receive_request(
                async move |prompt: PromptRequest, responder, cx: acp::ConnectionTo<acp::Client>| {
                    let seen = on_prompt.clone();
                    let root = root.clone();
                    cx.spawn({
                        let cx = cx.clone();
                        async move {
                            let session = prompt.session_id.clone();
                            let say = |update: SessionUpdate| cx.send_notification(SessionNotification::new(session.clone(), update));
                            say(SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new("Making the window.")))))?;
                            say(SessionUpdate::ToolCall(ToolCall::new(ToolCallId::new("t1"), "write_file design/Main.cxaml")))?;
                            say(SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(ToolCallId::new("t1"), ToolCallUpdateFields::new().status(ToolCallStatus::Completed))))?;
                            let options = || {
                                vec![
                                    PermissionOption::new(PermissionOptionId::new("always"), "Always", PermissionOptionKind::AllowAlways),
                                    PermissionOption::new(PermissionOptionId::new("once"), "Once", PermissionOptionKind::AllowOnce),
                                    PermissionOption::new(PermissionOptionId::new("no"), "No", PermissionOptionKind::RejectOnce),
                                ]
                            };
                            let chosen = |outcome: RequestPermissionOutcome| match outcome {
                                RequestPermissionOutcome::Selected(s) => s.option_id.to_string(),
                                _ => "cancelled".into(),
                            };
                            // An edit inside the project, told of first, then asked for by its id alone.
                            say(SessionUpdate::ToolCall(ToolCall::new(ToolCallId::new("t3"), "Update file").kind(ToolKind::Edit).locations(vec![ToolCallLocation::new(root.join("design/Main.cxaml"))])))?;
                            let inside = cx.send_request(RequestPermissionRequest::new(session.clone(), ToolCallUpdate::new(ToolCallId::new("t3"), ToolCallUpdateFields::new().title("Update file")), options())).block_task().await?;
                            // An edit outside the project.
                            let outside_edit = cx
                                .send_request(RequestPermissionRequest::new(
                                    session.clone(),
                                    ToolCallUpdate::new(ToolCallId::new("t4"), ToolCallUpdateFields::new().title("Update outside").kind(ToolKind::Edit).locations(vec![ToolCallLocation::new(root.join("../elsewhere.md"))])),
                                    options(),
                                ))
                                .block_task()
                                .await?;
                            let asked = cx
                                .send_request(RequestPermissionRequest::new(
                                    session.clone(),
                                    ToolCallUpdate::new(ToolCallId::new("t2"), ToolCallUpdateFields::new().title("Run cargo build")),
                                    vec![
                                        PermissionOption::new(PermissionOptionId::new("always"), "Always", PermissionOptionKind::AllowAlways),
                                        PermissionOption::new(PermissionOptionId::new("once"), "Once", PermissionOptionKind::AllowOnce),
                                        PermissionOption::new(PermissionOptionId::new("no"), "No", PermissionOptionKind::RejectOnce),
                                    ],
                                ))
                                .block_task()
                                .await?;
                            let decision = match asked.outcome {
                                RequestPermissionOutcome::Selected(s) => s.option_id.to_string(),
                                _ => "cancelled".into(),
                            };
                            let read = cx.send_request(ReadTextFileRequest::new(session.clone(), root.join("notes.md"))).block_task().await?;
                            cx.send_request(WriteTextFileRequest::new(session.clone(), root.join("design/New.cxaml"), "<Window />\n")).block_task().await?;
                            let outside = cx.send_request(WriteTextFileRequest::new(session.clone(), root.join("../escape.txt"), "x")).block_task().await.is_ok();
                            let mut seen = seen.lock().unwrap();
                            seen.inside_edit = Some(chosen(inside.outcome));
                            seen.outside_edit = Some(chosen(outside_edit.outcome));
                            seen.decision = Some(decision);
                            seen.read = Some(read.content);
                            seen.outside_write = Some(outside);
                            drop(seen);
                            responder.respond(PromptResponse::new(StopReason::EndTurn))
                        }
                    })
                },
                acp::on_receive_request!(),
            )
            .connect_to(transport)
            .await;
        if let Err(err) = ended {
            eprintln!("the fake agent ended: {err}");
        }
    });
}

struct Run {
    agent: Agent,
    seen: Arc<Mutex<Seen>>,
    events: mpsc::UnboundedReceiver<Event>,
    root: PathBuf,
    /// What the person was asked, in order.
    asked: Arc<Mutex<Vec<String>>>,
}

async fn started(name: &str, decide: Decision) -> Run {
    let root = std::env::temp_dir().join(format!("clarity-acp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("design")).unwrap();
    std::fs::write(root.join("notes.md"), "line one\nline two\n").unwrap();
    let (client_end, agent_end) = acp::Channel::duplex();
    let seen = Arc::new(Mutex::new(Seen::default()));
    fake_agent(agent_end, seen.clone(), root.clone());
    let (events_tx, events) = mpsc::unbounded_channel();
    let (permissions, mut asked) = mpsc::unbounded_channel::<(clarity_acp::Permission, tokio::sync::oneshot::Sender<Decision>)>();
    let titles = Arc::new(Mutex::new(Vec::new()));
    let heard = titles.clone();
    tokio::spawn(async move {
        while let Some((permission, reply)) = asked.recv().await {
            heard.lock().unwrap().push(permission.title);
            let _ = reply.send(decide);
        }
    });
    let agent = Agent::start_on(client_end, &root, Host { events: events_tx, permissions }).await.expect("the agent starts");
    Run { agent, seen, events, root, asked: titles }
}

fn drained(events: &mut mpsc::UnboundedReceiver<Event>) -> Vec<Event> {
    let mut out = Vec::new();
    while let Ok(e) = events.try_recv() {
        out.push(e);
    }
    out
}

#[tokio::test]
async fn a_turn_reaches_the_panel_with_clarity_s_tools() {
    let mut run = started("turn", Decision::Allow).await;
    assert_eq!(run.agent.name(), "fake-agent");
    assert_eq!(run.agent.sign_ins(), [SignIn { id: "chatgpt".into(), name: "Sign in with ChatGPT".into(), description: None }]);
    run.agent.sign_in("chatgpt").await.unwrap();

    let tools = ToolServer { name: "clarity".into(), command: PathBuf::from("/opt/clarity/clarity"), args: vec!["mcp".into(), "--turn".into(), "/tmp/turn.jsonl".into()] };
    let session = run.agent.new_session(vec![tools]).await.unwrap();
    let ended = run.agent.prompt(&session, "Make a window").await.unwrap();
    assert_eq!(ended, Ended::Done);

    let seen = run.seen.lock().unwrap();
    assert_eq!(seen.signed_in.as_deref(), Some("chatgpt"));
    assert_eq!(seen.servers, ["clarity /opt/clarity/clarity mcp --turn /tmp/turn.jsonl"]);
    assert_eq!(seen.cwd.as_deref(), Some(run.root.as_path()));
    assert_eq!(seen.fs, Some((true, true)), "the client reads and writes files");
    assert_eq!(seen.terminal, Some(false), "no terminal: Clarity runs nothing for the agent");
    // Allowed once, never always: the next command asks again.
    assert_eq!(seen.decision.as_deref(), Some("once"));
    assert_eq!(seen.read.as_deref(), Some("line one\nline two\n"));
    assert_eq!(seen.outside_write, Some(false), "a write outside the project is refused");
    // An edit inside the project is the client's to allow (once): Undo takes it back.
    assert_eq!(seen.inside_edit.as_deref(), Some("once"));
    assert_eq!(seen.outside_edit.as_deref(), Some("once"), "allowed by the person here");
    assert_eq!(*run.asked.lock().unwrap(), ["Update outside", "Run cargo build"], "the person is asked only for what is outside the project or runs");
    drop(seen);
    assert_eq!(std::fs::read_to_string(run.root.join("design/New.cxaml")).unwrap(), "<Window />\n");
    assert!(!run.root.join("../escape.txt").exists());

    let events = drained(&mut run.events);
    assert!(events.contains(&Event::Text("Making the window.".into())), "{events:?}");
    assert!(events.contains(&Event::Tool { id: "t1".into(), title: "write_file design/Main.cxaml".into(), status: ToolStatus::Pending }), "{events:?}");
    assert!(events.contains(&Event::ToolUpdate { id: "t1".into(), title: None, status: Some(ToolStatus::Done) }), "{events:?}");
    let _ = std::fs::remove_dir_all(&run.root);
}

#[tokio::test]
async fn denied_is_rejected_once() {
    let run = started("deny", Decision::Deny).await;
    run.agent.sign_in("chatgpt").await.unwrap();
    let session = run.agent.new_session(Vec::new()).await.unwrap();
    run.agent.prompt(&session, "Build it").await.unwrap();
    assert_eq!(run.seen.lock().unwrap().decision.as_deref(), Some("no"));
    let _ = std::fs::remove_dir_all(&run.root);
}

#[test]
fn only_files_inside_the_project_are_reached() {
    let root = std::env::temp_dir().join(format!("clarity-acp-inside-{}", std::process::id()));
    std::fs::create_dir_all(root.join("design")).unwrap();
    std::fs::write(root.join("design/Main.cxaml"), "<Window />").unwrap();
    assert_eq!(clarity_acp::inside(&root, &root.join("design/Main.cxaml")).unwrap(), root.canonicalize().unwrap().join("design/Main.cxaml"), "a file that is there, as it is");
    assert!(clarity_acp::inside(&root, &root.join("new/folder/file.md")).is_ok(), "a new folder inside");
    assert!(clarity_acp::inside(&root, &root.join("../x")).is_err());
    assert!(clarity_acp::inside(&root, Path::new("relative.md")).is_err(), "ACP paths are absolute");
    assert!(clarity_acp::inside(&root, Path::new("/etc/passwd")).is_err());
    let _ = std::fs::remove_dir_all(&root);
}

/// A program that is not signed in says so in ACP's own words, and the
/// client tells it apart, so the panel offers Sign in only then.
#[tokio::test]
async fn not_signed_in_is_told_apart() {
    let run = started("signin", Decision::Deny).await;
    assert_eq!(run.agent.new_session(Vec::new()).await, Err(Failure::NotSignedIn));
    run.agent.sign_in("chatgpt").await.unwrap();
    assert!(run.agent.new_session(Vec::new()).await.is_ok());
    let _ = std::fs::remove_dir_all(&run.root);
}
