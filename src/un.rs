//! High-level, safe wrapper around `UNUserNotificationCenter`.
//!
//! # Building blocks
//!
//! - [`check_bundle`] — verify the process has a bundle identifier.
//! - [`request_auth`] / [`request_auth_blocking`] — ask macOS
//!   for permission to display notifications.
//! - [`Notification`] — builder for the notification payload.
//! - [`send`] / [`send_blocking`] — schedule a fire-and-forget notification.
//! - [`Notification::send_async`] / [`Notification::send_blocking`] — unified
//!   entry point that automatically waits for user interaction when action
//!   buttons are present.
//! - [`run_main_loop_while`] — pump the main thread's `NSRunLoop`; required
//!   by CLI tools and Tokio apps that use actionable notifications.
//!
//! # Threading model
//!
//! All Objective-C work runs on a single, lazily-spawned **worker thread** that
//! continuously pumps its own `NSRunLoop`.
//!
//! **`UNUserNotificationCenter` always delivers `didReceiveNotificationResponse`
//! on the main thread's run loop**, regardless of which thread the delegate was
//! installed from (this is documented Apple behaviour).  The main thread must
//! therefore be pumping `NSRunLoop` while the user is expected to interact.
//!
//! ## macOS app bundles (`NSApplicationMain` / SwiftUI)
//!
//! The framework drives the main run loop automatically; `send_async` works
//! out of the box from any async task or executor.
//!
//! ## Tokio / async-std CLI tools
//!
//! `#[tokio::main]` blocks the main thread inside Tokio's event loop;
//! `NSRunLoop` is never pumped and callbacks never fire.
//!
//! The fix is to **keep the main thread free for `NSRunLoop`** and run the
//! async runtime on background threads:
//!
//! ```no_run
//! use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
//!
//! fn main() {
//!     // Multi-thread runtime lives entirely on background threads.
//!     let rt = tokio::runtime::Builder::new_multi_thread()
//!         .enable_all()
//!         .build()
//!         .unwrap();
//!
//!     let done = Arc::new(AtomicBool::new(false));
//!     let done2 = done.clone();
//!
//!     rt.spawn(async move {
//!         // ... your async code, using send_async() etc. ...
//!         done2.store(true, Ordering::Release);
//!     });
//!
//!     // Main thread pumps NSRunLoop until async work signals completion.
//!     mac_notification_sys::un::run_main_loop_while(|| !done.load(Ordering::Acquire));
//! }
//! ```
//!
//! See `examples/un_actions_tokio.rs` for a complete working example.
//!
//! ## Blocking helper (`send_blocking` from any thread)
//!
//! `Notification::send_blocking` calls `block_on(send_async(...))` and parks
//! the calling thread on the oneshot channel.  The callback still fires on the
//! main thread, so the main thread must be pumping `NSRunLoop` concurrently.
//! For command-line tools that don't use an async runtime at all, see
//! `examples/un_actions.rs` which drives the run loop directly from main.
//!
//! ## "Clear All" caveat
//!
//! If the user clicks **"Clear All"** in Notification Center,
//! `didReceiveNotificationResponse` is never called.  Without a timeout the
//! future will never resolve.  Always set a timeout via [`Notification::timeout`]
//! for actionable notifications.
//!
//! # Requirements
//!
//! The process must have a valid `CFBundleIdentifier` and be code-signed (an
//! ad-hoc signature is sufficient).  See the bundled examples for how to
//! satisfy these requirements via `cargo-bundle`.

use std::future::Future;
use std::time::Duration;

use objc2_foundation::{NSBundle, NSDate, NSDefaultRunLoopMode, NSRunLoop, NSString};
use objc2_user_notifications::{UNMutableNotificationContent, UNNotificationSound};

use crate::Sound;

mod auth;
mod send;
mod worker;

pub mod action;
pub(super) mod delegate;
pub mod response;

pub use action::Action;
pub use auth::{request_auth, request_auth_blocking};
pub use response::NotificationResponse;
pub use send::{send, send_blocking, send_with_actions, send_with_actions_blocking};

