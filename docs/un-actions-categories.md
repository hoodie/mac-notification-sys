# UNNotificationAction & UNNotificationCategory

> Source: `objc2-user-notifications` v0.3.2  
> Crate: [`objc2_user_notifications`](https://docs.rs/objc2-user-notifications/latest/objc2_user_notifications/)  
> License: Zlib OR Apache-2.0 OR MIT

---

## `UNNotificationAction`

[docs.rs](https://docs.rs/objc2-user-notifications/latest/objc2_user_notifications/struct.UNNotificationAction.html)

```rust
pub struct UNNotificationAction { /* private fields */ }
```

> Available on **crate feature `UNNotificationAction`** only.

See also [Apple's documentation](https://developer.apple.com/documentation/usernotifications/unnotificationaction).

### Instance Methods

| Method              | Return Type                                  | Notes                                       |
| ------------------- | -------------------------------------------- | ------------------------------------------- |
| `identifier(&self)` | `Retained<NSString>`                         |                                             |
| `title(&self)`      | `Retained<NSString>`                         |                                             |
| `options(&self)`    | `UNNotificationActionOptions`                |                                             |
| `icon(&self)`       | `Option<Retained<UNNotificationActionIcon>>` | Feature `UNNotificationActionIcon` required |

### Constructors

```rust
pub fn actionWithIdentifier_title_options(
    identifier: &NSString,
    title: &NSString,
    options: UNNotificationActionOptions,
) -> Retained<Self>
```

```rust
// Feature `UNNotificationActionIcon` required
pub fn actionWithIdentifier_title_options_icon(
    identifier: &NSString,
    title: &NSString,
    options: UNNotificationActionOptions,
    icon: Option<&UNNotificationActionIcon>,
) -> Retained<Self>
```

```rust
pub unsafe fn init(this: Allocated<Self>) -> Retained<Self>
pub unsafe fn new() -> Retained<Self>  // from NSObject
```

### Trait Implementations

| Trait                                    | Notes                                               |
| ---------------------------------------- | --------------------------------------------------- |
| `ClassType`                              | `NAME = "UNNotificationAction"`, `Super = NSObject` |
| `CopyingHelper`                          | `Result = UNNotificationAction`                     |
| `NSCopying`                              |                                                     |
| `NSCoding`                               |                                                     |
| `NSSecureCoding`                         |                                                     |
| `NSObjectProtocol`                       |                                                     |
| `Debug`, `Hash`, `PartialEq`, `Eq`       |                                                     |
| `Deref<Target = NSObject>`               |                                                     |
| `AsRef<AnyObject>`, `AsRef<NSObject>`    |                                                     |
| `Message`, `RefEncode`, `DowncastTarget` |                                                     |

**Note:** `UNTextInputNotificationAction` implements `AsRef<UNNotificationAction>` and `Borrow<UNNotificationAction>` — it is a subclass.

### Auto Trait Implementations

This type is **not** `Send`, `Sync`, `Freeze`, `Unpin`, `RefUnwindSafe`, or `UnwindSafe` — typical for Objective-C types.

---

## `UNNotificationCategory`

[docs.rs](https://docs.rs/objc2-user-notifications/latest/objc2_user_notifications/struct.UNNotificationCategory.html)

```rust
pub struct UNNotificationCategory { /* private fields */ }
```

> Available on **crate feature `UNNotificationCategory`** only.

See also [Apple's documentation](https://developer.apple.com/documentation/usernotifications/unnotificationcategory).

### Instance Methods

| Method                                 | Return Type                               | Notes                                                                              |
| -------------------------------------- | ----------------------------------------- | ---------------------------------------------------------------------------------- |
| `identifier(&self)`                    | `Retained<NSString>`                      |                                                                                    |
| `actions(&self)`                       | `Retained<NSArray<UNNotificationAction>>` | Feature `UNNotificationAction` required                                            |
| `intentIdentifiers(&self)`             | `Retained<NSArray<NSString>>`             |                                                                                    |
| `options(&self)`                       | `UNNotificationCategoryOptions`           |                                                                                    |
| `hiddenPreviewsBodyPlaceholder(&self)` | `Retained<NSString>`                      |                                                                                    |
| `categorySummaryFormat(&self)`         | `Retained<NSString>`                      | Format string for grouped notification summaries, e.g. `"%u new messages from %@"` |

### Constructors

All constructors below require feature `UNNotificationAction`.

```rust
pub fn categoryWithIdentifier_actions_intentIdentifiers_options(
    identifier: &NSString,
    actions: &NSArray<UNNotificationAction>,
    intent_identifiers: &NSArray<NSString>,
    options: UNNotificationCategoryOptions,
) -> Retained<Self>
```

```rust
pub fn categoryWithIdentifier_actions_intentIdentifiers_hiddenPreviewsBodyPlaceholder_options(
    identifier: &NSString,
    actions: &NSArray<UNNotificationAction>,
    intent_identifiers: &NSArray<NSString>,
    hidden_previews_body_placeholder: &NSString,
    options: UNNotificationCategoryOptions,
) -> Retained<Self>
```

```rust
pub fn categoryWithIdentifier_actions_intentIdentifiers_hiddenPreviewsBodyPlaceholder_categorySummaryFormat_options(
    identifier: &NSString,
    actions: &NSArray<UNNotificationAction>,
    intent_identifiers: &NSArray<NSString>,
    hidden_previews_body_placeholder: Option<&NSString>,
    category_summary_format: Option<&NSString>,
    options: UNNotificationCategoryOptions,
) -> Retained<Self>
```

```rust
pub unsafe fn init(this: Allocated<Self>) -> Retained<Self>
pub unsafe fn new() -> Retained<Self>  // from NSObject
```

### Trait Implementations

| Trait                                    | Notes                                                 |
| ---------------------------------------- | ----------------------------------------------------- |
| `ClassType`                              | `NAME = "UNNotificationCategory"`, `Super = NSObject` |
| `CopyingHelper`                          | `Result = UNNotificationCategory`                     |
| `NSCopying`                              |                                                       |
| `NSCoding`                               |                                                       |
| `NSSecureCoding`                         |                                                       |
| `NSObjectProtocol`                       |                                                       |
| `Debug`, `Hash`, `PartialEq`, `Eq`       |                                                       |
| `Deref<Target = NSObject>`               |                                                       |
| `AsRef<AnyObject>`, `AsRef<NSObject>`    |                                                       |
| `Message`, `RefEncode`, `DowncastTarget` |                                                       |

### Auto Trait Implementations

This type is **not** `Send`, `Sync`, `Freeze`, `Unpin`, `RefUnwindSafe`, or `UnwindSafe` — typical for Objective-C types.

---

## Feature Flags Summary

| Feature flag               | Unlocks                                                                   |
| -------------------------- | ------------------------------------------------------------------------- |
| `UNNotificationAction`     | `UNNotificationAction` struct and related constructors                    |
| `UNNotificationCategory`   | `UNNotificationCategory` struct                                           |
| `UNNotificationActionIcon` | `icon()` getter and `actionWithIdentifier_title_options_icon` constructor |

## Cargo.toml Example

```toml
[dependencies]
objc2-user-notifications = { version = "0.3.2", features = [
    "UNNotificationAction",
    "UNNotificationCategory",
] }
```
