//! DevTools (Clarity settings, a Clarity run from its repository only):
//! the cDdev's tools for working on Clarity itself. A tab per piece of
//! Clarity an app uses, with the release or the cDdev's own build (swap)
//! and a Build button (jobs); whether the browser files are behind their
//! source (browser); the release planning (release), which reads the
//! repository as it is, for review and for the agent's plan; the disk
//! (disk) and the versions to bump (bump).

pub mod browser;
pub mod bump;
pub mod disk;
pub mod jobs;
pub mod release;
pub mod swap;
