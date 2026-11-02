//! Work off the screen's thread (a file dialog, a long handler, a sound
//! being read): an element asks for it with `Cx::later`, it runs on a
//! thread of its own, and what it gives comes back to the element on the
//! screen's thread (`Element::on_later`) the next time the host lets the
//! screen take it (`Screen::take_later`). The host is woken when one is
//! done, so a window that waits for nothing else still hears it.

use std::any::Any;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::element::ElementId;

/// What work gives back: the element downcasts it to what it asked for.
pub type Done = Box<dyn Any + Send>;

/// Work to run off the screen's thread.
pub type Work = Box<dyn FnOnce() -> Done + Send>;

/// Wakes the host when work is done; a host without its own loop gives none.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// The work of one screen: what runs, and what came back.
pub struct Queue {
    sender: Sender<(ElementId, Done)>,
    receiver: Receiver<(ElementId, Done)>,
    waker: Arc<Mutex<Option<Waker>>>,
    running: Arc<std::sync::atomic::AtomicUsize>,
}

impl Default for Queue {
    fn default() -> Self {
        Queue::new()
    }
}

impl Queue {
    pub fn new() -> Queue {
        let (sender, receiver) = channel();
        Queue { sender, receiver, waker: Arc::new(Mutex::new(None)), running: Arc::new(std::sync::atomic::AtomicUsize::new(0)) }
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
