//! The Go generator's tests, one test program: the files it writes
//! (generate), sign-in (signin) and row versions (versions).

mod build;
mod generate;
mod readme;
mod signin;
mod versions;

/// Writes a guard's file only when its content differs, so writing it again
/// changes nothing on disk (a running IDE sees no change and does not restart).
pub(crate) fn write_changed(path: impl AsRef<std::path::Path>, bytes: impl AsRef<[u8]>) {
    let (path, bytes) = (path.as_ref(), bytes.as_ref());
    if std::fs::read(path).ok().as_deref() != Some(bytes) {
        std::fs::write(path, bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}
