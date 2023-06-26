//! Custom structs and enums for mac-notification-sys.

use objc_foundation::{INSDictionary, INSString, NSDictionary, NSString};
use objc_id::Id;
use std::default::Default;
use std::ops::Deref;
use std::path::PathBuf;

use crate::error::{NotificationError, NotificationResult};
use crate::{
    block_for_notification, ensure, ensure_application_set, send_notification_delegate, sys,
};

/// Possible actions accessible through the main button of the notification
#[derive(Clone, Debug)]
pub enum MainButton<'a> {
    /// Display a single action with the given name
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let _ = MainButton::SingleAction("Action name");
    /// ```
    SingleAction(&'a str),

    /// Display a dropdown with the given title, with a list of actions with given names
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let _ = MainButton::DropdownActions("Dropdown name", &["Action 1", "Action 2"]);
    /// ```
    DropdownActions(&'a str, &'a [&'a str]),

    /// Display a text input field with the given placeholder
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let _ = MainButton::Response("Enter some text...");
    /// ```
    Response(&'a str),
}

/// Options to further customize the notification
#[derive(Clone, Default)]
pub struct Notification<'a> {
    pub(crate) title: &'a str,
    pub(crate) subtitle: Option<&'a str>,
    pub(crate) message: &'a str,
    pub(crate) main_button: Option<MainButton<'a>>,
    pub(crate) close_button: Option<&'a str>,
    pub(crate) app_icon: Option<&'a str>,
    pub(crate) content_image: Option<&'a str>,
    pub(crate) delivery_date: Option<f64>,
    pub(crate) sound: Option<&'a str>,
    pub(crate) asynchronous: Option<bool>,
}

impl<'a> Notification<'a> {
    /// Create a Notification to further customize the notification
    pub fn new() -> Self {
        Default::default()
    }

    /// Set `title` field
    pub fn title(&mut self, title: &'a str) -> &mut Self {
        self.title = title;
        self
    }

    /// Set `subtitle` field
    pub fn subtitle(&mut self, subtitle: &'a str) -> &mut Self {
        self.subtitle = Some(subtitle);
        self
    }

    /// Set `subtitle` field
    pub fn maybe_subtitle(&mut self, subtitle: Option<&'a str>) -> &mut Self {
        self.subtitle = subtitle;
        self
    }

    /// Set `message` field
    pub fn message(&mut self, message: &'a str) -> &mut Self {
        self.message = message;
        self
    }

    /// Allow actions through a main button
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let _ = Notification::new().main_button(MainButton::SingleAction("Main button"));
    /// ```
    pub fn main_button(&mut self, main_button: MainButton<'a>) -> &mut Self {
        self.main_button = Some(main_button);
        self
    }

    /// Display a close button with the given name
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let _ = Notification::new().close_button("Close");
    /// ```
    pub fn close_button(&mut self, close_button: &'a str) -> &mut Self {
        self.close_button = Some(close_button);
        self
    }

    /// Display an icon on the left side of the notification
    ///
    /// NOTE: The icon of the app associated to the bundle will be displayed next to the notification title
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let _ = Notification::new().app_icon("/path/to/icon.icns");
    /// ```
    pub fn app_icon(&mut self, app_icon: &'a str) -> &mut Self {
        self.app_icon = Some(app_icon);
        self
    }

    /// Display an image on the right side of the notification
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let _ = Notification::new().content_image("/path/to/image.png");
    /// ```
    pub fn content_image(&mut self, content_image: &'a str) -> &mut Self {
        self.content_image = Some(content_image);
        self
    }

    /// Schedule the notification to be delivered at a later time
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let stamp = time::OffsetDateTime::now_utc().unix_timestamp() as f64 + 5.;
    /// let _ = Notification::new().delivery_date(stamp);
    /// ```
    pub fn delivery_date(&mut self, delivery_date: f64) -> &mut Self {
        self.delivery_date = Some(delivery_date);
        self
    }

    /// Play a system sound when the notification is delivered
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let _ = Notification::new().sound("Blow");
    /// ```
    pub fn sound(&mut self, sound: &'a str) -> &mut Self {
        self.sound = Some(sound);
        self
    }

    /// Play a system sound when the notification is delivered
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let _ = Notification::new().sound("Blow");
    /// ```
    pub fn maybe_sound(&mut self, sound: Option<&'a str>) -> &mut Self {
        self.sound = sound;
        self
    }

    /// Deliver the notification asynchronously (without waiting for an interaction).
    ///
    /// Note: Setting this to true is equivalent to a fire-and-forget.
    ///
    /// # Example:
    ///
    /// ```no_run
    /// # use mac_notification_sys::*;
    /// let _ = Notification::new().asynchronous(true);
    /// ```
    pub fn asynchronous(&mut self, asynchronous: bool) -> &mut Self {
        self.asynchronous = Some(asynchronous);
        self
    }

    /// Convert the Notification to an Objective C NSDictionary
    pub(crate) fn to_dictionary(&self) -> Id<NSDictionary<NSString, NSString>> {
        // TODO: If possible, find a way to simplify this so I don't have to manually convert struct to NSDictionary
        let keys = &[
            &*NSString::from_str("mainButtonLabel"),
            &*NSString::from_str("actions"),
            &*NSString::from_str("closeButtonLabel"),
            &*NSString::from_str("appIcon"),
            &*NSString::from_str("contentImage"),
            &*NSString::from_str("response"),
            &*NSString::from_str("deliveryDate"),
            &*NSString::from_str("asynchronous"),
            &*NSString::from_str("sound"),
        ];
        let (main_button_label, actions, is_response): (&str, &[&str], bool) =
            match &self.main_button {
                Some(main_button) => match main_button {
                    MainButton::SingleAction(main_button_label) => (main_button_label, &[], false),
                    MainButton::DropdownActions(main_button_label, actions) => {
                        (main_button_label, actions, false)
                    }
                    MainButton::Response(response) => (response, &[], true),
                },
                None => ("", &[], false),
            };

        let vals = vec![
            NSString::from_str(main_button_label),
            // TODO: Find a way to support NSArray as a NSDictionary Value rather than JUST NSString so I don't have to convert array to string and back
            NSString::from_str(&actions.join(",")),
            NSString::from_str(self.close_button.unwrap_or("")),
            NSString::from_str(self.app_icon.unwrap_or("")),
            NSString::from_str(self.content_image.unwrap_or("")),
            // TODO: Same as above, if NSDictionary could support multiple types, this could be a boolean
            NSString::from_str(if is_response { "yes" } else { "" }),
            NSString::from_str(&match self.delivery_date {
                Some(delivery_date) => delivery_date.to_string(),
                _ => String::new(),
            }),
            // TODO: Same as above, if NSDictionary could support multiple types, this could be a boolean
            NSString::from_str(match self.asynchronous {
                Some(true) => "yes",
                _ => "no",
            }),
            NSString::from_str(match self.sound {
                Some(sound) if check_sound(sound) => sound,
                _ => "_mute",
            }),
        ];
        NSDictionary::from_keys_and_objects(keys, vals)
    }

    /// Delivers a new notification
    ///
    /// Returns a `NotificationError` if a notification could not be delivered
    ///
    pub fn send(&self) -> NotificationResult<NotificationResponse> {
        if let Some(delivery_date) = self.delivery_date {
            ensure!(
                delivery_date >= time::OffsetDateTime::now_utc().unix_timestamp() as f64,
                NotificationError::ScheduleInThePast
            );
        };

        let options = self.to_dictionary();

        ensure_application_set()?;

        let dictionary_response = unsafe {
            sys::sendNotification(
                NSString::from_str(self.title).deref(),
                NSString::from_str(self.subtitle.unwrap_or("")).deref(),
                NSString::from_str(self.message).deref(),
                options.deref(),
            )
        };
        ensure!(
            dictionary_response
                .deref()
                .object_for(NSString::from_str("error").deref())
                .is_none(),
            NotificationError::UnableToDeliver
        );

        let response = NotificationResponse::from_dictionary(dictionary_response);

        Ok(response)
    }

    #[allow(missing_docs)]
    pub fn send_delegated(&self) -> NotificationResult<NotificationHandle> {
        send_notification_delegate(self.title, self.subtitle, self.message, Some(self))
    }
}

#[derive(Debug)]
pub struct NotificationHandle(pub(crate) sys::Handle);

impl NotificationHandle {
    pub(crate) fn as_inner(&self) -> &sys::Handle {
        &self.0
    }

    pub fn wait(self) -> NotificationResult<NotificationResponse> {
        block_for_notification(self)
    }
}

/// Response from the Notification
#[derive(Clone, Debug)]
pub enum NotificationResponse {
    /// No interaction has occured
    None,
    /// User clicked on an action button with the given name
    ActionButton(String),
    /// User clicked on the close button with the given name
    CloseButton(String),
    /// User clicked the notification directly
    Click,
    /// User submitted text to the input text field
    Reply(String),
}

impl NotificationResponse {
    /// Create a NotificationResponse from the given Objective C NSDictionary
    pub(crate) fn from_dictionary(dictionary: Id<NSDictionary<NSString, NSString>>) -> Self {
        let dictionary = dictionary.deref();

        let activation_type = dictionary
            .object_for(NSString::from_str("activationType").deref())
            .map(|str| str.deref().as_str().to_owned());

        match activation_type.as_deref() {
            Some("actionClicked") => NotificationResponse::ActionButton(
                match dictionary.object_for(NSString::from_str("activationValue").deref()) {
                    Some(str) => str.deref().as_str().to_owned(),
                    None => String::from(""),
                },
            ),
            Some("closeClicked") => NotificationResponse::CloseButton(
                match dictionary.object_for(NSString::from_str("activationValue").deref()) {
                    Some(str) => str.deref().as_str().to_owned(),
                    None => String::from(""),
                },
            ),
            Some("replied") => NotificationResponse::Reply(
                match dictionary.object_for(NSString::from_str("activationValue").deref()) {
                    Some(str) => str.deref().as_str().to_owned(),
                    None => String::from(""),
                },
            ),
            Some("contentsClicked") => NotificationResponse::Click,
            _ => NotificationResponse::None,
        }
    }
}

pub(crate) fn check_sound(sound_name: &str) -> bool {
    dirs_next::home_dir()
        .map(|path| path.join("/Library/Sounds/"))
        .into_iter()
        .chain(
            [
                "/Library/Sounds/",
                "/Network/Library/Sounds/",
                "/System/Library/Sounds/",
            ]
            .iter()
            .map(PathBuf::from),
        )
        .map(|sound_path| sound_path.join(format!("{}.aiff", sound_name)))
        .any(|some_path| some_path.exists())
}
