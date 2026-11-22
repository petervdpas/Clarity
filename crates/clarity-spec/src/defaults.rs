//! Clarity's defaults, owned here and written out once in
//! conformance/defaults.json: the generators take them from here, and each
//! runtime that must carry a copy checks it against that file in its own
//! tests.

/// How long a sign-in session lasts, in minutes, when the project's signin says nothing.
pub const SESSION_MINUTES: u32 = 8 * 60;
/// How many wrong tries before the sign-in waits.
pub const TRIES: u32 = 5;
/// How long the sign-in waits after too many wrong tries, in minutes.
pub const TRIES_MINUTES: u32 = 15;
/// The steps a window's undo keeps.
pub const UNDO_STEPS: u32 = 100;
/// The automatic backups kept of a database.
pub const KEPT_BACKUPS: u32 = 10;
/// How long an http call may take, in seconds.
pub const HTTP_SECONDS: u32 = 30;
/// How many records' held lists one read gathers (conformance/held-lists.json).
pub const OWNERS_PER_READ: u32 = 500;
/// The edits a window's edit bus keeps (conformance/edit-bus.json).
pub const EDIT_BUS_KEEPS: u32 = 500;
/// How long a browser page may be quiet before a web program lets it go, in minutes.
pub const QUIET_MINUTES: u32 = 30;
/// How long a toast shows when neither it nor the app says, in milliseconds.
pub const TOAST_LIFESPAN: u32 = 5000;
/// The shortest and the longest lifespan a toast may have, in milliseconds.
pub const TOAST_LIFESPANS: std::ops::RangeInclusive<u32> = 500..=120_000;
