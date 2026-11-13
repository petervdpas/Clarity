//! The program's files (`ui::AppFiles`) as the caller gives them: its
//! windows' and apps' markup, its themes, its model, its resources, its
//! title and size. The library keeps them for the rest of the process once
//! they are used (AppFiles holds them for good), one copy for each time they
//! changed.

use std::ffi::c_char;

use ui::AppFiles;

use crate::{bytes, guard, handle, handle_mut, put, text, ClarityStatus, Failure};

/// The files as given, and the copy the library used last.
#[derive(Default)]
pub struct ClarityFiles {
    windows: Vec<(String, String)>,
    apps: Vec<(String, String)>,
    themes: Vec<(String, String)>,
    model: String,
    resources: Vec<(String, Vec<u8>)>,
    title: String,
    width: f64,
    height: f64,
    kept: std::cell::Cell<Option<AppFiles>>,
}

fn leak(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

fn set(list: &mut Vec<(String, String)>, name: &str, value: &str) {
    match list.iter_mut().find(|(n, _)| n == name) {
        Some(found) => found.1 = value.to_string(),
        None => list.push((name.to_string(), value.to_string())),
    }
}

impl ClarityFiles {
    /// The files as Clarity.UI reads them, kept for the process.
    pub fn app_files(&self) -> AppFiles {
        if let Some(kept) = self.kept.get() {
            return kept;
        }
        let pairs = |list: &[(String, String)]| -> &'static [(&'static str, &'static str)] {
            Box::leak(list.iter().map(|(n, v)| (leak(n), leak(v))).collect::<Vec<_>>().into_boxed_slice())
        };
        let resources: &'static [(&'static str, &'static [u8])] =
            Box::leak(self.resources.iter().map(|(n, b)| (leak(n), &*Box::leak(b.clone().into_boxed_slice()))).collect::<Vec<_>>().into_boxed_slice());
        let files = AppFiles {
            windows: pairs(&self.windows),
            apps: pairs(&self.apps),
            themes: pairs(&self.themes),
            model: leak(&self.model),
            resources,
            title: leak(&self.title),
            width: self.width,
            height: self.height,
        };
        self.kept.set(Some(files));
        files
    }

    fn changed(&mut self) {
        self.kept.set(None);
    }
}

/// Empty files.
///
/// # Safety
/// `out` points at writable memory for a handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_files_new(out: *mut *mut ClarityFiles) -> ClarityStatus {
    guard(|| unsafe { put(out, Box::into_raw(Box::new(ClarityFiles { model: "{}".to_string(), ..ClarityFiles::default() }))) })
}

/// Frees files.
///
/// # Safety
/// `files` is NULL or a handle from `clarity_files_new`, not freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_files_free(files: *mut ClarityFiles) {
    if !files.is_null() {
        crate::quietly(|| drop(unsafe { Box::from_raw(files) }));
    }
}

/// Sets one named text of the files.
unsafe fn named(files: *mut ClarityFiles, name: *const c_char, name_len: usize, value: *const c_char, value_len: usize, list: fn(&mut ClarityFiles) -> &mut Vec<(String, String)>) -> ClarityStatus {
    guard(|| {
        let files = unsafe { handle_mut(files) }?;
        let (name, value) = unsafe { (text(name, name_len)?, text(value, value_len)?) };
        if name.is_empty() {
            return Err(Failure::invalid("the name is empty"));
        }
        set(list(files), name, value);
        files.changed();
        Ok(())
    })
}

/// # Safety
/// `files` is a live handle; the texts are (pointer, length) pairs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_files_window(files: *mut ClarityFiles, name: *const c_char, name_len: usize, markup: *const c_char, markup_len: usize) -> ClarityStatus {
    unsafe { named(files, name, name_len, markup, markup_len, |f| &mut f.windows) }
}

/// # Safety
/// As `clarity_files_window`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_files_app(files: *mut ClarityFiles, name: *const c_char, name_len: usize, markup: *const c_char, markup_len: usize) -> ClarityStatus {
    unsafe { named(files, name, name_len, markup, markup_len, |f| &mut f.apps) }
}

/// # Safety
/// As `clarity_files_window`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_files_theme(files: *mut ClarityFiles, name: *const c_char, name_len: usize, json: *const c_char, json_len: usize) -> ClarityStatus {
    unsafe { named(files, name, name_len, json, json_len, |f| &mut f.themes) }
}

/// # Safety
/// As `clarity_files_window`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_files_model(files: *mut ClarityFiles, json: *const c_char, json_len: usize) -> ClarityStatus {
    guard(|| {
        let files = unsafe { handle_mut(files) }?;
        let model = unsafe { text(json, json_len) }?;
        serde_json::from_str::<serde_json::Value>(model).map_err(|e| Failure::invalid(format!("the model does not read: {e}")))?;
        files.model = model.to_string();
        files.changed();
        Ok(())
    })
}

