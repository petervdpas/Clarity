//! Custom parts written in Rust (DESIGN 5.19 and 6.5): a part is a crate of
//! the project, which implements the trait Clarity writes from its
//! declaration; the program registers each kind of part, and this host runs
//! them. It makes a native child window for each part where the screen
//! says the part is, calls its `frame` about 60 times a second, and passes
//! values, hears and tells. Everything runs on a thread of the host's own:
//! a part is made there and lives there, so it need not be `Send`.
//!
//! The part's crate does not depend on this one: the program's glue gives
//! it the window as its own type, which carries the raw handle wgpu, Bevy and
//! softbuffer take.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub use chrono::{NaiveDate, NaiveDateTime};
pub use rust_decimal::Decimal;
use serde::Serialize;
use serde::de::DeserializeOwned;
pub use serde_json::Value;

/// How long between frames: about 60 a second.
const FRAME: Duration = Duration::from_micros(16_667);

/// What a part told: its window's number, the part, the tell and its values.
pub type Told = Box<dyn Fn(u64, &str, &str, &[Value]) + Send + Sync>;

/// A part as the host runs it: the generated glue around the part's own type.
pub trait Part {
    fn resize(&mut self, _width: u32, _height: u32) {}
    fn frame(&mut self) {}
    /// A property by name (`value`, `enabled` and the declared ones).
    fn set(&mut self, name: &str, value: &Value);
    /// A hear by name, with its values.
    fn hear(&mut self, name: &str, values: &[Value]);
}

/// How a kind of part is made: in its window, at its size, telling through its host.
pub type Make = fn(Window, u32, u32, Host) -> Box<dyn Part>;

/// The native window a part draws in: on Linux an X11 window.
#[cfg(not(windows))]
#[derive(Debug, Clone, Copy)]
pub struct Window {
    /// The X11 window.
    pub xid: u64,
    /// The X11 connection the window is on, which a graphics library may share.
    pub display: *mut std::ffi::c_void,
    pub screen: i32,
}

/// The native window a part draws in: on Windows a Win32 child window of
/// the webview's, which the host's thread owns.
#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
pub struct Window {
    /// The child window (an HWND).
    pub hwnd: *mut std::ffi::c_void,
    /// The module its window class belongs to (an HINSTANCE).
    pub hinstance: *mut std::ffi::c_void,
}

/// Where a part tells what happened.
#[derive(Debug, Clone)]
pub struct Host {
    window: u64,
    part: String,
    tells: Sender<Ask>,
}

impl Host {
    /// Tells `tell` with its values: its handler runs with them.
    pub fn tell(&self, tell: &str, values: Vec<Value>) {
        let _ = self.tells.send(Ask::Tell { window: self.window, part: self.part.clone(), tell: tell.to_string(), values });
    }
}

/// A value as a tell carries it.
pub fn value(of: &impl Serialize) -> Value {
    serde_json::to_value(of).unwrap_or(Value::Null)
}

/// A value given to a part as its type; the type's empty value when it does not fit.
pub fn read<T: DeserializeOwned + Default>(value: Option<&Value>) -> T {
    value.and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
}

enum Ask {
    Place { window: u64, part: String, kind: String, parent: u64, x: i32, y: i32, width: i32, height: i32, shown: bool },
    Set { window: u64, part: String, name: String, value: Value },
    Hear { window: u64, part: String, hear: String, values: Vec<Value> },
    Tell { window: u64, part: String, tell: String, values: Vec<Value> },
    Close { window: u64 },
    Wait(Sender<()>),
    Stop,
}

static KINDS: Mutex<Vec<(String, Make)>> = Mutex::new(Vec::new());

struct Running {
    asks: Sender<Ask>,
    thread: Option<JoinHandle<()>>,
}

static RUNNING: OnceLock<Mutex<Option<Running>>> = OnceLock::new();

fn running() -> &'static Mutex<Option<Running>> {
    RUNNING.get_or_init(|| Mutex::new(None))
}

/// Registers a kind of part by its declared name; the program does it for each of its parts.
pub fn register(kind: &str, make: Make) {
    if let Ok(mut kinds) = KINDS.lock()
        && !kinds.iter().any(|(k, _)| k == kind)
    {
        kinds.push((kind.to_string(), make));
    }
}

/// Whether the program has parts.
pub fn present() -> bool {
    KINDS.lock().is_ok_and(|k| !k.is_empty())
}

