//! Notification action buttons and the categories that group them.
//!
//! [`Action`] is the public-facing type for defining action buttons.
//! [`ActionCategory`] is an implementation detail — you do not need to use it
//! directly. When you add actions to a [`Notification`] via
//! [`Notification::action`], a category is synthesised and registered
//! automatically when the notification is sent.
//!
//! [`ActionCategory`] is still available for the advanced use case of
//! **server-defined categories**: when a push notification from your server
//! references a `categoryIdentifier` that your app must pre-register at
//! launch (before any push arrives).
//!
//! [`Notification`]: crate::un::Notification
//! [`Notification::action`]: crate::un::Notification::action

use objc2_foundation::{NSArray, NSSet, NSString};
use objc2_user_notifications::{
    UNNotificationAction, UNNotificationActionOptions, UNNotificationCategory,
    UNNotificationCategoryOptions, UNTextInputNotificationAction, UNUserNotificationCenter,
};

use crate::un::worker;

/// Configuration for a text-input (reply) action.
///
/// Used with [`Action::reply`] to create a button that opens an inline text
/// field when tapped.
#[derive(Debug, Clone)]
pub struct ReplyConfig {
    /// Label on the submit button (e.g. `"Send"`).
    pub button_title: String,
    /// Placeholder text shown in the empty input field.
    pub placeholder: String,
}

/// A single action button shown on a notification.
///
/// Construct with [`Action::new`] for a regular button or [`Action::reply`] for
/// a text-input action, and optionally chain [`destructive`] or
/// [`requires_authentication`].
///
/// [`destructive`]: Action::destructive
/// [`requires_authentication`]: Action::requires_authentication
#[derive(Debug, Clone)]
pub struct Action {
    /// Identifier sent back in [`NotificationResponse::action_identifier`](`crate::un::NotificationResponse::action_identifier`).
    pub identifier: String,
    /// Label shown on the button.
    pub title: String,
    /// If `true` the button is tinted red (destructive style).
    pub destructive: bool,
    /// If `true` the user must authenticate (Touch ID / password) before the
    /// action fires.
    pub requires_authentication: bool,
    /// If `Some`, the action opens an inline text field instead of firing
    /// immediately; the typed text is delivered via
    /// [`NotificationResponse::reply_text`](`crate::un::NotificationResponse::reply_text`).
    pub reply: Option<ReplyConfig>,
}

impl Action {
    /// Create a new action with the given identifier and button title.
    pub fn new(identifier: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            title: title.into(),
            destructive: false,
            requires_authentication: false,
            reply: None,
        }
    }

    /// Create a text-input action that opens a reply field when tapped.
    ///
    /// When the user submits their text, the response is delivered with
    /// [`NotificationResponse::reply_text`](`crate::un::NotificationResponse::reply_text`)
    /// set to `Some(typed_text)`.
    ///
    /// * `button_title` — label on the submit button (e.g. `"Send"`).
    /// * `placeholder`  — greyed-out hint shown in the empty input field.
    pub fn reply(
        identifier: impl Into<String>,
        title: impl Into<String>,
        button_title: impl Into<String>,
        placeholder: impl Into<String>,
    ) -> Self {
        Self {
            identifier: identifier.into(),
            title: title.into(),
            destructive: false,
            requires_authentication: false,
            reply: Some(ReplyConfig {
                button_title: button_title.into(),
                placeholder: placeholder.into(),
            }),
        }
    }

    /// Mark the action as destructive.
    ///
    /// Sets [`UNNotificationActionOptions::Destructive`][apple-opts]. Note that
    /// macOS does not visually distinguish destructive actions in the notification
    /// banner UI, so this has no visible effect at runtime.
    ///
    /// [apple-opts]: https://developer.apple.com/documentation/usernotifications/unnotificationactionoptions/destructive
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    /// Require the user to authenticate (Touch ID / password) before this action fires.
    ///
    /// Sets [`UNNotificationActionOptions::AuthenticationRequired`][apple-opts].
    ///
    /// [apple-opts]: https://developer.apple.com/documentation/usernotifications/unnotificationactionoptions/authenticationrequired
    pub fn requires_authentication(mut self) -> Self {
        self.requires_authentication = true;
        self
    }

    /// Build the Objective-C action object.
    ///
    /// Returns a `UNTextInputNotificationAction` (upcast to its superclass)
    /// when `reply` is configured, and a plain `UNNotificationAction` otherwise.
    pub(crate) fn build(&self) -> objc2::rc::Retained<UNNotificationAction> {
        let mut options = UNNotificationActionOptions::empty();
        if self.destructive {
            options |= UNNotificationActionOptions::Destructive;
        }
        if self.requires_authentication {
            options |= UNNotificationActionOptions::AuthenticationRequired;
        }
        if let Some(reply) = &self.reply {
            // UNTextInputNotificationAction is a direct subclass of
            // UNNotificationAction; into_super() performs the safe upcast.
            UNTextInputNotificationAction::actionWithIdentifier_title_options_textInputButtonTitle_textInputPlaceholder(
                &NSString::from_str(&self.identifier),
                &NSString::from_str(&self.title),
                options,
                &NSString::from_str(&reply.button_title),
                &NSString::from_str(&reply.placeholder),
            )
            .into_super()
        } else {
            UNNotificationAction::actionWithIdentifier_title_options(
                &NSString::from_str(&self.identifier),
                &NSString::from_str(&self.title),
                options,
            )
        }
    }
}

// ── ActionCategory ───────────────────────────────────────────────────────────

/// A named group of [`Action`]s that macOS attaches to a notification.
///
/// Register the category with [`ActionCategory::register`] before sending any
/// notification that references its identifier.
#[derive(Debug, Clone)]
pub struct ActionCategory {
    /// Identifier you pass to `UNMutableNotificationContent::setCategoryIdentifier`.
    pub identifier: String,
    pub(crate) actions: Vec<Action>,
}

impl ActionCategory {
    /// Create a new category with the given identifier.
    pub fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            actions: Vec::new(),
        }
    }

    /// Create a category from an existing list of actions.
    pub(crate) fn from_actions(identifier: &str, actions: Vec<Action>) -> Self {
        Self {
            identifier: identifier.to_owned(),
            actions,
        }
    }

    /// Append an [`Action`] button to this category.
    pub fn action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Build the `UNNotificationCategory` object.
    fn build_category(&self) -> objc2::rc::Retained<UNNotificationCategory> {
        let un_actions: Vec<_> = self.actions.iter().map(|a| a.build()).collect();
        let actions_array = NSArray::from_retained_slice(&un_actions);
        UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
            &NSString::from_str(&self.identifier),
            &actions_array,
            &NSArray::new(),
            UNNotificationCategoryOptions::CustomDismissAction,
        )
    }

    /// Register this category synchronously on the current thread.
    ///
    /// Must only be called from the worker thread inside a dispatched closure.
    pub(crate) fn register_now(&self) {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let category = self.build_category();
        let new_set = NSSet::from_retained_slice(&[category]);
        center.setNotificationCategories(&new_set);
        log::debug!("un::action: registered category '{}'", self.identifier);
    }

    /// Register this category with `UNUserNotificationCenter`.
    ///
    /// Only needed for **server-defined categories** that must exist before a
    /// push notification arrives. For local notifications with action buttons,
    /// use [`Notification::action`] instead — registration happens automatically.
    ///
    /// Dispatches to the worker thread and returns immediately.
    ///
    /// [`Notification::action`]: crate::un::Notification::action
    pub fn register(self) {
        worker::dispatch(move || self.register_now());
    }
}
