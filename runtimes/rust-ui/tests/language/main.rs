//! What changes the language or translations of the whole process, kept apart so no other test reads its words meanwhile.
//! One test binary for these files (each a module, its tests named
//! `<file>::<test>`), so Skia is linked once for them all.

#[allow(dead_code, unused_imports)]
#[path = "../common/core_windows_fake.rs"]
mod core_windows_fake;
#[allow(dead_code, unused_imports)]
#[path = "../common/inputs_program.rs"]
mod inputs_program;

mod core_facts;
mod core_windows_language;
mod inputs_check_fix_input;
mod inputs_picker_parity;
mod windows_host;
mod windows_language;

/// The language and choices are the whole process's: these tests take turns.
fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());
    TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