/// Pump the main thread's `NSRunLoop` until `should_continue` returns `false`.
///
/// **Must be called from the main thread.**
///
/// `UNUserNotificationCenter` always delivers `didReceiveNotificationResponse`
/// on the main thread's run loop.  In processes where the main thread is
/// occupied by an async runtime (`#[tokio::main]`, `async-std`, …), that run
/// loop is never pumped, so callbacks never fire and `send_async` hangs forever.
///
/// Call this on the main thread while your async work runs on background
/// threads; return `false` from `should_continue` once all work is done.
///
/// # Example
///
/// ```no_run
/// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
///
/// fn main() {
///     let rt = tokio::runtime::Builder::new_multi_thread()
///         .enable_all().build().unwrap();
///
///     let done = Arc::new(AtomicBool::new(false));
///     let done2 = done.clone();
///     rt.spawn(async move {
///         // ... your async work ...
///         done2.store(true, Ordering::Release);
///     });
///
///     mac_notification_sys::un::run_main_loop_while(|| !done.load(Ordering::Acquire));
/// }
/// ```
pub fn run_main_loop_while<F: Fn() -> bool>(should_continue: F) {
    let run_loop = NSRunLoop::mainRunLoop();
    while should_continue() {
        let until = NSDate::dateWithTimeIntervalSinceNow(0.05);
        unsafe { run_loop.runMode_beforeDate(NSDefaultRunLoopMode, &until) };
    }
}

/// Run a future to completion on the main thread while pumping `NSRunLoop`.
///
/// **Must be called from the main thread.**  Drop-in replacement for
/// `futures_lite::future::block_on` for CLI tools that want to `await`
/// notification responses on the main thread.
///
/// The future is polled with a no-op waker; between polls, the main
/// `NSRunLoop` is pumped for up to 50 ms.  This guarantees that
/// `UNUserNotificationCenter` delegate callbacks fire and resolve any
/// `oneshot` channels the future is awaiting.
///
/// Works for any `Future`, including `tokio::task::JoinHandle` — you can
/// spawn work onto a Tokio runtime and `await` the handle inside the future
/// passed here.
///
/// # GUI apps don't need this
///
/// Tauri, winit, AppKit and SwiftUI applications already pump `NSRunLoop`
/// via their main event loop, so `send_async` works without any helper.  This
/// function exists for headless CLI tools.
///
/// # Example
///
/// ```no_run
/// use mac_notification_sys::un::{Notification, block_on_main};
///
/// fn main() {
///     block_on_main(async {
///         let _ = Notification::new()
///             .title("Hi")
///             .send_async()
///             .await;
///     });
/// }
/// ```
pub fn block_on_main<F: Future>(future: F) -> F::Output {
    use std::task::{Context, Poll};

    let mut future = std::pin::pin!(future);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);

    let run_loop = NSRunLoop::mainRunLoop();
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
        let until = NSDate::dateWithTimeIntervalSinceNow(0.05);
        unsafe { run_loop.runMode_beforeDate(NSDefaultRunLoopMode, &until) };
    }
}

/// Errors that can be returned by the `un` module.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The process has no bundle identifier.
    ///
    /// `UNUserNotificationCenter` requires a valid `.app` bundle.
    /// Run via `cargo bundle --example <name>` and open the resulting `.app`.
    #[error("No bundle identifier found. UNUserNotificationCenter requires a valid .app bundle.")]
    NoBundleIdentifier,

    /// macOS rejected the notification scheduling request (e.g. no permission).
    #[error("macOS rejected the notification request")]
    NotificationRejected,

    /// No user interaction was received within the allowed timeout.
    ///
    /// This happens when the notification is cleared without interaction
    /// (e.g. "Clear All" in Notification Center) or the deadline simply
    /// passes before the user acts.
    #[error("Timed out waiting for notification response")]
    ResponseTimeout,
}

/// Verify the process has a bundle identifier.
///
/// `UNUserNotificationCenter` requires a valid app bundle and will crash with
/// an `NSInternalInconsistencyException` without one.  Call this before
/// touching the notification center to get a clean error instead.
pub fn check_bundle() -> Result<(), Error> {
    NSBundle::mainBundle()
        .bundleIdentifier()
        .ok_or(Error::NoBundleIdentifier)?;
    Ok(())
}

