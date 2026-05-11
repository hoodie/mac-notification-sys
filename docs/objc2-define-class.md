# `define_class!` — objc2 0.6.4

> Source: <https://docs.rs/objc2/latest/objc2/macro.define_class.html>

## Macro Signature

```mac-notification-sys2/docs/objc2-define-class.md#L1-1
macro_rules! define_class {
    {
        // Special attributes supported:
        // - #[unsafe(super($($superclasses:path),*))]
        // - #[unsafe(super = $superclass:path)]
        // - #[thread_kind = $thread_kind:path]
        // - #[name = $name:literal]
        // - #[ivars = $ivars:path]
        $(#[$($attrs:tt)*])*
        $v:vis struct $class:ident;

        // unsafe impl Protocol for $class { ... }
        // impl $class { ... }
        $($impls:tt)*
    } => { ... };
}
```

## Overview

Create a new Objective-C class.

Useful for implementing "delegate" design patterns common in Objective-C frameworks, where you subclass or implement a protocol to hook into functionality.

This macro is the **declarative** way of creating classes, in contrast with `ClassBuilder`, which is imperative. Prefer this macro — it contains extra debug assertions and soundness checks.

The class is guaranteed to be created and registered with the Objective-C runtime after `ClassType::class()` has been called.

---

## Specification

The macro consists of:

1. **The type definition** with special attributes
2. **Inherent `impl` blocks** (zero or more)
3. **Protocol `impl` blocks** (zero or more)

The syntax resembles a mix of `extern_class!` and `extern_methods!`. It creates an opaque struct, implements `DefinedClass`, and implements any specified protocols.

If the type implements `Drop`, the macro auto-generates a `dealloc` method that calls `drop`.

> **Note:** Generic types are not supported.

---

## Attributes

Most normal Rust attributes work (`#[cfg(...)]`, `#[allow(...)]`, doc comments). Exceptions are noted below.

### `#[unsafe(super(...))]` _(required)_

Same semantics as in `extern_class!`. Specifies the superclass(es).

### `#[thread_kind = ...]` _(optional)_

Same as in `extern_class!`. Use e.g. `MainThreadOnly` for UI delegates.

### `#[name = "..."]` _(optional)_

Sets the Objective-C runtime name for the class. Must be globally unique in the process.

If omitted, the name defaults to:

```mac-notification-sys2/docs/objc2-define-class.md#L1-1
concat!(module_path!(), "::", $class, env!("CARGO_PKG_VERSION"))
// e.g. "my_crate::my_module::MyClass0.1.0"
```

**Library authors:** leave this unset — the default name handles multiple SemVer-incompatible versions in the same binary correctly and works across shared dynamic libraries.

### `#[ivars = ...]` _(optional)_

Specifies the instance variable type for the class. Defaults to `()` if omitted.

Use interior mutability (`Cell`, `RefCell`, atomics, etc.) to allow mutation of ivars.

> If you want to use inherited initializers (e.g. `init`), you must override the subclass' designated initializers and initialize your ivars there.

### `#[derive(...)]`

Overridden — only `PartialEq`, `Eq`, `Hash`, and `Debug` are supported. Implementations delegate to the superclass. To customize behavior, override `isEqual:` and `hash` instead.

---

## Inherent Method Definitions

Inside `impl` blocks you define two kinds of functions:

| Rust                               | Objective-C equivalent |
| ---------------------------------- | ---------------------- |
| Associated function (no `self`)    | Class method           |
| Method (`self` / `this` / `_this`) | Instance method        |

Supported receiver types: `&self`, `self: *const Self`, `this: *const Self`, etc.  
`&mut self` is **not** supported — use `Cell` or similar for mutation.

### Selector attributes

- `#[unsafe(method(my:selector:))]` — regular return type
- `#[unsafe(method_id(my:selector:))]` — return type must be `Option<Retained<T>>` or `Retained<T>`; for `init`-family selectors, `self`/`this` must be `Allocated<Self>`

`bool` parameters/return values are automatically converted to Objective-C `BOOL`. Use `runtime::Bool` to control this manually.

Methods undergo an ABI transformation and **should not be called directly from Rust**. Use `extern_methods!` to expose a Rust interface.

---

## Protocol Implementations

Specify protocols with `unsafe impl Protocol for MyClass { ... }`. The protocol must be defined with `extern_protocol!`. Methods follow the same rules as inherent methods.

---

## Panics

`ClassType::class()` may panic if:

