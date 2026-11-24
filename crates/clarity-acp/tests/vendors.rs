//! The agent programs Clarity's chat panel starts (docs/ledger/acp.md):
//! found by a scan of the PATH and the usual install folders, or at the path
//! the person chose, with the arguments that make them speak ACP.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clarity_acp::vendors::{self, Vendor};

fn folder(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("vendors-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A program file named as the vendor's is on this system, that can be started.
fn fake_program(dir: &Path, vendor: Vendor) -> PathBuf {
    let file = dir.join(vendors::program_names(vendor)[0]);
    std::fs::write(&file, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    file
}

fn folders(list: &[&Path]) -> Vec<PathBuf> {
    list.iter().map(|f| f.to_path_buf()).collect()
}

#[test]
fn a_vendor_is_named_by_its_connection_kind_in_any_case() {
    assert_eq!(Vendor::of_kind("codex"), Some(Vendor::Codex));
    assert_eq!(Vendor::of_kind("Copilot"), Some(Vendor::Copilot));
    assert_eq!(Vendor::of_kind(" COPILOT "), Some(Vendor::Copilot));
    assert_eq!(Vendor::of_kind("anthropic"), None);
    assert_eq!(Vendor::Codex.name(), "Codex");
    assert_eq!(Vendor::Copilot.name(), "GitHub Copilot");
    for vendor in [Vendor::Codex, Vendor::Copilot] {
        assert!(vendor.install_page().starts_with("https://"));
    }
}

#[test]
fn a_vendor_is_known_by_its_key() {
    assert_eq!(Vendor::ALL, [Vendor::Codex, Vendor::Copilot]);
    for vendor in Vendor::ALL {
        assert_eq!(Vendor::of_key(vendor.key()), Some(vendor));
        assert_eq!(Vendor::of_kind(vendor.key()), Some(vendor), "an old vault connection's kind is the key");
    }
    assert_eq!(Vendor::Codex.key(), "codex");
    assert_eq!(Vendor::Copilot.key(), "copilot");
    // A key is exact: a connection named Codex in the vault is not taken for it.
    assert_eq!(Vendor::of_key("Codex"), None);
    assert_eq!(Vendor::of_key("anthropic"), None);
}

#[test]
fn the_program_is_found_by_the_scan_with_its_acp_arguments() {
    let empty = folder("path-empty");
    let tools = folder("path-tools");
    let copilot = fake_program(&tools, Vendor::Copilot);
    let found = vendors::find_in(Vendor::Copilot, None, None, &folders(&[&empty, &tools])).unwrap();
    assert_eq!(found.command, copilot);
    assert_eq!(found.args, vec!["--acp".to_string(), "--stdio".to_string()]);
    assert!(found.env.is_empty());
}

/// The scan gives each agent installed, with the first copy in the folders'
/// order; an agent not installed is not in it.
#[test]
fn a_scan_gives_each_agent_installed_with_its_program() {
    let first = folder("scan-first");
    let second = folder("scan-second");
    let empty = folder("scan-empty");
    assert!(vendors::scan_in(&folders(&[&empty])).is_empty(), "nothing installed");
    assert!(vendors::scan_in(&[]).is_empty());
    let copilot = fake_program(&second, Vendor::Copilot);
    let found = vendors::scan_in(&folders(&[&empty, &first, &second]));
    assert_eq!(found, [vendors::Found { vendor: Vendor::Copilot, program: copilot.clone() }]);
    let codex_second = fake_program(&second, Vendor::Codex);
    let codex_first = fake_program(&first, Vendor::Codex);
    let found = vendors::scan_in(&folders(&[&first, &second]));
    assert_eq!(found, [vendors::Found { vendor: Vendor::Codex, program: codex_first.clone() }, vendors::Found { vendor: Vendor::Copilot, program: copilot }]);
    // Every copy, for the Environment settings to choose from.
    assert_eq!(vendors::copies_in(Vendor::Codex, &folders(&[&first, &second, &first])), [codex_first, codex_second]);
}

/// Where Codex's ACP adapter and Copilot CLI get installed beside the PATH:
/// npm's global folders (its default, a prefix of the person's, nvm's
/// versions, Volta, pnpm), WinGet's and Scoop's links on Windows, and the
/// install script's and Homebrew's folders on Linux and macOS.
#[test]
fn the_agents_install_folders_on_each_system() {
    let home = folder("home");
    for version in ["v20.11.0", "v22.3.0", "v9.0.0"] {
        std::fs::create_dir_all(home.join(".nvm/versions/node").join(version).join("bin")).unwrap();
    }
    let none = |_: &str| None::<OsString>;
    let linux = vendors::install_folders("linux", &none, Some(&home));
    for wanted in [home.join(".npm-global/bin"), home.join(".local/bin"), home.join(".volta/bin"), home.join(".local/share/pnpm"), home.join(".bun/bin"), PathBuf::from("/usr/local/bin")] {
        assert!(linux.contains(&wanted), "{wanted:?} in {linux:?}");
    }
    // nvm's versions, newest first.
    let nvm: Vec<&PathBuf> = linux.iter().filter(|f| f.starts_with(home.join(".nvm"))).collect();
    assert_eq!(nvm, [&home.join(".nvm/versions/node/v22.3.0/bin"), &home.join(".nvm/versions/node/v20.11.0/bin"), &home.join(".nvm/versions/node/v9.0.0/bin")]);
    let macos = vendors::install_folders("macos", &none, Some(&home));
    assert!(macos.contains(&home.join("Library/pnpm")), "{macos:?}");
    assert!(macos.contains(&PathBuf::from("/opt/homebrew/bin")), "{macos:?}");
    assert!(!macos.contains(&home.join(".local/share/pnpm")), "{macos:?}");

    // A prefix of the person's, by the variables npm, nvm and pnpm set.
    let vars = |name: &str| match name {
        "NPM_CONFIG_PREFIX" => Some(OsString::from("/opt/npm")),
        "NVM_BIN" => Some(OsString::from("/opt/nvm/bin")),
        "PNPM_HOME" => Some(OsString::from("/opt/pnpm")),
        "VOLTA_HOME" => Some(OsString::from("/opt/volta")),
        _ => None,
    };
    let set = vendors::install_folders("linux", &vars, Some(&home));
    for wanted in ["/opt/npm/bin", "/opt/nvm/bin", "/opt/pnpm", "/opt/volta/bin"] {
        assert!(set.contains(&PathBuf::from(wanted)), "{wanted} in {set:?}");
    }

    let windows_vars = |name: &str| match name {
        "APPDATA" => Some(OsString::from("C:\\Users\\p\\AppData\\Roaming")),
        "LOCALAPPDATA" => Some(OsString::from("C:\\Users\\p\\AppData\\Local")),
        "ProgramFiles" => Some(OsString::from("C:\\Program Files")),
        "NPM_CONFIG_PREFIX" => Some(OsString::from("D:\\npm")),
        "NVM_SYMLINK" => Some(OsString::from("C:\\nvm4w\\nodejs")),
        _ => None,
    };
    let windows = vendors::install_folders("windows", &windows_vars, Some(&home));
    for wanted in [
        PathBuf::from("C:\\Users\\p\\AppData\\Roaming").join("npm"),
        PathBuf::from("D:\\npm"),
        PathBuf::from("C:\\nvm4w\\nodejs"),
        PathBuf::from("C:\\Program Files").join("nodejs"),
        PathBuf::from("C:\\Users\\p\\AppData\\Local").join("Microsoft").join("WinGet").join("Links"),
        PathBuf::from("C:\\Users\\p\\AppData\\Local").join("pnpm"),
        home.join("scoop").join("shims"),
        home.join(".volta").join("bin"),
    ] {
        assert!(windows.contains(&wanted), "{wanted:?} in {windows:?}");
    }
    assert!(!windows.iter().any(|f| f.starts_with("/usr")), "{windows:?}");
    // No folder twice.
    for list in [&linux, &macos, &windows] {
        let mut seen = list.to_vec();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), list.len(), "{list:?}");
    }
}

/// What the panel lists: each agent with the program it starts, the one
/// chosen in the settings first, else the one the scan finds; a chosen
/// program that is gone counts as not chosen, as `find` would refuse it.
#[test]
fn the_agents_installed_count_the_chosen_program() {
    let scanned = folder("installed-scanned");
    let chosen_dir = folder("installed-chosen");
    let codex = fake_program(&scanned, Vendor::Codex);
    let chosen = fake_program(&chosen_dir, Vendor::Copilot);
    let none = |_: Vendor| None::<PathBuf>;
    assert_eq!(vendors::installed_in(&none, &folders(&[&scanned])), [vendors::Found { vendor: Vendor::Codex, program: codex.clone() }]);
    let copilot_chosen = |v: Vendor| (v == Vendor::Copilot).then(|| chosen.clone());
    assert_eq!(
        vendors::installed_in(&copilot_chosen, &folders(&[&scanned])),
        [vendors::Found { vendor: Vendor::Codex, program: codex.clone() }, vendors::Found { vendor: Vendor::Copilot, program: chosen.clone() }]
    );
    let gone = |v: Vendor| (v == Vendor::Codex).then(|| scanned.join("gone"));
    assert_eq!(vendors::installed_in(&gone, &folders(&[&scanned])), [vendors::Found { vendor: Vendor::Codex, program: codex }]);
}

#[test]
fn a_missing_program_names_where_to_install_it() {
    let empty = folder("missing");
    for vendor in Vendor::ALL {
        let why = vendors::find_in(vendor, None, None, &folders(&[&empty])).unwrap_err();
        assert!(why.contains(vendor.install_page()), "{why}");
        assert!(why.contains(vendor.name()), "{why}");
    }
    assert!(vendors::find_in(Vendor::Codex, None, None, &[]).is_err(), "no folders at all");
}

/// The path the person chose in Clarity settings > Environment comes before
/// the scan; the command of an old vault connection before both, and a
/// command there that cannot start is refused, as it always was.
#[test]
fn a_chosen_path_is_used_as_it_is() {
    let dir = folder("settings");
    let chosen = fake_program(&dir, Vendor::Codex);
    let scanned = folder("settings-scanned");
    fake_program(&scanned, Vendor::Codex);
    let found = vendors::find_in(Vendor::Codex, None, Some(&chosen), &folders(&[&scanned])).unwrap();
    assert_eq!(found.command, chosen);
    // Started asking before it changes anything (its ACP mode read-only), never in its default mode.
    assert_eq!(found.env, vec![("INITIAL_AGENT_MODE".to_string(), "read-only".to_string())]);
    assert!(found.args.is_empty());
    // A chosen program that is gone falls back to the scan, as a chosen build tool does.
    let gone = dir.join("nowhere");
    assert_eq!(vendors::find_in(Vendor::Codex, None, Some(&gone), &folders(&[&scanned])).unwrap().command, scanned.join(vendors::program_names(Vendor::Codex)[0]));
    assert!(vendors::find_in(Vendor::Codex, None, Some(&gone), &[]).is_err());

    let vault = folder("vault");
    let old = fake_program(&vault, Vendor::Codex);
    assert_eq!(vendors::find_in(Vendor::Codex, Some(&old), Some(&chosen), &folders(&[&scanned])).unwrap().command, old);
    let why = vendors::find_in(Vendor::Codex, Some(&gone), Some(&chosen), &[]).unwrap_err();
    assert!(why.contains("its connection in the vault"), "{why}");
}

/// A folder of the same name, or (on Linux and macOS) a file that cannot be
/// started, is not the program.
#[test]
fn only_a_program_that_can_be_started_counts() {
    let dir = folder("not-program");
    std::fs::create_dir_all(dir.join(vendors::program_names(Vendor::Copilot)[0])).unwrap();
    assert!(vendors::find_in(Vendor::Copilot, None, None, &folders(&[&dir])).is_err());
    assert!(vendors::scan_in(&folders(&[&dir])).is_empty());
    #[cfg(unix)]
    {
        let plain = folder("not-executable");
        std::fs::write(plain.join(vendors::program_names(Vendor::Copilot)[0]), "text").unwrap();
        assert!(vendors::find_in(Vendor::Copilot, None, None, &folders(&[&plain])).is_err());
    }
}

/// Copilot takes MCP servers over ACP only through HTTP (its log: "Rejecting
/// non-http/sse MCP server"), so Clarity's tools go to it at its start, as
/// `--additional-mcp-config`, allowed as a whole since they run nothing.
/// Codex takes them with the session.
#[test]
fn copilot_takes_clarity_s_tools_at_its_start() {
    let tools = clarity_acp::ToolServer { name: "clarity".into(), command: PathBuf::from("/opt/clarity/clarity"), args: vec!["mcp".into(), "--turn".into(), "/tmp/turn.jsonl".into()] };
    assert_eq!(Vendor::Codex.tools_at_start(&tools), None);
    let args = Vendor::Copilot.tools_at_start(&tools).expect("Copilot's start arguments");
    assert_eq!(args[0], "--additional-mcp-config");
    let config: serde_json::Value = serde_json::from_str(&args[1]).unwrap();
    let server = &config["mcpServers"]["clarity"];
    assert_eq!(server["type"], "local");
    assert_eq!(server["command"], "/opt/clarity/clarity");
    assert_eq!(server["args"], serde_json::json!(["mcp", "--turn", "/tmp/turn.jsonl"]));
    assert_eq!(server["tools"], serde_json::json!(["*"]));
    // Copilot hands a server only PATH; Clarity's tools find the open project through HOME.
    if let Some(home) = std::env::var_os("HOME") {
        assert_eq!(server["env"]["HOME"], home.to_string_lossy().as_ref());
    }
    assert_eq!(&args[2..], ["--allow-tool=clarity"]);
}

/// Each vendor signs in its own way, by name: Codex with its ChatGPT method
/// (`chat-gpt`, never the `api-key` it lists first), Copilot with its login command.
#[test]
fn each_vendor_signs_in_by_name() {
    assert_eq!(Vendor::Codex.sign_in(), vendors::SignIn::Acp("chat-gpt"));
    assert_eq!(Vendor::Copilot.sign_in(), vendors::SignIn::Command(&["login", "--web-flow"]));
}

/// A missing program says where its path is chosen: Clarity settings >
/// Environment, not the vault, which keeps only secrets.
#[test]
fn a_missing_program_points_at_the_environment_settings() {
    let err = vendors::find_in(Vendor::Codex, None, None, &[]).unwrap_err();
    assert!(err.contains("Clarity settings > Environment"), "{err}");
    assert!(!err.contains("vault"), "{err}");
}

/// What the person may choose per chat: only choices that keep the program
/// asking before it changes anything, so every AI changes the project through
/// Clarity's tools. Model and effort pass; a mode that lets it write by itself
/// (Codex's workspace-write, agent, full access; Copilot's autopilot and
/// allow-all) never reaches the panel, and an option no list names is left out.
#[test]
fn only_safe_choices_reach_the_panel() {
    use clarity_acp::{Choice, ChoiceValue};
    let choice = |id: &str, values: &[&str]| Choice {
        id: id.into(),
        name: id.into(),
        current: values[0].into(),
        values: values.iter().map(|v| ChoiceValue { value: v.to_string(), name: v.to_string() }).collect(),
    };
    let codex = [
        choice("mode", &["read-only", "workspace-write", "agent", "agent-full-access"]),
        choice("collaboration_mode", &["default", "plan"]),
        choice("model", &["gpt-6.1-sol", "gpt-6-astra"]),
        choice("reasoning_effort", &["low", "high"]),
        choice("fast-mode", &["off", "on"]),
    ];
    let safe = Vendor::Codex.safe_choices(&codex);
    assert_eq!(safe.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["collaboration_mode", "model", "reasoning_effort"]);

    const AGENT: &str = "https://agentclientprotocol.com/protocol/session-modes#agent";
    const PLAN: &str = "https://agentclientprotocol.com/protocol/session-modes#plan";
    const AUTOPILOT: &str = "https://agentclientprotocol.com/protocol/session-modes#autopilot";
    let copilot = [choice("mode", &[AGENT, PLAN, AUTOPILOT]), choice("model", &["auto", "auto", "gpt-6"]), choice("allow_all", &["off", "on"])];
    let safe = Vendor::Copilot.safe_choices(&copilot);
    assert_eq!(safe.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["mode", "model"]);
    assert_eq!(safe[0].values.iter().map(|v| v.value.as_str()).collect::<Vec<_>>(), [AGENT, PLAN], "no autopilot");
    assert_eq!(safe[1].values.iter().map(|v| v.value.as_str()).collect::<Vec<_>>(), ["auto", "gpt-6"], "a value told twice shows once");
    // A mode whose current value is not safe is not offered at all.
    let mut loose = copilot.clone();
    loose[0].current = AUTOPILOT.into();
    assert!(Vendor::Copilot.safe_choices(&loose).iter().all(|c| c.id != "mode"));
}
