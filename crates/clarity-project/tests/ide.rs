//! The project the Clarity IDE has open, kept in `ide.json` in Clarity's
//! own folder, so `clarity mcp` works on that project and follows the IDE
//! when it opens another (docs/MCP-PLAN.md, "Which project").

use std::path::{Path, PathBuf};

use clarity_project::ide::{self, Ide};

fn home(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ide-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// This test's own process, which is surely running.
fn here() -> Ide {
    Ide::current().expect("the running test can be found")
}

fn project(dir: &Path, name: &str) -> PathBuf {
    let file = dir.join(format!("{name}.clarity"));
    std::fs::write(&file, format!("project {name}\n    stack rust desktop\n")).unwrap();
    file
}

#[test]
fn the_open_project_is_read_back() {
    let dir = home("read");
    let shop = project(&dir, "Shop");
    let file = dir.join("ide.json");
    let ide = here();
    let settings = serde_json::json!({ "openProject": shop, "idePid": ide.pid, "ideStarted": ide.started });
    std::fs::write(&file, settings.to_string()).unwrap();
    assert_eq!(ide::open_project_in(&file), Some(shop));
}

#[test]
fn a_project_whose_ide_has_stopped_is_not_open() {
    let dir = home("stopped");
    let shop = project(&dir, "Shop");
    let file = dir.join("ide.json");
    // Far above any process id Linux, Windows or macOS hands out.
    ide::set_open_project_in(&file, &shop, &Ide { pid: 0x7fff_fff0, started: 1 }).unwrap();
    assert_eq!(ide::open_project_in(&file), None);
}

#[test]
fn a_process_id_used_again_by_another_program_is_not_the_ide() {
    let dir = home("reused");
    let shop = project(&dir, "Shop");
    let file = dir.join("ide.json");
    let ide = here();
    ide::set_open_project_in(&file, &shop, &Ide { pid: ide.pid, started: ide.started.wrapping_sub(3600) }).unwrap();
    assert_eq!(ide::open_project_in(&file), None);
}

#[test]
fn a_project_recorded_without_its_ide_is_not_open() {
    let dir = home("older");
    let shop = project(&dir, "Shop");
    let file = dir.join("ide.json");
    std::fs::write(&file, serde_json::json!({ "openProject": shop }).to_string()).unwrap();
    assert_eq!(ide::open_project_in(&file), None);
}

#[test]
fn nothing_is_open_without_the_settings_file_or_the_key() {
    let dir = home("missing");
    let file = dir.join("ide.json");
    assert_eq!(ide::open_project_in(&file), None, "no settings file");
    std::fs::write(&file, r#"{ "other": 1 }"#).unwrap();
    assert_eq!(ide::open_project_in(&file), None, "no key");
    std::fs::write(&file, r#"{ "openProject": null }"#).unwrap();
    assert_eq!(ide::open_project_in(&file), None, "key cleared");
    std::fs::write(&file, "not json").unwrap();
    assert_eq!(ide::open_project_in(&file), None, "unreadable settings");
}

#[test]
fn a_project_file_that_is_gone_is_not_open() {
    let dir = home("gone");
    let shop = project(&dir, "Shop");
    let file = dir.join("ide.json");
    ide::set_open_project_in(&file, &shop, &here()).unwrap();
    std::fs::remove_file(&shop).unwrap();
    assert_eq!(ide::open_project_in(&file), None);
}

#[test]
fn opening_changes_only_its_own_key() {
    let dir = home("merge");
    let shop = project(&dir, "Shop");
    let file = dir.join("ide.json");
    std::fs::write(&file, r#"{ "theme": "dark", "recent": [1, 2] }"#).unwrap();
    ide::set_open_project_in(&file, &shop, &here()).unwrap();
    let kept: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(kept["theme"], "dark");
    assert_eq!(kept["recent"], serde_json::json!([1, 2]));
    assert_eq!(ide::open_project_in(&file), Some(shop));
}

#[test]
fn opening_another_project_switches_to_it() {
    let dir = home("switch");
    let (shop, notes) = (project(&dir, "Shop"), project(&dir, "Notes"));
    let file = dir.join("ide.json");
    ide::set_open_project_in(&file, &shop, &here()).unwrap();
    ide::set_open_project_in(&file, &notes, &here()).unwrap();
    assert_eq!(ide::open_project_in(&file), Some(notes));
}

#[test]
fn the_folder_is_made_when_it_is_not_there() {
    let dir = home("folder");
    let shop = project(&dir, "Shop");
    let file = dir.join("clarity-home").join("ide.json");
    ide::set_open_project_in(&file, &shop, &here()).unwrap();
    assert_eq!(ide::open_project_in(&file), Some(shop));
}

#[test]
fn closing_removes_the_key_and_keeps_the_rest() {
    let dir = home("close");
    let shop = project(&dir, "Shop");
    let file = dir.join("ide.json");
    std::fs::write(&file, r#"{ "theme": "dark" }"#).unwrap();
    ide::set_open_project_in(&file, &shop, &here()).unwrap();
    ide::close_project_in(&file, &shop, here().pid).unwrap();
    let kept: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(kept, serde_json::json!({ "theme": "dark" }), "the IDE's process goes with the project");
    assert_eq!(ide::open_project_in(&file), None);
}

#[test]
fn closing_leaves_a_project_another_ide_opened_since() {
    let dir = home("two");
    let (shop, notes) = (project(&dir, "Shop"), project(&dir, "Notes"));
    let file = dir.join("ide.json");
    ide::set_open_project_in(&file, &shop, &here()).unwrap();
    ide::set_open_project_in(&file, &notes, &here()).unwrap();
    ide::close_project_in(&file, &shop, here().pid).unwrap();
    assert_eq!(ide::open_project_in(&file), Some(notes));
}

#[test]
fn settings_that_cannot_be_read_are_not_overwritten() {
    let dir = home("broken");
    let shop = project(&dir, "Shop");
    let file = dir.join("ide.json");
    std::fs::write(&file, "{ half").unwrap();
    assert!(ide::set_open_project_in(&file, &shop, &here()).is_err());
    assert!(ide::close_project_in(&file, &shop, here().pid).is_err());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{ half");
}

#[test]
fn the_settings_file_is_in_claritys_own_folder() {
    assert_eq!(ide::file(), clarity_project::clarity_home().join("ide.json"));
}

#[test]
fn closing_leaves_the_same_project_another_ide_opened_since() {
    let dir = home("same");
    let shop = project(&dir, "Shop");
    let file = dir.join("ide.json");
    let other = here();
    ide::set_open_project_in(&file, &shop, &other).unwrap();
    ide::close_project_in(&file, &shop, other.pid.wrapping_add(1)).unwrap();
    assert_eq!(ide::open_project_in(&file), Some(shop));
}

#[test]
fn outside_agents_are_let_in_until_the_person_turns_them_off() {
    let dir = home("outside");
    let file = dir.join("ide.json");
    assert!(ide::outside_agents_in(&file), "no settings file: on");
    std::fs::write(&file, r#"{ "theme": "dark" }"#).unwrap();
    assert!(ide::outside_agents_in(&file), "no key: on");
    ide::set_outside_agents_in(&file, false).unwrap();
    assert!(!ide::outside_agents_in(&file));
    let kept: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(kept["theme"], "dark", "the other keys stay");
    ide::set_outside_agents_in(&file, true).unwrap();
    assert!(ide::outside_agents_in(&file));
}

#[test]
fn the_switch_is_not_written_over_settings_that_cannot_be_read() {
    let dir = home("outside-broken");
    let file = dir.join("ide.json");
    std::fs::write(&file, "{ half").unwrap();
    assert!(ide::set_outside_agents_in(&file, false).is_err());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{ half");
}
