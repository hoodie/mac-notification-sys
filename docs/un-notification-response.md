# `UNNotificationResponse`

**Crate:** `objc2-user-notifications` v0.3.2  
**Docs:** <https://docs.rs/objc2-user-notifications/latest/objc2_user_notifications/struct.UNNotificationResponse.html>  
**Apple docs:** <https://developer.apple.com/documentation/usernotifications/unnotificationresponse>

> Available on **crate feature `UNNotificationResponse`** only.

```/dev/null/example.rs#L1-3
pub struct UNNotificationResponse { /* private fields */ }
```

---

## Implementations

### `impl UNNotificationResponse`

| Method                                                        | Notes                                             |
| ------------------------------------------------------------- | ------------------------------------------------- |
| `pub fn notification(&self) -> Retained<UNNotification>`      | Available on crate feature `UNNotification` only. |
| `pub fn actionIdentifier(&self) -> Retained<NSString>`        |                                                   |
| `pub unsafe fn init(this: Allocated<Self>) -> Retained<Self>` |                                                   |

### `impl UNNotificationResponse` — Methods from superclass `NSObject`

| Method                                  | Notes |
| --------------------------------------- | ----- |
| `pub unsafe fn new() -> Retained<Self>` |       |

---

## Methods from `Deref<Target = NSObject>`

### `doesNotRecognizeSelector`

```/dev/null/example.rs#L1
pub fn doesNotRecognizeSelector(&self, sel: Sel) -> !
```

Handle messages the object doesn't recognize. See [Apple's documentation](<https://developer.apple.com/documentation/objectivec/nsobject/doesnotrecognizeselector(_:)>) for details.

---

## Methods from `Deref<Target = AnyObject>`

### `class`

```/dev/null/example.rs#L1
pub fn class(&self) -> &'static AnyClass
```

Dynamically find the class of this object.

**Panics:** May panic if the object is invalid (which may be the case for objects returned from unavailable `init`/`new` methods).

**Example:**

```/dev/null/example.rs#L1-5
use objc2::ClassType;
use objc2::runtime::NSObject;

let obj = NSObject::new();
assert_eq!(obj.class(), NSObject::class());
```

---

### `get_ivar` _(deprecated)_

```/dev/null/example.rs#L1
pub unsafe fn get_ivar<T>(&self, name: &str) -> &T where T: Encode
```

> **Deprecated:** Use `Ivar::load` instead.

**Safety:** The object must have an instance variable with the given name, and it must be of type `T`.

---

### `downcast_ref`

```/dev/null/example.rs#L1
pub fn downcast_ref<T>(&self) -> Option<&T> where T: DowncastTarget
```

Attempt to downcast the object to a class of type `T`. This is the reference-variant. Use `Retained::downcast` if you want to convert a retained object to another type.

**Notes:**

- **Mutable classes:** Some classes have immutable/mutable variants (e.g. `NSString`/`NSMutableString`). Using this to convert between them is generally frowned upon unless you created the object yourself.
- **Generic classes:** Objective-C generics are not exposed in the runtime, so downcasting to generic collections with specific type parameters is disallowed. You can downcast to generic collections where all type parameters are `AnyObject`.
- **Panics:** Works by calling `isKindOfClass:`, which may throw an exception or abort if that method is not available.

**Examples:**

Cast an `NSString` back and forth from `NSObject`:

```/dev/null/example.rs#L1-6
use objc2::rc::Retained;
use objc2_foundation::{NSObject, NSString};

let obj: Retained<NSObject> = NSString::new().into_super();
let string = obj.downcast_ref::<NSString>().unwrap();
let string = obj.downcast::<NSString>().unwrap();
```

Try (and fail) to cast an `NSObject` to `NSString`:

```/dev/null/example.rs#L1-4
use objc2_foundation::{NSObject, NSString};

let obj = NSObject::new();
assert!(obj.downcast_ref::<NSString>().is_none());
```

---

## Trait Implementations

| Trait                                                                  | Notes                                                                                                                                                                   |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `AsRef<AnyObject>`                                                     |                                                                                                                                                                         |
| `AsRef<NSObject>`                                                      |                                                                                                                                                                         |
| `AsRef<UNNotificationResponse>` for `UNNotificationResponse`           |                                                                                                                                                                         |
| `AsRef<UNNotificationResponse>` for `UNTextInputNotificationResponse`  |                                                                                                                                                                         |
| `Borrow<AnyObject>`                                                    |                                                                                                                                                                         |
| `Borrow<NSObject>`                                                     |                                                                                                                                                                         |
| `Borrow<UNNotificationResponse>` for `UNTextInputNotificationResponse` |                                                                                                                                                                         |
| `ClassType`                                                            | `NAME = "UNNotificationResponse"`, `Super = NSObject`                                                                                                                   |
| `CopyingHelper`                                                        | `Result = UNNotificationResponse`                                                                                                                                       |
| `Debug`                                                                |                                                                                                                                                                         |
| `Deref`                                                                | `Target = NSObject`                                                                                                                                                     |
| `DowncastTarget`                                                       |                                                                                                                                                                         |
| `Eq`                                                                   |                                                                                                                                                                         |
| `Hash`                                                                 |                                                                                                                                                                         |
| `Message`                                                              | Provides `retain(&self) -> Retained<Self>`                                                                                                                              |
| `NSCoding`                                                             |                                                                                                                                                                         |
| `NSCopying`                                                            | Provides `copy()` and `copyWithZone()`                                                                                                                                  |
| `NSObjectProtocol`                                                     | Provides `isEqual`, `hash`, `isKindOfClass`, `isMemberOfClass`, `respondsToSelector`, `conformsToProtocol`, `description`, `debugDescription`, `isProxy`, `retainCount` |
| `NSSecureCoding`                                                       | Provides `supportsSecureCoding() -> bool`                                                                                                                               |
| `PartialEq`                                                            |                                                                                                                                                                         |
| `RefEncode`                                                            | `ENCODING_REF` matches `NSObject`                                                                                                                                       |

---

## Auto Trait Implementations

| Trait           | Status             |
| --------------- | ------------------ |
| `Freeze`        | ❌ not implemented |
| `RefUnwindSafe` | ❌ not implemented |
| `Send`          | ❌ not implemented |
| `Sync`          | ❌ not implemented |
| `Unpin`         | ❌ not implemented |
| `UnwindSafe`    | ❌ not implemented |

---

## Blanket Implementations

| Trait             | Provided by                                                                     |
| ----------------- | ------------------------------------------------------------------------------- |
| `Any`             | All `'static + ?Sized` types                                                    |
| `AnyThread`       | Types with `ThreadKind = dyn AnyThread` — provides `alloc() -> Allocated<Self>` |
| `Borrow<T>`       | All `T: ?Sized`                                                                 |
| `BorrowMut<T>`    | All `T: ?Sized`                                                                 |
| `From<T>`         | All `T` (identity)                                                              |
| `Into<U>`         | When `U: From<T>`                                                               |
| `TryFrom<U>`      | When `U: Into<T>`, `Error = Infallible`                                         |
| `TryInto<U>`      | When `U: TryFrom<T>`                                                            |
| `AutoreleaseSafe` | All `T: ?Sized`                                                                 |
