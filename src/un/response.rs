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
}
