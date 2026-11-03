//! The program as callbacks (`clarity_runtime::program::Program`): every
//! call becomes a command with its arguments as JSON, which the caller's
//! program answers `{"value": ...}` or `{"error": Text}` (see the header),
//! through clarity-ui's `WireProgram`, which the browser host uses too.

use std::ffi::{c_char, c_void};
use std::sync::Arc;

use serde_json::{Map, Value};
use ui::program_wire::{Asks, WireProgram};

use crate::{guard, put, ClarityAnswer, ClarityStatus, Failure};

/// The caller's program: its pointer and the function it answers by.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClarityProgramCallbacks {
    pub user: *mut c_void,
    pub call: Option<unsafe extern "C" fn(user: *mut c_void, command: *const c_char, command_len: usize, args: *const c_char, args_len: usize, answer: *mut ClarityAnswer)>,
    pub release: Option<unsafe extern "C" fn(user: *mut c_void)>,
}

/// The program behind the callbacks.
pub struct CallbackProgram {
    callbacks: ClarityProgramCallbacks,
}

// The header asks the caller's `call` to be safe from any thread.
unsafe impl Send for CallbackProgram {}
unsafe impl Sync for CallbackProgram {}

impl Drop for CallbackProgram {
    fn drop(&mut self) {
        if let Some(release) = self.callbacks.release {
            crate::quietly(|| unsafe { release(self.callbacks.user) });
        }
    }
}

/// The handle the caller holds: one hold of the program.
pub struct ClarityProgram {
    pub program: Arc<WireProgram<CallbackProgram>>,
}

impl CallbackProgram {
    pub fn new(callbacks: ClarityProgramCallbacks) -> CallbackProgram {
        CallbackProgram { callbacks }
    }

    /// Asks the program: its whole answer as JSON; none when it set none, or it does not read.
    pub fn ask(&self, command: &str, args: Map<String, Value>) -> Option<Value> {
        let call = self.callbacks.call?;
        let args = Value::Object(args).to_string();
        let mut answer = ClarityAnswer::default();
        unsafe { call(self.callbacks.user, command.as_ptr() as *const c_char, command.len(), args.as_ptr() as *const c_char, args.len(), &mut answer) };
        answer.json()
    }
}

impl Asks for CallbackProgram {
    fn ask(&self, command: &str, args: Map<String, Value>) -> Option<Value> {
        CallbackProgram::ask(self, command, args)
    }
}

/// A program over the caller's callbacks.
///
/// # Safety
/// `callbacks` points at a filled table; `out` at writable memory for a handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_program_new(callbacks: *const ClarityProgramCallbacks, out: *mut *mut ClarityProgram) -> ClarityStatus {
    guard(|| {
        let callbacks = *unsafe { crate::handle(callbacks) }?;
        if callbacks.call.is_none() {
            return Err(Failure::invalid("the program has no call"));
        }
        let program = ClarityProgram { program: Arc::new(WireProgram::new(CallbackProgram::new(callbacks))) };
        unsafe { put(out, Box::into_raw(Box::new(program))) }
    })
}

/// Lets go of the caller's hold of a program.
///
/// # Safety
/// `program` is NULL or a handle from `clarity_program_new`, not freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_program_free(program: *mut ClarityProgram) {
    if !program.is_null() {
        crate::quietly(|| drop(unsafe { Box::from_raw(program) }));
    }
}
