//! Clarity.UI in the browser (DESIGN 7.9, round B): the entry points the
//! worker script (js/clarity-ui-worker.js) calls, over clarity-ui's browser
//! host (`clarity_ui::browser::web`). Text comes in as NUL-terminated UTF-8
//! the worker allocated (and frees); text going out is freed with
//! `clarity_ui_string_free`. Built for wasm32-unknown-emscripten by build.sh.

// setjmp and longjmp for Skia's prebuilt decoders.
#[cfg(target_os = "emscripten")]
mod sjlj;

#[cfg(target_os = "emscripten")]
mod entry {
    use std::ffi::{CStr, CString, c_char};

    use base64::Engine as _;
    use clarity_ui::browser::web;

    fn text<'a>(pointer: *const c_char) -> &'a str {
        if pointer.is_null() {
            return "";
        }
        unsafe { CStr::from_ptr(pointer) }.to_str().unwrap_or("")
    }

    fn given(text: String) -> *mut c_char {
        CString::new(text.replace('\0', "")).map(CString::into_raw).unwrap_or(std::ptr::null_mut())
    }

    /// Starts the program from `files.json`'s text, with the person's
    /// `choices` the page kept (JSON), on a canvas of `width` by `height` CSS
    /// pixels at `scale`: NULL, or why it could not start.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_start(files: *const c_char, choices: *const c_char, width: f64, height: f64, scale: f64, dark: i32) -> *mut c_char {
        match web::start(text(files), text(choices), width, height, scale as f32, dark != 0) {
            Ok(()) => std::ptr::null_mut(),
            Err(why) => given(why),
        }
    }

    /// Starts the program again from new `files` (files.json's text) on the
    /// same canvas, with the person's `choices` (JSON): NULL, or why it could not.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_reload(files: *const c_char, choices: *const c_char) -> *mut c_char {
        match web::reload(text(files), text(choices)) {
            Ok(()) => std::ptr::null_mut(),
            Err(why) => given(why),
        }
    }

    /// Where the design's parts and grids show, as JSON.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_layout() -> *mut c_char {
        given(web::layout())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_resize(width: f64, height: f64, scale: f64) {
        web::resize(width, height, scale as f32);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_dark(dark: i32) {
        web::set_dark(dark != 0);
    }

    /// `kind`: 0 move, 1 press, 2 release, 3 leave, 4 cancel; `button` the event's; `held` Shift 1, Control 2, Alt 4, Meta 8.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_pointer(kind: i32, x: f64, y: f64, button: i32, held: u32) {
        web::pointer(kind, x, y, button, held);
    }

    /// `mode` is the wheel event's deltaMode.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_wheel(x: f64, y: f64, dx: f64, dy: f64, mode: u32, held: u32) {
        web::wheel(x, y, dx, dy, mode, held);
    }

    /// A key down (1) or up (0) by the event's `key` and `code`: 1 when the screen took it.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_key(down: i32, key: *const c_char, code: *const c_char, held: u32) -> i32 {
        web::key(down != 0, text(key), text(code), held) as i32
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_text(typed: *const c_char) {
        web::text(text(typed));
    }

    /// An input method composes `composing` (NULL or empty: it ended), the marked part from `start` to `end` in bytes (-1 for none).
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_compose(composing: *const c_char, start: i32, end: i32) {
        let marked = (start >= 0 && end >= start).then_some((start as usize, end as usize));
        let composing = text(composing);
        web::compose((!composing.is_empty()).then_some(composing), marked);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_pasted(pasted: *const c_char) {
        web::pasted(text(pasted));
    }

    /// How the page's song `id` plays: playing, paused and ended as 0 or 1, seconds played and its length.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_song_said(id: f64, playing: i32, paused: i32, ended: i32, position: f64, length: f64) {
        web::song_said(id as u64, clarity_ui::browser::page::SongSaid { playing: playing != 0, paused: paused != 0, ended: ended != 0, position, length });
    }

    /// How the page's video `id` plays: playing, paused, ended and failed as 0 or 1, seconds played and its length.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_video_said(id: f64, playing: i32, paused: i32, ended: i32, failed: i32, position: f64, length: f64) {
        web::video_said(id as u64, playing != 0, paused != 0, ended != 0, failed != 0, position, length);
    }

    /// Video `id`'s next picture: `len` bytes of RGBA at `bytes`, `width` by `height`.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_video_frame(id: f64, width: i32, height: i32, bytes: *const u8, len: usize) {
        let rgba = if bytes.is_null() { Vec::new() } else { unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec() };
        web::video_frame(id as u64, width.max(0) as u32, height.max(0) as u32, rgba);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_sound_ended(id: f64) {
        web::sound_ended(id as u64);
    }

    /// The file the person chose in the page: its name, and `len` bytes at `bytes`.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_file_chosen(name: *const c_char, bytes: *const u8, len: usize) {
        let bytes = if bytes.is_null() { Vec::new() } else { unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec() };
        web::file_chosen(text(name), bytes);
    }

    /// The answer (NULL for none) to request `id`, which went out without waiting.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_answered(id: u32, answer: *const c_char) {
        web::answered(id, (!answer.is_null()).then(|| text(answer)));
    }

    /// The page keeps an accessible mirror (1) or not (0): while on, the page is told the tree's changes (`access`).
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_access(on: i32) {
        web::access(on != 0);
    }

    /// A screen reader's `action` on the mirror's `node` (its id as text), with `value` (NULL for none): 1 when done.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_act(node: *const c_char, action: *const c_char, value: *const c_char) -> i32 {
        web::act(text(node), text(action), (!value.is_null()).then(|| text(value))) as i32
    }

    /// A frame: 1 when it drew.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_frame() -> i32 {
        web::frame() as i32
    }

    /// Milliseconds until the screen wants a frame by itself; -1 while it waits for the person.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_next_wake() -> f64 {
        web::next_wake()
    }

    /// A driven step (DESIGN 1.6) as JSON; its answer as JSON.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_step(step: *const c_char) -> *mut c_char {
        given(web::step(text(step)))
    }

    /// What the canvas shows, as a PNG in base64; NULL before the first frame.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_shot() -> *mut c_char {
        match web::shot() {
            Some(png) => given(base64::engine::general_purpose::STANDARD.encode(png)),
            None => std::ptr::null_mut(),
        }
    }

    /// Frees text the program gave.
    #[unsafe(no_mangle)]
    pub extern "C" fn clarity_ui_string_free(given: *mut c_char) {
        if !given.is_null() {
            drop(unsafe { CString::from_raw(given) });
        }
    }
}

fn main() {
    // Emscripten runs main as the module loads; the worker calls the entry points after.
    #[cfg(not(target_os = "emscripten"))]
    eprintln!("clarity-ui is the browser program: build it for wasm32-unknown-emscripten (runtimes/rust-ui-browser/build.sh)");
}
