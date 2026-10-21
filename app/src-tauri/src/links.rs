//! A link of a markdown part, followed in the designer's preview: it opens in
//! the person's browser, as in a generated application. Only a web or a
//! mail address opens, as on every stack.

/// Whether a link may open: a web or a mail address.
fn opens(address: &str) -> bool {
    ["http://", "https://", "mailto:"].iter().any(|scheme| address.starts_with(scheme))
}

/// Opens `address` with `open`, once it is one a link may open.
fn open_with(address: &str, open: impl FnOnce(&str) -> std::io::Result<()>) -> Result<(), String> {
    if !opens(address) {
        return Err(format!("a link opens only an http, https or mailto address, not {address}"));
    }
    open(address).map_err(|err| format!("cannot open {address}: {err}"))
}

/// Starts the system's own way of opening an address.
fn in_browser(address: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let opened = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", address]).spawn();
    #[cfg(target_os = "macos")]
    let opened = std::process::Command::new("open").arg(address).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let opened = std::process::Command::new("xdg-open").arg(address).spawn();
    opened.map(|_| ())
}

#[tauri::command]
pub fn open_link(address: String) -> Result<(), String> {
    open_with(&address, in_browser)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_web_or_mail_address_opens() {
        let mut opened = Vec::new();
        let mut note = |a: &str| {
            opened.push(a.to_string());
            Ok(())
        };
        assert_eq!(open_with("https://example.com/rules", &mut note), Ok(()));
        assert_eq!(open_with("mailto:help@example.com", &mut note), Ok(()));
        assert_eq!(
            open_with("file:///etc/passwd", |_| panic!("not opened")),
            Err("a link opens only an http, https or mailto address, not file:///etc/passwd".to_string())
        );
        assert_eq!(opened, ["https://example.com/rules", "mailto:help@example.com"]);
    }
}