/// # Safety
/// As `clarity_files_window`; `data` points at `data_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_files_resource(files: *mut ClarityFiles, name: *const c_char, name_len: usize, data: *const u8, data_len: usize) -> ClarityStatus {
    guard(|| {
        let files = unsafe { handle_mut(files) }?;
        let name = unsafe { text(name, name_len) }?;
        if name.is_empty() {
            return Err(Failure::invalid("the name is empty"));
        }
        let data = unsafe { bytes(data, data_len) }.to_vec();
        match files.resources.iter_mut().find(|(n, _)| n == name) {
            Some(found) => found.1 = data,
            None => files.resources.push((name.to_string(), data)),
        }
        files.changed();
        Ok(())
    })
}

/// # Safety
/// As `clarity_files_window`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_files_title(files: *mut ClarityFiles, title: *const c_char, title_len: usize) -> ClarityStatus {
    guard(|| {
        let files = unsafe { handle_mut(files) }?;
        files.title = unsafe { text(title, title_len) }?.to_string();
        files.changed();
        Ok(())
    })
}

/// # Safety
/// `files` is a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_files_size(files: *mut ClarityFiles, width: f64, height: f64) -> ClarityStatus {
    guard(|| {
        let files = unsafe { handle_mut(files) }?;
        if !(width.is_finite() && height.is_finite()) || width < 0.0 || height < 0.0 {
            return Err(Failure::invalid("the size is no size"));
        }
        files.width = width;
        files.height = height;
        files.changed();
        Ok(())
    })
}

// -- Choices -----------------------------------------------------------------

/// Starts the person's choices from the files' model.
///
/// # Safety
/// `files` is a live handle; `folder` is (pointer, length) or NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_choices_start(files: *const ClarityFiles, folder: *const c_char, folder_len: usize) -> ClarityStatus {
    guard(|| {
        let files = unsafe { handle(files) }?.app_files();
        let folder = unsafe { crate::text_or_none(folder, folder_len) }?.map(std::path::PathBuf::from);
        clarity_runtime::i18n::load(files.model);
        ui::choices::start(&files, folder);
        Ok(())
    })
}

/// # Safety
/// `tag` is (pointer, length).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_language_choose(tag: *const c_char, tag_len: usize) -> ClarityStatus {
    guard(|| {
        let tag = unsafe { text(tag, tag_len) }?;
        if !ui::choices::languages().iter().any(|l| l.name == tag) {
            return Err(Failure::not_found(format!("the program offers no language {tag}")));
        }
        ui::choices::choose_language(tag);
        Ok(())
    })
}

/// # Safety
/// `name` is (pointer, length).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_theme_choose(name: *const c_char, name_len: usize) -> ClarityStatus {
    guard(|| {
        let name = unsafe { text(name, name_len) }?;
        if !ui::choices::themes().iter().any(|t| t.name == name) {
            return Err(Failure::not_found(format!("the program offers no theme {name}")));
        }
        ui::choices::choose_theme(name);
        Ok(())
    })
}

/// What is chosen and offered, as JSON.
///
/// # Safety
/// `out` points at writable memory for a string pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_choices_json(out: *mut *mut c_char) -> ClarityStatus {
    guard(|| {
        let offers = |list: Vec<ui::choices::Offer>| -> Vec<serde_json::Value> {
            list.into_iter().map(|o| serde_json::json!({ "name": o.name, "label": o.label, "current": o.current })).collect()
        };
        let chosen = serde_json::json!({
            "language": ui::choices::language(),
            "languages": offers(ui::choices::languages()),
            "theme": ui::choices::theme(),
            "themes": offers(ui::choices::themes()),
        });
        unsafe { put(out, crate::give(&chosen.to_string())) }
    })
}

/// A translated text of the model by its key.
///
/// # Safety
/// The texts are (pointer, length); `out` points at writable memory for a string pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_translate(key: *const c_char, key_len: usize, english: *const c_char, english_len: usize, out: *mut *mut c_char) -> ClarityStatus {
    guard(|| {
        let (key, english) = unsafe { (text(key, key_len)?, text(english, english_len)?) };
        unsafe { put(out, crate::give(&ui::locale::translate(key, english))) }
    })
}

/// Asks the AI agent whose command the program runs now (inside the program
/// callback's `agent_call`, docs/ledger/app-mcp.md) `question` of the kind
/// `form` (JSON, as conformance/command-line.json writes it): the answer as
/// the terminal gives it in `*out`, which the caller frees with
/// clarity_string_free; `*out` is NULL when the agent gives none.
///
/// # Safety
/// The texts are (pointer, length); `out` points at writable memory for a string pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_agent_ask(question: *const c_char, question_len: usize, form: *const c_char, form_len: usize, out: *mut *mut c_char) -> ClarityStatus {
    guard(|| {
        let (question, form) = unsafe { (text(question, question_len)?, text(form, form_len)?) };
        let answer = ui::program_wire::ask_agent(question, form).map(|a| crate::give(&a)).unwrap_or(std::ptr::null_mut());
        unsafe { put(out, answer) }
    })
}