/// The payload for a notification — title, body, and optional subtitle / sound.
///
/// Construct with [`Notification::new`] and chain the setter methods.
///
/// # Example
///
/// ```no_run
/// use mac_notification_sys::un::Notification;
///
/// let n = Notification::new()
///     .title("Title")
///     .message("Body text")
///     .subtitle("A subtitle")
///     .sound("Submarine");
/// ```
#[derive(Default)]
pub struct Notification {
    title: String,
    body: String,
    subtitle: Option<String>,
    sound: Option<Sound>,
    actions: Vec<Action>,
    /// How long to wait for user interaction before giving up.
    ///
    /// Only relevant when the notification has action buttons.  `None` means
    /// wait indefinitely (not recommended — see [`Error::ResponseTimeout`]).
    pub(super) action_timeout: Option<Duration>,
}

impl Notification {
    /// Create an empty `Notification`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the notification title.
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_owned();
        self
    }

    /// Set the notification body text.
    pub fn message(mut self, message: &str) -> Self {
        self.body = message.to_owned();
        self
    }

    /// Set an optional subtitle shown below the title.
    pub fn subtitle(mut self, subtitle: &str) -> Self {
        self.subtitle = Some(subtitle.to_owned());
        self
    }

    /// Add an action button to this notification.
    ///
    /// Actions are registered with macOS automatically when the notification is
    /// sent — no manual category registration is needed.
    ///
    /// See [`Action`] for how to mark an action as destructive or requiring
    /// authentication.
    pub fn action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Set how long to wait for the user to interact with this notification.
    ///
    /// Only meaningful when action buttons are present (see [`action`]).  If
    /// the deadline passes before the user responds, [`send`] /
    /// [`send_blocking`] return [`Error::ResponseTimeout`] and the pending
    /// sender is cleaned up automatically.
    ///
    /// Without a timeout, "Clear All" in Notification Center will cause an
    /// indefinite wait.
    ///
    /// [`action`]: Self::action
    /// [`send`]: Self::send_async
    /// [`send_blocking`]: Self::send_blocking
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.action_timeout = Some(duration);
        self
    }

    /// Play the default system notification sound.
    pub fn default_sound(mut self) -> Self {
        self.sound = Some(Sound::Default);
        self
    }

    /// Play a named sound from the app bundle or system sound library.
    pub fn sound<S: Into<Sound>>(mut self, sound: S) -> Self {
        self.sound = Some(sound.into());
        self
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        objc2::rc::Retained<UNMutableNotificationContent>,
        Vec<Action>,
        Option<Duration>,
    ) {
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&self.title));
        content.setBody(&NSString::from_str(&self.body));
        if let Some(sub) = self.subtitle {
            content.setSubtitle(&NSString::from_str(&sub));
        }
        match self.sound {
            None => {}
            Some(Sound::Default) => {
                content.setSound(Some(&UNNotificationSound::defaultSound()));
            }
            Some(Sound::Custom(name)) => {
                content.setSound(Some(&UNNotificationSound::soundNamed(&NSString::from_str(
                    &name,
                ))));
            }
        }
        (content, self.actions, self.action_timeout)
    }
}

impl Notification {
    /// Send the notification, returning a [`Future`] that resolves once macOS
    /// accepts or rejects the scheduling request.
    ///
    /// If the notification has action buttons (added via [`action`]), the
    /// future resolves to `Ok(Some(response))` once the user interacts, or
    /// `Err(`[`Error::ResponseTimeout`]`)` if the optional timeout elapses
    /// first.  Notifications without actions resolve to `Ok(None)` as soon as
    /// macOS accepts the request.
    ///
    /// [`action`]: Self::action
    pub async fn send_async(self) -> Result<Option<NotificationResponse>, Error> {
        if self.actions.is_empty() {
            send(self).await.map(|()| None)
        } else {
            send_with_actions(self).await.map(Some)
        }
    }

    /// Send the notification, blocking the current thread.
    ///
    /// If the notification has action buttons (added via [`action`]), blocks
    /// until the user interacts or the optional timeout elapses.  Returns
    /// `Ok(Some(response))` on interaction, `Ok(None)` for plain
    /// notifications, or `Err` on failure / timeout.
    ///
    /// [`action`]: Self::action
    pub fn send_blocking(self) -> Result<Option<NotificationResponse>, Error> {
        if self.actions.is_empty() {
            send_blocking(self).map(|()| None)
        } else {
            send_with_actions_blocking(self).map(Some)
        }
    }
}
