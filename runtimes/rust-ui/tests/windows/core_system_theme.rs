//! The system's light or dark choice, read the way each system answers it
//! (as C#'s SystemTheme), for the desktop where the window system says
//! nothing (winit on X11): never hard to run, light when nothing answers.

use clarity_ui::system_theme::{self, System};

fn answers<'a>(given: &'a [(&'a str, &'a str)]) -> impl Fn(&str, &[&str]) -> Option<String> + 'a {
    move |program, args| {
        let asked = format!("{program} {}", args.first().copied().unwrap_or_default());
        given.iter().find(|(k, _)| *k == asked).map(|(_, v)| v.to_string())
    }
}

#[test]
fn linux_asks_the_desktop_portal_first() {
    assert_eq!(system_theme::portal("(<<uint32 1>>,)\n"), Some(true));
    assert_eq!(system_theme::portal("(<<uint32 2>>,)"), Some(false));
    assert_eq!(system_theme::portal("(<<uint32 0>>,)"), None);
    assert_eq!(system_theme::portal("Error: GDBus.Error"), None);
    assert!(system_theme::dark(System::Linux, None, &answers(&[("gdbus call", "(<<uint32 1>>,)")])));
}

#[test]
fn linux_without_the_portal_asks_gnomes_settings() {
    assert_eq!(system_theme::color_scheme("'prefer-dark'\n"), Some(true));
    assert_eq!(system_theme::color_scheme("'prefer-light'"), Some(false));
    assert_eq!(system_theme::color_scheme("'default'"), None);
    assert_eq!(system_theme::gtk_theme("'Adwaita-dark'"), Some(true));
    assert_eq!(system_theme::gtk_theme("'Adwaita'"), None);
    assert!(system_theme::dark(System::Linux, None, &answers(&[("gsettings get", "'prefer-dark'")])));
    assert!(!system_theme::dark(System::Linux, None, &answers(&[])));
}

#[test]
fn gtk_theme_in_the_environment_goes_first_on_linux() {
    // As GTK programs honour it: `GTK_THEME=Adwaita:dark`.
    assert!(system_theme::dark(System::Linux, Some("Adwaita:dark"), &answers(&[("gdbus call", "(<<uint32 2>>,)")])));
    assert!(!system_theme::dark(System::Linux, Some("Adwaita"), &answers(&[])));
}

#[test]
fn windows_and_macos_read_their_own_settings() {
    let dark = "\r\n    AppsUseLightTheme    REG_DWORD    0x0\r\n";
    assert_eq!(system_theme::registry(dark), Some(true));
    assert_eq!(system_theme::registry(&dark.replace("0x0", "0x1")), Some(false));
    assert_eq!(system_theme::registry("ERROR"), None);
    assert!(system_theme::dark(System::Windows, None, &answers(&[("reg query", dark)])));
    assert!(system_theme::dark(System::Macos, None, &answers(&[("defaults read", "Dark\n")])));
    assert!(!system_theme::dark(System::Macos, None, &answers(&[])));
    assert!(!system_theme::dark(System::Other, None, &answers(&[])));
}
