//! Thin bridge between async Rust and the macOS run-loop world.
//!
//! Owns a single background thread that pumps `NSRunLoop` continuously and runs arbitrary closures submitted from any thread.

use super::delegate;
use std::{
    sync::{OnceLock, mpsc},
    thread,
};

use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop};

/// How long each `runMode:beforeDate:` slice lasts when the worker is idle.
/// Short enough for responsive callback delivery; long enough not to busy-spin.
const TICK_SECS: f64 = 0.05;

type Task = Box<dyn FnOnce() + Send + 'static>;

static WORKER: OnceLock<mpsc::Sender<Task>> = OnceLock::new();

fn handle() -> &'static mpsc::Sender<Task> {
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Task>();
        thread::Builder::new()
            .name("mac-notification-sys".into())
            .spawn(move || {
                log::debug!("un::worker: thread started");
                worker_loop(rx);
            })
            .expect("failed to spawn UN worker thread");
        tx
    })
}

fn worker_loop(rx: mpsc::Receiver<Task>) {
    // Install the delegate here so it is ready before any notification is
    // scheduled.  Note: UNUserNotificationCenter always delivers
    // `didReceiveNotificationResponse` on the *main* thread's run loop,
    // regardless of which thread the delegate was installed from.  The main
    // thread must pump NSRunLoop while waiting for a response — see
    // `run_main_loop_while` in the public API.
    delegate::install();

    let run_loop = NSRunLoop::currentRunLoop();
    loop {
        while let Ok(task) = rx.try_recv() {
            log::debug!("un::worker: running job");
            // Catch panics so a buggy closure does not kill the entire worker.
            // The closure's oneshot::Sender drops on panic, cancelling the
            // receiver — the caller sees Err(NotificationRejected) rather than a hung future.
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(task)) {
                Ok(()) => log::debug!("un::worker: task done"),
                Err(_) => log::error!("un::worker: task panicked"),
            }
        }
        // Yield to the run loop so completion handlers and (Phase 2b) delegate
        // methods can fire. runMode:beforeDate: returns early when an input
        // source fires, so this is not a busy loop.
        let until = NSDate::dateWithTimeIntervalSinceNow(TICK_SECS);
        // SAFETY: NSDefaultRunLoopMode is a valid, non-null mode string.
        let _ = unsafe { run_loop.runMode_beforeDate(NSDefaultRunLoopMode, &until) };
    }
}

/// Schedule `f` to run on the worker thread.
///
/// `f` is executed between run-loop slices. Any `oneshot::Sender` captured by
/// `f` will be resolved once the corresponding Objective-C callback fires.
///
/// If the worker thread has died the send silently fails; the caller's oneshot
/// receiver will resolve to `Err(Canceled)`, which callers should map to
/// `Error::NotificationRejected`.
pub(super) fn dispatch<F: FnOnce() + Send + 'static>(f: F) {
    if handle().send(Box::new(f)).is_err() {
        log::error!("un::worker: dispatch failed (worker thread is gone)");
    }
}
