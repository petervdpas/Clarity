//! Windows' open, save and folder dialog (the common item dialog,
//! IFileDialog) through COM, called by hand with no crate of its own, on a
//! thread of its own as COM wants (a single-threaded apartment).

use std::ffi::c_void;

use super::{Ask, Kind, SystemWindow};

type Hresult = i32;

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid(u32, u16, u16, [u8; 8]);

const CLSID_OPEN: Guid = Guid(0xDC1C5A9C, 0xE88A, 0x4DDE, [0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7]);
const CLSID_SAVE: Guid = Guid(0xC0B4E2F3, 0xBA21, 0x4773, [0x8D, 0xBA, 0x33, 0x5E, 0xC9, 0x46, 0xEB, 0x8B]);
const IID_FILE_DIALOG: Guid = Guid(0x42F85136, 0xDB7E, 0x439C, [0x85, 0xF1, 0xE4, 0x07, 0x5D, 0x13, 0x5F, 0xC8]);

const CLSCTX_INPROC_SERVER: u32 = 0x1;
const COINIT_APARTMENTTHREADED: u32 = 0x2;
const OVERWRITE_PROMPT: u32 = 0x2;
const PICK_FOLDERS: u32 = 0x20;
const FORCE_FILE_SYSTEM: u32 = 0x40;
const PATH_MUST_EXIST: u32 = 0x800;
const FILE_MUST_EXIST: u32 = 0x1000;
const FILE_SYSTEM_PATH: u32 = 0x8005_8000;
const CANCELLED: Hresult = 0x8007_04C7_u32 as i32;

#[repr(C)]
struct FilterSpec {
    name: *const u16,
    spec: *const u16,
}

/// IFileDialog's functions, in the order of its interface (IUnknown, IModalWindow, IFileDialog).
// The slots Clarity does not call are kept for their place only.
#[allow(dead_code)]
#[repr(C)]
struct FileDialogVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    show: unsafe extern "system" fn(*mut c_void, *mut c_void) -> Hresult,
    set_file_types: unsafe extern "system" fn(*mut c_void, u32, *const FilterSpec) -> Hresult,
    set_file_type_index: usize,
    get_file_type_index: usize,
    advise: usize,
    unadvise: usize,
    set_options: unsafe extern "system" fn(*mut c_void, u32) -> Hresult,
    get_options: unsafe extern "system" fn(*mut c_void, *mut u32) -> Hresult,
    set_default_folder: usize,
    set_folder: usize,
    get_folder: usize,
    get_current_selection: usize,
    set_file_name: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    get_file_name: usize,
    set_title: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    set_ok_button_label: usize,
    set_file_name_label: usize,
    get_result: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
}

/// IShellItem's functions, in the order of its interface.
#[allow(dead_code)]
#[repr(C)]
struct ShellItemVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    bind_to_handler: usize,
    get_parent: usize,
    get_display_name: unsafe extern "system" fn(*mut c_void, u32, *mut *mut u16) -> Hresult,
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, init: u32) -> Hresult;
    fn CoUninitialize();
    fn CoCreateInstance(class: *const Guid, outer: *mut c_void, context: u32, iid: *const Guid, out: *mut *mut c_void) -> Hresult;
    fn CoTaskMemFree(memory: *mut c_void);
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn failed(hr: Hresult) -> bool {
    hr < 0
}

/// The path the person chose; none when they chose none; an error when the dialog could not show.
pub fn choose(ask: &Ask, owner: Option<SystemWindow>) -> Result<Option<String>, String> {
    let ask = ask.clone();
    // The window it belongs to, which takes no input meanwhile.
    let owner = owner.map(|o| o.win32).unwrap_or(0);
    std::thread::spawn(move || unsafe { show(&ask, owner) }).join().map_err(|_| "the file dialog stopped".to_string())?
}

unsafe fn show(ask: &Ask, owner: isize) -> Result<Option<String>, String> {
    unsafe {
        let init = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
        let result = dialog(ask, owner);
        if !failed(init) {
            CoUninitialize();
        }
        result
    }
}

unsafe fn dialog(ask: &Ask, owner: isize) -> Result<Option<String>, String> {
    unsafe {
        let class = if ask.kind == Kind::Save { CLSID_SAVE } else { CLSID_OPEN };
        let mut made: *mut c_void = std::ptr::null_mut();
        let hr = CoCreateInstance(&class, std::ptr::null_mut(), CLSCTX_INPROC_SERVER, &IID_FILE_DIALOG, &mut made);
        if failed(hr) || made.is_null() {
            return Err(format!("no common item dialog ({hr:#x})"));
        }
        let vtbl = &**(made as *mut *const FileDialogVtbl);
        let answer = (|| {
            let mut options = 0u32;
            (vtbl.get_options)(made, &mut options);
            options |= FORCE_FILE_SYSTEM | PATH_MUST_EXIST;
            options |= match ask.kind {
                Kind::Save => OVERWRITE_PROMPT,
                Kind::Folder => PICK_FOLDERS,
                Kind::Open => FILE_MUST_EXIST,
            };
            (vtbl.set_options)(made, options);
            let title = wide(&ask.title);
            (vtbl.set_title)(made, title.as_ptr());
            let name = wide(ask.name.as_deref().unwrap_or(""));
            if ask.kind == Kind::Save {
                (vtbl.set_file_name)(made, name.as_ptr());
            }
            let filter = ask.filter.as_ref().filter(|_| ask.kind == Kind::Open).map(|f| (wide(&f.name), wide(&f.patterns.join(";"))));
            if let Some((name, spec)) = &filter {
                let specs = [FilterSpec { name: name.as_ptr(), spec: spec.as_ptr() }];
                (vtbl.set_file_types)(made, 1, specs.as_ptr());
            }
            let shown = (vtbl.show)(made, owner as *mut c_void);
            if shown == CANCELLED {
                return Ok(None);
            }
            if failed(shown) {
                return Err(format!("the dialog did not show ({shown:#x})"));
            }
            let mut item: *mut c_void = std::ptr::null_mut();
            if failed((vtbl.get_result)(made, &mut item)) || item.is_null() {
                return Ok(None);
            }
            let item_vtbl = &**(item as *mut *const ShellItemVtbl);
            let mut name: *mut u16 = std::ptr::null_mut();
            let path = if !failed((item_vtbl.get_display_name)(item, FILE_SYSTEM_PATH, &mut name)) && !name.is_null() {
                let mut len = 0;
                while *name.add(len) != 0 {
                    len += 1;
                }
                let path = String::from_utf16_lossy(std::slice::from_raw_parts(name, len));
                CoTaskMemFree(name as *mut c_void);
                Some(path)
            } else {
                None
            };
            (item_vtbl.release)(item);
            Ok(path)
        })();
        (vtbl.release)(made);
        answer
    }
}
