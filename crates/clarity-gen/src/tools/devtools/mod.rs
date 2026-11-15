//! DevTools (Clarity settings, a Clarity run from its repository only):
//! the cDdev's tools for working on Clarity itself. A tab per piece of
//! Clarity an app uses, with the release or the cDdev's own build (swap)
//! and a Build button (jobs); and the release planning (release), which
//! reads the repository as it is and writes the Release Helper's plan.

pub mod jobs;
pub mod release;
pub mod swap;