/// Starts the host; `told` is called on its thread. Gives why parts cannot run.
pub fn start(told: Told) -> Result<(), String> {
    let mut slot = running().lock().map_err(|_| "the part host is not available".to_string())?;
    if slot.is_some() {
        return Ok(());
    }
    let windows = platform::Windows::open()?;
    let (asks, received) = channel();
    let sender = asks.clone();
    let thread = std::thread::Builder::new()
        .name("clarity parts".into())
        .spawn(move || run(windows, received, sender, told))
        .map_err(|e| format!("the part host's thread could not start: {e}"))?;
    *slot = Some(Running { asks, thread: Some(thread) });
    Ok(())
}

fn ask(a: Ask) {
    if let Ok(slot) = running().lock()
        && let Some(r) = slot.as_ref()
    {
        let _ = r.asks.send(a);
    }
}

/// Where a part is, in the native window `parent`, and whether it shows.
#[allow(clippy::too_many_arguments)]
pub fn place(window: u64, part: &str, kind: &str, parent: u64, x: i32, y: i32, width: i32, height: i32, shown: bool) {
    ask(Ask::Place { window, part: part.into(), kind: kind.into(), parent, x, y, width, height, shown });
}

/// Sets a property of a part.
pub fn set(window: u64, part: &str, name: &str, value: Value) {
    ask(Ask::Set { window, part: part.into(), name: name.into(), value });
}

/// Gives a part a hear with its values.
pub fn hear(window: u64, part: &str, hear: &str, values: Vec<Value>) {
    ask(Ask::Hear { window, part: part.into(), hear: hear.into(), values });
}

/// Ends every part of a window that closed.
pub fn close(window: u64) {
    ask(Ask::Close { window });
}

/// Waits until everything asked before has been done. For tests.
pub fn wait() {
    let (done, waited) = channel();
    ask(Ask::Wait(done));
    let _ = waited.recv_timeout(Duration::from_secs(5));
}

