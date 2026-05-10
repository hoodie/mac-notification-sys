//! High-level, safe wrapper around `UNUserNotificationCenter`.
//!
//! # Building blocks
//!
//! - [`check_bundle`] — verify the process has a bundle identifier.
//! - [`request_auth`] / [`request_auth_blocking`] — ask macOS
//!   for permission to display notifications.
//! - [`Notification`] — builder for the notification payload.
//! - [`send`] / [`send_blocking`] — schedule a notification for immediate delivery.
//!
//! # Threading model
//!
//! All Objective-C work executes on a single, lazily-spawned background thread
//! (`worker`) that continuously pumps an `NSRunLoop`.  Notification-specific
//! logic is submitted to that thread as a closure; the result is signalled back
//! via a `futures_channel::oneshot` channel, which is compatible with any async
//! executor (Tokio, async-std, futures, …).
//!
//! **The preferred API is the async one:** simply `.await` [`send`] (or
//! [`Notification::send_async`]) from any async context — Tokio, async-std,
//! futures, or a bare `block_on` — and the calling task will park efficiently
//! while the worker does its job.  Use [`send_blocking`] only from threads
//! that are explicitly allowed to block (e.g. `tokio::task::spawn_blocking`,
//! a plain `std::thread`, or a test body).  Calling [`send_blocking`] from
//! inside a Tokio (or similar) worker task starves the thread pool.
//!
//! # Requirements
//!
//! The process must have a valid `CFBundleIdentifier` and be code-signed (an
//! ad-hoc signature is sufficient).  See the bundled examples for how to
//! satisfy these requirements via `cargo-bundle`.

use objc2_foundation::{NSBundle, NSString};
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
        (content, self.actions)
    }
}

impl Notification {
    /// Send the notification asynchronously, returning a [`Future`] that
    /// resolves once macOS accepts or rejects the request.
    pub async fn send_async(self) -> Result<(), Error> {
        send(self).await
    }

    /// Send the notification synchronously, blocking the current thread until
    /// macOS accepts or rejects the request.
    pub fn send_blocking(self) -> Result<(), Error> {
        send_blocking(self)
    }
}
