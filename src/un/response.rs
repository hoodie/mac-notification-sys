//! The response a user gave to a notification.
//!
//! Delivered by the delegate after the user interacts with a notification.
//! Callers receive this via the `Future` returned from `send_with_actions`.

use objc2_user_notifications::{
    UNNotificationDefaultActionIdentifier, UNNotificationDismissActionIdentifier,
};

/// What the user did with a notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationResponse {
    /// The identifier of the action the user chose.
    ///
    /// Use [`is_default_action`] and [`is_dismiss_action`] for the two
    /// built-in cases, or compare directly against your own action identifiers.
    ///
    /// [`is_default_action`]: NotificationResponse::is_default_action
    /// [`is_dismiss_action`]: NotificationResponse::is_dismiss_action
    pub action_identifier: String,

    /// The text typed by the user, if this was a text-input (reply) action.
    ///
    /// `None` for regular button actions, the default-action (body click), and
    /// dismiss.  Use [`is_reply`] to check conveniently, or call
    /// `.reply_text.as_deref()` to borrow the text as `&str`.
    ///
    /// [`is_reply`]: NotificationResponse::is_reply
    pub reply_text: Option<String>,
}

impl NotificationResponse {
    /// Returns `true` if the user clicked the notification body (default action).
    ///
    /// Corresponds to [`UNNotificationDefaultActionIdentifier`][apple].
    ///
    /// [apple]: https://developer.apple.com/documentation/usernotifications/unnotificationdefaultactionidentifier
    pub fn is_default_action(&self) -> bool {
        self.action_identifier == unsafe { UNNotificationDefaultActionIdentifier.to_string() }
    }

    /// Returns `true` if the user dismissed the notification without choosing an action.
    ///
    /// Corresponds to [`UNNotificationDismissActionIdentifier`][apple].
    ///
    /// [apple]: https://developer.apple.com/documentation/usernotifications/unnotificationdismissactionidentifier
    pub fn is_dismiss_action(&self) -> bool {
        self.action_identifier == unsafe { UNNotificationDismissActionIdentifier.to_string() }
    }

    /// Returns `true` if the user submitted text via a text-input (reply) action.
    ///
    /// When `true`, [`reply_text`] holds the submitted string.
    ///
    /// [`reply_text`]: NotificationResponse::reply_text
    pub fn is_reply(&self) -> bool {
        self.reply_text.is_some()
    }
}
