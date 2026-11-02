//! The system's own event for a change of its light or dark choice, which
//! `system_theme::follow` listens to instead of reading the choice every few
//! seconds. On Linux the desktop portal's SettingChanged signal for
//! org.freedesktop.appearance color-scheme, over the session's D-Bus (spoken
//! as the file dialogs speak it, with no crate of its own). On Windows a
//! change of the registry value AppsUseLightTheme, the same change that
//! sends WM_SETTINGCHANGE "ImmersiveColorSet" to every window, heard without
//! a window procedure of its own. macOS gives none here, so its choice is
//! read every few seconds.

use crate::system_theme::{Heard, Listening};

/// Listens to the system's change event, calling `heard` each time; none
/// where the system has none, an error where it cannot be listened to now.
pub fn listen(heard: Heard) -> Result<Option<Listening>, String> {
    #[cfg(target_os = "linux")]
    {
        portal::listen(heard)
    }
    #[cfg(windows)]
    {
        registry::listen(heard)
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = heard;
        Ok(None)
    }
}

/// Whether a portal's SettingChanged names the color scheme: its namespace and key.
pub fn is_color_scheme(namespace: &str, key: &str) -> bool {
    namespace == "org.freedesktop.appearance" && key == "color-scheme"
}

#[cfg(target_os = "linux")]
mod portal {
    use std::io::Write;
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use super::super::files::portal::{connect, login, method_call, read_message, Dbus, Reader};
    use super::is_color_scheme;
    use crate::system_theme::{Heard, Listening};

    /// The bus's socket, shut when listening stops so the thread reading it ends.
    struct Stop(UnixStream);

    impl Drop for Stop {
        fn drop(&mut self) {
            let _ = self.0.shutdown(Shutdown::Both);
        }
    }

    pub fn listen(heard: Heard) -> Result<Option<Listening>, String> {
        // No session bus at all (a bare X server, a service): nothing to listen to.
        let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
        let address = std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_default();
        if address.is_empty() && (runtime.is_empty() || !std::path::Path::new(&runtime).join("bus").exists()) {
            return Ok(None);
        }
        let mut stream = connect()?;
        // A bus that does not answer within a few seconds: the choice is read every few seconds instead.
        stream.set_read_timeout(Some(Duration::from_secs(3))).map_err(|e| e.to_string())?;
        login(&mut stream)?;
        let bus = "org.freedesktop.DBus";
        let desktop = "/org/freedesktop/portal/desktop";
        stream.write_all(&method_call(1, bus, "/org/freedesktop/DBus", bus, "Hello", "", &[])).map_err(|e| e.to_string())?;
        let rule = format!("type='signal',interface='org.freedesktop.portal.Settings',member='SettingChanged',path='{desktop}'");
        stream.write_all(&method_call(2, bus, "/org/freedesktop/DBus", bus, "AddMatch", "s", &[Dbus::Str(rule)])).map_err(|e| e.to_string())?;
        // The portal is asked for the setting once: a bus without it has no event to give.
        let read = [Dbus::Str("org.freedesktop.appearance".into()), Dbus::Str("color-scheme".into())];
        stream.write_all(&method_call(3, "org.freedesktop.portal.Desktop", desktop, "org.freedesktop.portal.Settings", "Read", "ss", &read)).map_err(|e| e.to_string())?;
        loop {
            let message = read_message(&mut stream).ok_or("the bus did not answer")?;
            if message.reply_to == Some(3) {
                // 2 is a reply, 3 an error: no portal, or one without the appearance settings.
                if message.kind != 2 {
                    return Ok(None);
                }
                break;
            }
        }
        stream.set_read_timeout(None).map_err(|e| e.to_string())?;
        let stop = Stop(stream.try_clone().map_err(|e| e.to_string())?);
        std::thread::spawn(move || {
            while let Some(message) = read_message(&mut stream) {
                if message.kind != 4 || message.member.as_deref() != Some("SettingChanged") {
                    continue;
                }
                let mut reader = Reader::new(&message.body, 0);
                if let (Some(Dbus::Str(namespace)), Some(Dbus::Str(key))) = (reader.value("s"), reader.value("s")) {
                    if is_color_scheme(&namespace, &key) {
                        heard();
                    }
                }
            }
        });
        Ok(Some(Box::new(stop)))
    }
}

#[cfg(windows)]
mod registry {
    use std::ffi::c_void;
    use std::sync::Arc;

    use crate::system_theme::{Heard, Listening};

    type Handle = isize;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(key: Handle, sub_key: *const u16, options: u32, access: u32, result: *mut Handle) -> i32;
        fn RegNotifyChangeKeyValue(key: Handle, watch_subtree: i32, filter: u32, event: Handle, asynchronous: i32) -> i32;
        fn RegCloseKey(key: Handle) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateEventW(attributes: *mut c_void, manual_reset: i32, initial: i32, name: *const u16) -> Handle;
        fn SetEvent(event: Handle) -> i32;
        fn WaitForMultipleObjects(count: u32, handles: *const Handle, wait_all: i32, milliseconds: u32) -> u32;
        fn CloseHandle(handle: Handle) -> i32;
    }

    const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    // HKEY_CURRENT_USER, sign-extended as Windows' headers make it.
    const CURRENT_USER: Handle = 0x8000_0001_u32 as i32 as Handle;
    const KEY_READ: u32 = 0x20019;
    const CHANGE_LAST_SET: u32 = 0x4;
    const INFINITE: u32 = 0xFFFF_FFFF;

    /// An event of Windows', closed when the last holder lets it go.
    struct Event(Handle);

    // A Windows event is used from any thread.
    unsafe impl Send for Event {}
    unsafe impl Sync for Event {}

    impl Drop for Event {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    /// Stops the waiting thread when listening stops.
    struct Stop(Arc<Event>);

    impl Drop for Stop {
        fn drop(&mut self) {
            unsafe {
                SetEvent(self.0 .0);
            }
        }
    }

    /// A thread of its own that waits for the Personalize key to change, until stopped; none when the key cannot be opened.
    pub fn listen(heard: Heard) -> Result<Option<Listening>, String> {
        let name: Vec<u16> = PERSONALIZE.encode_utf16().chain(std::iter::once(0)).collect();
        let mut key: Handle = 0;
        if unsafe { RegOpenKeyExW(CURRENT_USER, name.as_ptr(), 0, KEY_READ, &mut key) } != 0 {
            return Ok(None);
        }
        let changed = unsafe { CreateEventW(std::ptr::null_mut(), 0, 0, std::ptr::null()) };
        let stop = unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null()) };
        if changed == 0 || stop == 0 {
            unsafe {
                if changed != 0 {
                    CloseHandle(changed);
                }
                if stop != 0 {
                    CloseHandle(stop);
                }
                RegCloseKey(key);
            }
            return Err("no event to wait on".to_string());
        }
        let (changed, stop) = (Arc::new(Event(changed)), Arc::new(Event(stop)));
        let stopping = stop.clone();
        let key = key as usize;
        std::thread::spawn(move || {
            let key = key as Handle;
            let handles = [changed.0, stop.0];
            // The notice is for one change: it is asked for again after each.
            while unsafe { RegNotifyChangeKeyValue(key, 0, CHANGE_LAST_SET, changed.0, 1) } == 0 && unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, INFINITE) } == 0 {
                heard();
            }
            unsafe {
                RegCloseKey(key);
            }
        });
        Ok(Some(Box::new(Stop(stopping))))
    }
}
