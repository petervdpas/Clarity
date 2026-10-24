//! Custom part Dice (DESIGN 5.19): the trait the crate's `Dice` implements, the
//! window it draws in and the tells it calls. Written by Clarity from the
//! part's declaration, and written again when the declaration changes; the
//! rest of the crate is yours.
//!
//! Every method is called on the part host's thread, one at a time; the part
//! is made there and lives there.

#![allow(dead_code)]

use std::ffi::c_void;

/// The native window the part draws in: on Linux an X11 window, on the
/// part host's own X11 connection; on Windows a Win32 child window of the
/// webview's. Its raw handle is what wgpu, Bevy and softbuffer take.
#[cfg(not(windows))]
#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub xid: u64,
    pub display: *mut c_void,
    pub screen: i32,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
pub struct Window {
    /// The child window (an HWND).
    pub hwnd: *mut c_void,
    /// The module its window class belongs to (an HINSTANCE).
    pub hinstance: *mut c_void,
}

#[cfg(not(windows))]
impl raw_window_handle::HasWindowHandle for Window {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let handle = raw_window_handle::XlibWindowHandle::new(self.xid as std::ffi::c_ulong);
        // The window lives as long as the part: the host ends the part first.
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(handle.into()) })
    }
}

#[cfg(not(windows))]
impl raw_window_handle::HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        let handle = raw_window_handle::XlibDisplayHandle::new(std::ptr::NonNull::new(self.display), self.screen);
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(handle.into()) })
    }
}

#[cfg(windows)]
impl raw_window_handle::HasWindowHandle for Window {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let window = std::num::NonZeroIsize::new(self.hwnd as isize).ok_or(raw_window_handle::HandleError::Unavailable)?;
        let mut handle = raw_window_handle::Win32WindowHandle::new(window);
        handle.hinstance = std::num::NonZeroIsize::new(self.hinstance as isize);
        // The window lives as long as the part: the host ends the part first.
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(handle.into()) })
    }
}

#[cfg(windows)]
impl raw_window_handle::HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        let handle = raw_window_handle::WindowsDisplayHandle::new();
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(handle.into()) })
    }
}

/// A value a tell gives.
#[derive(Debug, Clone, PartialEq)]
pub enum Told {
    Integer(i64),
    Bool(bool),
    Text(String),
    Empty,
}

/// What part Dice does.
pub trait DicePart {
    /// Makes the part in `window`, `width` by `height` pixels.
    fn create(window: Window, width: u32, height: u32, host: DiceHost) -> Self
    where
        Self: Sized;
    /// The window changed size.
    fn resize(&mut self, width: u32, height: u32) {
        let _ = (width, height);
    }
    /// About 60 times a second while the part shows: draw, and read input.
    fn frame(&mut self) {}
    /// Whether the person can use it, as code sets `enabled`.
    fn set_enabled(&mut self, enabled: bool) {
        let _ = enabled;
    }
    /// Property count, as the window or code sets it.
    fn set_count(&mut self, count: i64);
    /// What code tells it: roll.
    fn roll(&mut self);
}

/// Where part Dice tells what happened.
pub struct DiceHost {
    tell: Box<dyn Fn(&str, Vec<Told>)>,
}

impl DiceHost {
    /// Made by the program for each part it shows.
    pub fn new(tell: Box<dyn Fn(&str, Vec<Told>)>) -> Self {
        DiceHost { tell }
    }

    /// Tells landed: its handler runs with these values.
    pub fn landed(&self, total: i64, faces: &str) {
        (self.tell)("landed", vec![Told::Integer(total), Told::Text(faces.to_string())]);
    }
}
