//! running.json: one per app, beside its settings, readable by the person only.

use clarity_app_mcp::discovery::{self, Running};

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-app-mcp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_running_app_is_found_by_its_own_file() {
    let dir = folder("found");
    let file = discovery::file_in(&dir);
    let running = Running::new("shop", "/run/user/1000/shop.sock").unwrap();
    assert_eq!(running.token.len(), 48);
    assert_ne!(running.token, Running::new("shop", "x").unwrap().token, "a new token each time");
    discovery::write(&file, &running).unwrap();
    assert_eq!(discovery::read(&file), Some(running));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600, "the person's only");
    }
    discovery::remove(&file);
    assert!(!file.exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_app_removes_only_its_own_file() {
    let dir = folder("other");
    let file = discovery::file_in(&dir);
    let other = Running { pid: std::process::id().wrapping_add(1), ..Running::new("shop", "x").unwrap() };
    discovery::write(&file, &other).unwrap();
    discovery::remove(&file);
    assert_eq!(discovery::read(&file), Some(other), "another process's file stays");
    std::fs::write(&file, "{half").unwrap();
    assert_eq!(discovery::read(&file), None);
    let _ = std::fs::remove_dir_all(dir);
}
