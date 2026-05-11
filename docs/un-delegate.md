# `UNUserNotificationCenterDelegate`

**Crate:** [`objc2-user-notifications`](https://docs.rs/objc2-user-notifications/latest/objc2_user_notifications/) v0.3.2  
**Source:** [docs.rs](https://docs.rs/objc2-user-notifications/latest/objc2_user_notifications/trait.UNUserNotificationCenterDelegate.html)  
**Apple docs:** [UNUserNotificationCenter – Apple Developer](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter)

---

## Trait Definition

```mac-notification-sys2/docs/un-delegate.md#L1-1
pub unsafe trait UNUserNotificationCenterDelegate: NSObjectProtocol {
    // Provided methods
    fn userNotificationCenter_willPresentNotification_withCompletionHandler(
        &self,
        center: &UNUserNotificationCenter,
        notification: &UNNotification,
        completion_handler: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
    ) where Self: Sized + Message { ... }

    fn userNotificationCenter_didReceiveNotificationResponse_withCompletionHandler(
        &self,
        center: &UNUserNotificationCenter,
        response: &UNNotificationResponse,
        completion_handler: &DynBlock<dyn Fn()>,
    ) where Self: Sized + Message { ... }

    fn userNotificationCenter_openSettingsForNotification(
        &self,
        center: &UNUserNotificationCenter,
        notification: Option<&UNNotification>,
    ) where Self: Sized + Message { ... }
}
```

> Available on **crate feature `UNUserNotificationCenter`** only.

---

## Provided Methods

### `userNotificationCenter_willPresentNotification_withCompletionHandler`

```mac-notification-sys2/docs/un-delegate.md#L1-1
fn userNotificationCenter_willPresentNotification_withCompletionHandler(
    &self,
    center: &UNUserNotificationCenter,
    notification: &UNNotification,
    completion_handler: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
) where Self: Sized + Message
```

Called when a notification is about to be delivered to a foregrounded app.

> Available on **crate features `UNNotification` and `block2`** only.

---

### `userNotificationCenter_didReceiveNotificationResponse_withCompletionHandler`

```mac-notification-sys2/docs/un-delegate.md#L1-1
fn userNotificationCenter_didReceiveNotificationResponse_withCompletionHandler(
    &self,
    center: &UNUserNotificationCenter,
    response: &UNNotificationResponse,
    completion_handler: &DynBlock<dyn Fn()>,
) where Self: Sized + Message
```

Called when the user responded to a notification (e.g. tapped an action or dismissed it).

> Available on **crate features `UNNotificationResponse` and `block2`** only.

---

### `userNotificationCenter_openSettingsForNotification`

```mac-notification-sys2/docs/un-delegate.md#L1-1
fn userNotificationCenter_openSettingsForNotification(
    &self,
    center: &UNUserNotificationCenter,
    notification: Option<&UNNotification>,
) where Self: Sized + Message
```

Called when the user taps "Settings" in a notification. `notification` is `None` if the
settings were opened from the app itself rather than from a specific notification.

> Available on **crate feature `UNNotification`** only.

---

## Trait Implementations

### `ProtocolType for dyn UNUserNotificationCenterDelegate`

| Item                 | Value                                                                     |
| -------------------- | ------------------------------------------------------------------------- |
| `NAME: &'static str` | `"UNUserNotificationCenterDelegate"`                                      |
| `fn protocol()`      | Returns `Option<&'static AnyProtocol>` – the Objective-C protocol object. |

### `ImplementedBy<T> for dyn UNUserNotificationCenterDelegate`

```mac-notification-sys2/docs/un-delegate.md#L1-1
impl<T> ImplementedBy<T> for dyn UNUserNotificationCenterDelegate
where
    T: ?Sized + Message + UNUserNotificationCenterDelegate
```

### `UNUserNotificationCenterDelegate for ProtocolObject<T>`

```mac-notification-sys2/docs/un-delegate.md#L1-1
impl<T> UNUserNotificationCenterDelegate for ProtocolObject<T>
where
    T: ?Sized + UNUserNotificationCenterDelegate
```

Blanket impl that forwards the delegate methods through `ProtocolObject<T>`.

---

## Required Cargo Features

| Method                                                                        | Features needed                                                |
| ----------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `userNotificationCenter_willPresentNotification_withCompletionHandler`        | `UNUserNotificationCenter`, `UNNotification`, `block2`         |
| `userNotificationCenter_didReceiveNotificationResponse_withCompletionHandler` | `UNUserNotificationCenter`, `UNNotificationResponse`, `block2` |
| `userNotificationCenter_openSettingsForNotification`                          | `UNUserNotificationCenter`, `UNNotification`                   |