/// Ends every part and the host's thread.
pub fn stop() {
    let running = running().lock().ok().and_then(|mut slot| slot.take());
    if let Some(mut r) = running {
        let _ = r.asks.send(Ask::Stop);
        if let Some(thread) = r.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A part of an open window.
struct Placed {
    made: Option<Box<dyn Part>>,
    child: u64,
    place: (i32, i32, i32, i32),
    shown: bool,
    /// What it was given before it was made, in order.
    waiting: Vec<Ask>,
}

fn run(mut windows: platform::Windows, asks: Receiver<Ask>, sender: Sender<Ask>, told: Told) {
    let tracing = std::env::var_os("CLARITY_PARTS_TRACE").is_some();
    let mut parts: HashMap<(u64, String), Placed> = HashMap::new();
    let mut next_frame = Instant::now() + FRAME;
    loop {
        let wait = next_frame.saturating_duration_since(Instant::now());
        match asks.recv_timeout(wait) {
            Ok(Ask::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Ok(Ask::Wait(done)) => {
                let _ = done.send(());
            }
            Ok(Ask::Tell { window, part, tell, values }) => told(window, &part, &tell, &values),
            Ok(Ask::Place { window, part, kind, parent, x, y, width, height, shown }) => {
                if tracing {
                    eprintln!("clarity parts: place {part} ({kind}) of window {window} in {parent:#x} at {x},{y} {width}x{height} {}", if shown { "shown" } else { "hidden" });
                }
                let make = KINDS.lock().ok().and_then(|k| k.iter().find(|(name, _)| *name == kind).map(|(_, make)| *make));
                let Some(make) = make else { continue };
                let key = (window, part.clone());
                let placed = parts.entry(key).or_insert_with(|| Placed { made: None, child: 0, place: (x, y, width, height), shown, waiting: Vec::new() });
                if placed.made.is_none() {
                    placed.child = windows.child(parent, x, y, width, height);
                    windows.show(placed.child, shown);
                    placed.place = (x, y, width, height);
                    placed.shown = shown;
                    let host = Host { window, part: part.clone(), tells: sender.clone() };
                    let mut made = make(windows.window(placed.child), width.max(1) as u32, height.max(1) as u32, host);
                    for given in placed.waiting.drain(..) {
                        match given {
                            Ask::Set { name, value, .. } => made.set(&name, &value),
                            Ask::Hear { hear, values, .. } => made.hear(&hear, &values),
                            _ => {}
                        }
                    }
                    placed.made = Some(made);
                    continue;
                }
                let resized = (width, height) != (placed.place.2, placed.place.3);
                if (x, y, width, height) != placed.place {
                    windows.move_to(placed.child, x, y, width, height);
                }
                if shown != placed.shown {
                    windows.show(placed.child, shown);
                }
                placed.place = (x, y, width, height);
                placed.shown = shown;
                if resized && let Some(made) = placed.made.as_mut() {
                    made.resize(width.max(1) as u32, height.max(1) as u32);
                }
            }
            Ok(given @ (Ask::Set { .. } | Ask::Hear { .. })) => {
                let key = match &given {
                    Ask::Set { window, part, .. } | Ask::Hear { window, part, .. } => (*window, part.clone()),
                    _ => continue,
                };
                let placed = parts.entry(key).or_insert_with(|| Placed { made: None, child: 0, place: (0, 0, 0, 0), shown: false, waiting: Vec::new() });
                match (placed.made.as_mut(), given) {
                    (Some(made), Ask::Set { name, value, .. }) => made.set(&name, &value),
                    (Some(made), Ask::Hear { hear, values, .. }) => made.hear(&hear, &values),
                    (None, given) => placed.waiting.push(given),
                    _ => {}
                }
            }
            Ok(Ask::Close { window }) => {
                let gone: Vec<(u64, String)> = parts.keys().filter(|(w, _)| *w == window).cloned().collect();
                for key in gone {
                    if let Some(placed) = parts.remove(&key) {
                        drop(placed.made);
                        if placed.child != 0 {
                            windows.destroy(placed.child);
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        if Instant::now() >= next_frame {
            for placed in parts.values_mut().filter(|p| p.shown) {
                if let Some(made) = placed.made.as_mut() {
                    made.frame();
                }
            }
            next_frame = Instant::now() + FRAME;
        }
        windows.flush();
    }
    for (_, placed) in parts.drain() {
        drop(placed.made);
        if placed.child != 0 {
            windows.destroy(placed.child);
        }
    }
}

/// The native windows of the host: X11 on Linux, through a connection of
/// its own; none in the tests, which run without a display.
#[cfg(all(target_os = "linux", not(test)))]
mod platform {
    use std::ffi::{c_int, c_uint, c_ulong, c_void};

    #[link(name = "X11")]
    unsafe extern "C" {
        fn XOpenDisplay(name: *const i8) -> *mut c_void;
        fn XCloseDisplay(display: *mut c_void) -> c_int;
        fn XDefaultScreen(display: *mut c_void) -> c_int;
        fn XBlackPixel(display: *mut c_void, screen: c_int) -> c_ulong;
        fn XCreateSimpleWindow(display: *mut c_void, parent: c_ulong, x: c_int, y: c_int, width: c_uint, height: c_uint, border: c_uint, border_color: c_ulong, background: c_ulong) -> c_ulong;
        fn XMoveResizeWindow(display: *mut c_void, window: c_ulong, x: c_int, y: c_int, width: c_uint, height: c_uint) -> c_int;
        fn XMapRaised(display: *mut c_void, window: c_ulong) -> c_int;
        fn XUnmapWindow(display: *mut c_void, window: c_ulong) -> c_int;
        fn XDestroyWindow(display: *mut c_void, window: c_ulong) -> c_int;
        fn XFlush(display: *mut c_void) -> c_int;
    }

    pub struct Windows {
        display: *mut c_void,
        screen: c_int,
    }

    // The connection is used on the host's thread only.
    unsafe impl Send for Windows {}

    impl Windows {
        pub fn open() -> Result<Windows, String> {
            let display = unsafe { XOpenDisplay(std::ptr::null()) };
            if display.is_null() {
                return Err("custom parts need X11, and no X display could be opened".into());
            }
            Ok(Windows { display, screen: unsafe { XDefaultScreen(display) } })
        }

        pub fn child(&mut self, parent: u64, x: i32, y: i32, width: i32, height: i32) -> u64 {
            let black = unsafe { XBlackPixel(self.display, self.screen) };
            let child = unsafe { XCreateSimpleWindow(self.display, parent as c_ulong, x, y, width.max(1) as c_uint, height.max(1) as c_uint, 0, 0, black) };
            unsafe { XFlush(self.display) };
            child as u64
        }

        pub fn window(&self, child: u64) -> super::Window {
            super::Window { xid: child, display: self.display, screen: self.screen }
        }

        pub fn move_to(&mut self, child: u64, x: i32, y: i32, width: i32, height: i32) {
            unsafe { XMoveResizeWindow(self.display, child as c_ulong, x, y, width.max(1) as c_uint, height.max(1) as c_uint) };
        }

        pub fn show(&mut self, child: u64, shown: bool) {
            unsafe {
                if shown {
                    XMapRaised(self.display, child as c_ulong);
                } else {
                    XUnmapWindow(self.display, child as c_ulong);
                }
            }
        }

        pub fn destroy(&mut self, child: u64) {
            unsafe { XDestroyWindow(self.display, child as c_ulong) };
        }

        pub fn flush(&mut self) {
            unsafe { XFlush(self.display) };
        }
    }

    impl Drop for Windows {
        fn drop(&mut self) {
            unsafe { XCloseDisplay(self.display) };
        }
    }
}

/// The native windows of the host: Win32 child windows on Windows, of the
/// webview's own window. A window belongs to the thread that made it, so
/// they are made, moved and ended on the host's thread, which pumps their
/// messages between frames.
#[cfg(all(windows, not(test)))]
mod platform {
    use std::ffi::c_void;

    use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{BLACK_BRUSH, GetStockObject, HBRUSH};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, EnumChildWindows, GWL_STYLE, GetParent, GetWindowLongW, HWND_TOP,
        IDC_ARROW, LoadCursorW, MSG, PM_REMOVE, PeekMessageW, RegisterClassW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SWP_NOZORDER, SW_HIDE, SW_SHOWNA, SetWindowLongW, SetWindowPos, ShowWindow, TranslateMessage, WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN,
        WS_CLIPSIBLINGS,
    };

    /// The window class of every part's window, "ClarityPart".
    const CLASS: [u16; 12] = [b'C' as u16, b'l' as u16, b'a' as u16, b'r' as u16, b'i' as u16, b't' as u16, b'y' as u16, b'P' as u16, b'a' as u16, b'r' as u16, b't' as u16, 0];

    /// Makes a window of the parent's own children clip its siblings. A
    /// window without this style paints over whatever shares its place,
    /// however the two are ordered, and the webview of every shell is such
    /// a window: without this the part is drawn and then painted over.
    unsafe extern "system" fn clip_siblings(window: HWND, parent: LPARAM) -> i32 {
        unsafe {
            if GetParent(window) as isize != parent {
                return 1;
            }
            let style = GetWindowLongW(window, GWL_STYLE);
            if style != 0 && (style as u32) & WS_CLIPSIBLINGS == 0 {
                SetWindowLongW(window, GWL_STYLE, style | WS_CLIPSIBLINGS as i32);
                SetWindowPos(window, std::ptr::null_mut(), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
            }
        }
        1
    }

    /// The part draws the window itself, so its messages take their usual course.
    unsafe extern "system" fn window_proc(window: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        unsafe { DefWindowProcW(window, message, w, l) }
    }

    pub struct Windows {
        instance: HINSTANCE,
    }

    // The windows are made and used on the host's thread only.
    unsafe impl Send for Windows {}

    impl Windows {
        pub fn open() -> Result<Windows, String> {
            let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
            if instance.is_null() {
                return Err("custom parts need the program's own module, which Windows did not give".into());
            }
            let class = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: std::ptr::null_mut(),
                hCursor: unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) },
                hbrBackground: unsafe { GetStockObject(BLACK_BRUSH) } as HBRUSH,
                lpszMenuName: std::ptr::null(),
                lpszClassName: CLASS.as_ptr(),
            };
            // The class is registered once; a second program window finds it there.
            unsafe { RegisterClassW(&class) };
            Ok(Windows { instance })
        }

        pub fn child(&mut self, parent: u64, x: i32, y: i32, width: i32, height: i32) -> u64 {
            let child = unsafe {
                CreateWindowExW(
                    0,
                    CLASS.as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
                    x,
                    y,
                    width.max(1),
                    height.max(1),
                    parent as HWND,
                    std::ptr::null_mut(),
                    self.instance,
                    std::ptr::null(),
                )
            };
            if child.is_null() {
                return 0;
            }
            // Above the webview, which fills the same parent, and which is
            // made to leave the part's place alone.
            unsafe {
                EnumChildWindows(parent as HWND, Some(clip_siblings), parent as LPARAM);
                SetWindowPos(child, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            child as u64
        }

        pub fn window(&self, child: u64) -> super::Window {
            super::Window { hwnd: child as *mut c_void, hinstance: self.instance }
        }

        pub fn move_to(&mut self, child: u64, x: i32, y: i32, width: i32, height: i32) {
            unsafe { SetWindowPos(child as HWND, HWND_TOP, x, y, width.max(1), height.max(1), SWP_NOACTIVATE) };
        }

        pub fn show(&mut self, child: u64, shown: bool) {
            // SW_SHOWNA: shown without taking the keyboard from the window.
            unsafe { ShowWindow(child as HWND, if shown { SW_SHOWNA } else { SW_HIDE }) };
        }

        pub fn destroy(&mut self, child: u64) {
            unsafe { DestroyWindow(child as HWND) };
        }

        /// The messages of the part windows, which belong to this thread.
        /// The host asks for this between frames, so they are answered about
        /// sixty times a second and the parent is never kept waiting.
        pub fn flush(&mut self) {
            let mut message = MSG {
                hwnd: std::ptr::null_mut(),
                message: 0,
                wParam: 0,
                lParam: 0,
                time: 0,
                pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
            };
            while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
    }
}

#[cfg(any(all(not(target_os = "linux"), not(windows)), test))]
mod platform {
    pub struct Windows {
        next: u64,
    }

    impl Windows {
        #[cfg(not(test))]
        pub fn open() -> Result<Windows, String> {
            Err("custom parts run on Linux for now".into())
        }

        #[cfg(test)]
        pub fn open() -> Result<Windows, String> {
            Ok(Windows { next: 1 })
        }

        pub fn child(&mut self, _parent: u64, _x: i32, _y: i32, _width: i32, _height: i32) -> u64 {
            self.next += 1;
            self.next
        }

        #[cfg(not(windows))]
        pub fn window(&self, child: u64) -> super::Window {
            super::Window { xid: child, display: std::ptr::null_mut(), screen: 0 }
        }

        #[cfg(windows)]
        pub fn window(&self, child: u64) -> super::Window {
            super::Window { hwnd: child as *mut std::ffi::c_void, hinstance: std::ptr::null_mut() }
        }

        pub fn move_to(&mut self, _child: u64, _x: i32, _y: i32, _width: i32, _height: i32) {}
        pub fn show(&mut self, _child: u64, _shown: bool) {}
        pub fn destroy(&mut self, _child: u64) {}
        pub fn flush(&mut self) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    static CALLS: Mutex<Vec<String>> = Mutex::new(Vec::new());

    fn called(text: String) {
        CALLS.lock().unwrap().push(text);
    }

    fn taken() -> Vec<String> {
        std::mem::take(&mut *CALLS.lock().unwrap())
    }

    struct Probe {
        host: Host,
    }

    impl Part for Probe {
        fn resize(&mut self, width: u32, height: u32) {
            called(format!("resize {width}x{height}"));
        }
        fn set(&mut self, name: &str, value: &Value) {
            called(format!("set {name} {value}"));
        }
        fn hear(&mut self, name: &str, values: &[Value]) {
            called(format!("hear {name} {}", Value::from(values.to_vec())));
            if name == "ping" {
                self.host.tell("pong", vec![value(&42)]);
            }
        }
    }

    fn make(_window: Window, width: u32, height: u32, host: Host) -> Box<dyn Part> {
        called(format!("create {width}x{height}"));
        Box::new(Probe { host })
    }

    // As the C part host does (runtimes/parts/tests/parts_test.c).
    #[test]
    fn the_host_makes_places_sets_and_tells_its_parts() {
        register("Probe", make);
        assert!(present());
        let heard = Arc::new(Mutex::new(Vec::new()));
        let into = heard.clone();
        start(Box::new(move |window, part, tell, values| into.lock().unwrap().push(format!("{window} {part} {tell} {}", Value::from(values.to_vec()))))).unwrap();

        // What is given before the part is placed reaches it once it is made, in order.
        set(1, "arena", "speed", value(&5));
        hear(1, "arena", "restart", vec![value(&1)]);
        place(1, "arena", "Probe", 99, 10, 20, 300, 200, true);
        wait();
        assert_eq!(taken(), ["create 300x200", "set speed 5", "hear restart [1]"]);

        // A new size resizes it; a tell comes back with its window and name.
        place(1, "arena", "Probe", 99, 10, 20, 400, 250, true);
        hear(1, "arena", "ping", vec![]);
        wait();
        wait();
        assert_eq!(taken(), ["resize 400x250", "hear ping []"]);
        assert_eq!(*heard.lock().unwrap(), ["1 arena pong [42]"]);

        // Closing its window ends it; a kind the program does not have makes nothing.
        close(1);
        place(2, "map", "Nowhere", 98, 0, 0, 10, 10, true);
        wait();
        assert!(taken().is_empty());
        stop();
    }
}
