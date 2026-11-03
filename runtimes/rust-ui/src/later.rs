//! Work off the screen's thread (a file dialog, a long handler, a sound
//! being read): an element asks for it with `Cx::later`, it runs on a
//! thread of its own, and what it gives comes back to the element on the
//! screen's thread (`Element::on_later`) the next time the host lets the
//! screen take it (`Screen::take_later`). The host is woken when one is
//! done, so a window that waits for nothing else still hears it. A browser
//! build has no threads (its Skia is built without them): there the work
//! runs at once on the screen's thread, which is the page's worker, and
//! comes back the same way; a long call's request there is sent without
//! waiting (`Asked`, through the host's `AskLater`) and its answer comes
//! back the same way when the server gives it.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value};

use crate::element::ElementId;

/// What work gives back: the element downcasts it to what it asked for.
pub type Done = Box<dyn Any + Send>;

/// Work to run off the screen's thread.
pub type Work = Box<dyn FnOnce() -> Done + Send>;

/// Wakes the host when work is done; a host without its own loop gives none.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// What a request's answer gives back: the program's whole answer, none when it answered none.
pub type Answered = Box<dyn FnOnce(Option<Value>) -> Done + Send>;

/// Sends the program `command` with `args` and returns at once; the answer
/// goes to the callback when it comes (the browser host's server).
pub type AskLater = Rc<dyn Fn(&str, Map<String, Value>, Box<dyn FnOnce(Option<Value>)>)>;

/// Work that is one request of the program: sent without waiting where the
/// host can (`AskLater`), else `work` runs off the screen's thread as any.
pub struct Asked {
    pub command: String,
    pub args: Map<String, Value>,
    pub answered: Answered,
    pub work: Work,
}

/// The work of one screen: what runs, and what came back.
pub struct Queue {
    sender: Sender<(ElementId, Done)>,
    receiver: Receiver<(ElementId, Done)>,
    waker: Arc<Mutex<Option<Waker>>>,
    running: Arc<std::sync::atomic::AtomicUsize>,
    asker: RefCell<Option<AskLater>>,
    asking: Rc<std::cell::Cell<usize>>,
}

impl Default for Queue {
    fn default() -> Self {
        Queue::new()
    }
}

impl Queue {
    pub fn new() -> Queue {
        let (sender, receiver) = channel();
        Queue { sender, receiver, waker: Arc::new(Mutex::new(None)), running: Arc::new(std::sync::atomic::AtomicUsize::new(0)), asker: RefCell::new(None), asking: Rc::default() }
    }

    /// The host is woken by `waker` whenever work is done.
    pub fn set_waker(&self, waker: Waker) {
        if let Ok(mut w) = self.waker.lock() {
            *w = Some(waker);
        }
    }

    /// Runs `work` on a thread of its own; what it gives goes to `element`.
    /// A panic in it gives nothing back, and the screen goes on.
    pub fn start(&self, element: ElementId, work: Work) {
        let sender = self.sender.clone();
        let waker = self.waker.clone();
        let running = self.running.clone();
        running.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if cfg!(target_os = "emscripten") {
            let given = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
            if let Ok(done) = given {
                let _ = sender.send((element, done));
            }
            running.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            return;
        }
        let spawned = std::thread::Builder::new().name("clarity-later".to_string()).spawn(move || {
            let given = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
            if let Ok(done) = given {
                let _ = sender.send((element, done));
            }
            running.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            if let Some(wake) = waker.lock().ok().and_then(|w| w.clone()) {
                wake();
            }
        });
        if spawned.is_err() {
            self.running.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Requests of the program are sent through `asker` without waiting.
    pub fn set_asker(&self, asker: AskLater) {
        *self.asker.borrow_mut() = Some(asker);
    }

    /// Sends `asked` through the asker, its answer coming back to `element`;
    /// without one, its work runs as any (`start`).
    pub fn ask(&self, element: ElementId, asked: Asked) {
        let asker = self.asker.borrow().clone();
        let Some(asker) = asker else {
            return self.start(element, asked.work);
        };
        let sender = self.sender.clone();
        let asking = self.asking.clone();
        let answered = asked.answered;
        asking.set(asking.get() + 1);
        asker(
            &asked.command,
            asked.args,
            Box::new(move |answer| {
                let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| answered(answer)));
                if let Ok(done) = done {
                    let _ = sender.send((element, done));
                }
                asking.set(asking.get().saturating_sub(1));
            }),
        );
    }

    /// How many requests sent through the asker wait for their answer.
    pub fn asking(&self) -> usize {
        self.asking.get()
    }

    /// What came back since the last time, in the order it came.
    pub fn take(&self) -> Vec<(ElementId, Done)> {
        self.receiver.try_iter().collect()
    }

    /// How much work is still running.
    pub fn running(&self) -> usize {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Waits until the work running now is done (a test's way to finish it), at most `limit`.
    pub fn wait(&self, limit: std::time::Duration) {
        let until = std::time::Instant::now() + limit;
        while self.running() > 0 && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
}
