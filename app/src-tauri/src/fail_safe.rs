//! Fail safe: a panic in one command never leaves the IDE broken. A lock a
//! panicking command held stays usable (`held`), and the panic itself shows
//! in the IDE's error bar (`report_panics`), not only on the terminal.

use std::sync::{LockResult, Mutex, MutexGuard, OnceLock, PoisonError};

use tauri::Emitter;

/// A lock that stays usable after a command that held it panicked.
pub trait Held<T> {
    /// As `lock`, but a lock a panic left behind is taken all the same: it never fails.
    fn held(&self) -> LockResult<MutexGuard<'_, T>>;
}

impl<T> Held<T> for Mutex<T> {
    fn held(&self) -> LockResult<MutexGuard<'_, T>> {
        Ok(self.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

/// From now on a panic anywhere in the backend is told to the screen
/// (`clarity-panic`, with its message and place) as well as printed.
pub fn report_panics(app: tauri::AppHandle) {
    let _ = APP.set(app);
    let printed = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        printed(info);
        let message = info
            .payload()
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| info.payload().downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "a command stopped".to_string());
        let place = info.location().map(|l| format!(" ({}:{})", l.file(), l.line())).unwrap_or_default();
        if let Some(app) = APP.get() {
            let _ = app.emit("clarity-panic", format!("{message}{place}"));
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lock_a_panic_left_behind_is_held_all_the_same() {
        let shared = std::sync::Arc::new(Mutex::new(1));
        let poisoner = shared.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.lock().unwrap();
            panic!("a command failed");
        })
        .join();
        assert!(shared.lock().is_err(), "the lock is poisoned");
        *shared.held().unwrap() += 1;
        assert_eq!(*shared.held().unwrap(), 2);
    }
}
