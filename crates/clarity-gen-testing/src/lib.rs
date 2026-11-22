//! What the generators' slow tests share, whichever crate they are in
//! (clarity-gen and the stack crates): a scratch server database emptied
//! (scratch), a program served and asked over HTTP (served), and the
//! everything test project prepared for a stack (everything_project).
//! Only tests use it.

pub mod everything_project;
pub mod scratch;
pub mod served;