- A class with the same name already exists.
- Debug assertions are on and an overridden method's signature doesn't match the superclass.
- Debug assertions are on and required protocol methods are not implemented.

---

## Safety

### `#[unsafe(super(...))]`

You must ensure:

- Any invariants of the superclass are upheld.
- If `Drop` is implemented:
  - Must not call any overridden methods.
  - Must not `retain` the object past the drop lifetime.
  - Must not `retain` while `&mut self` is active.

### `#[unsafe(method(...))]` / `#[unsafe(method_id(...))]`

Types must match what Objective-C expects when the method is invoked. Unlike `extern_methods!`, there are **no safe-guards** here.

### `unsafe impl Protocol for Type { ... }`

All required protocol methods must be implemented, and any implicit/explicit protocol requirements must be upheld.

---

## Thread Safety

Classes created with `define_class!` are automatically `Send + Sync` (via auto traits) if:

- The superclass is thread-safe (or is `NSObject`)
- The ivars are thread-safe
- The thread kind is not `MainThreadOnly`

---

## Example

```mac-notification-sys2/docs/objc2-define-class.md#L1-1
use std::ffi::c_int;

use objc2_foundation::{CopyingHelper, NSCopying, NSObject, NSObjectProtocol, NSZone};
use objc2::rc::{Allocated, Retained};
use objc2::{
    define_class, extern_methods, extern_protocol, msg_send, AnyThread,
    ClassType, DefinedClass, ProtocolType,
};

#[derive(Clone)]
struct Ivars {
    foo: u8,
    bar: c_int,
    object: Retained<NSObject>,
}

define_class!(
    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - `MyCustomObject` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[ivars = Ivars]
    struct MyCustomObject;

    impl MyCustomObject {
        #[unsafe(method(foo))]
        fn __get_foo(&self) -> u8 {
            self.ivars().foo
        }

        #[unsafe(method_id(object))]
        fn __get_object(&self) -> Retained<NSObject> {
            self.ivars().object.clone()
        }

        #[unsafe(method(myClassMethod))]
        fn __my_class_method() -> bool {
            true
        }
    }

    unsafe impl NSObjectProtocol for MyCustomObject {}

    unsafe impl NSCopying for MyCustomObject {
        #[unsafe(method_id(copyWithZone:))]
        fn copyWithZone(&self, _zone: *const NSZone) -> Retained<Self> {
            let new = Self::alloc().set_ivars(self.ivars().clone());
            unsafe { msg_send![super(new), init] }
        }
    }
);

unsafe impl CopyingHelper for MyCustomObject {
    type Result = Self;
}

impl MyCustomObject {
    fn new(foo: u8) -> Retained<Self> {
        let this = Self::alloc().set_ivars(Ivars {
            foo,
            bar: 42,
            object: NSObject::new(),
        });
        unsafe { msg_send![super(this), init] }
    }
}

impl MyCustomObject {
    extern_methods!(
        #[unsafe(method(foo))]
        pub fn get_foo(&self) -> u8;

        #[unsafe(method(object))]
        pub fn get_object(&self) -> Retained<NSObject>;

        #[unsafe(method(myClassMethod))]
        pub fn my_class_method() -> bool;
    );
}

fn main() {
    let obj = MyCustomObject::new(3);
    assert_eq!(obj.ivars().foo, 3);
    assert_eq!(obj.ivars().bar, 42);

    let obj = obj.copy();
    assert_eq!(obj.get_foo(), 3);
    assert!(MyCustomObject::my_class_method());
}
```

### Equivalent Objective-C (ARC)

```mac-notification-sys2/docs/objc2-define-class.md#L1-1
@interface MyCustomObject: NSObject <NSCopying>
- (instancetype)initWithFoo:(uint8_t)foo;
- (uint8_t)foo;
- (NSObject*)object;
+ (BOOL)myClassMethod;
@end

@implementation MyCustomObject {
    uint8_t foo;
    int bar;
    NSObject* _Nonnull object;
}

- (instancetype)initWithFoo:(uint8_t)foo_arg {
    self = [super init];
    if (self) {
        self->foo = foo_arg;
        self->bar = 42;
        self->object = [NSObject new];
    }
    return self;
}

- (uint8_t)foo { return self->foo; }
- (NSObject*)object { return self->object; }
+ (BOOL)myClassMethod { return YES; }

- (id)copyWithZone:(NSZone *)_zone {
    MyCustomObject* new = [[MyCustomObject alloc] initWithFoo:self->foo];
    new->bar = self->bar;
    new->object = self->object;
    return new;
}

@end
```
