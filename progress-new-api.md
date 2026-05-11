# Project: Async Notification Actions

## Goal

Add non-blocking (`async`-friendly) notification action handling to `notify-rust` on macOS, so that waiting for a user to interact with a notification no longer blocks the calling thread. The macOS-specific implementation requires an app bundle, and **that is an acceptable constraint** — we document how to satisfy it.

### Deliverables

1. **`mac-notification-sys`** gains a new `un` submodule implemented on top of `UNUserNotificationCenter`. The existing `notification` module is left **unchanged**.
2. **`notify-rust`** is the first customer: it will use `un` to expose non-blocking (async-friendly) notification action callbacks on macOS.

## Architecture Decision

We implement the macOS async notification path using `UNUserNotificationCenter`, gated behind a bundle requirement. The public API in `notify-rust` will expose the async actions interface unconditionally; the macOS backend documents that using it requires the binary to be run from within an app bundle.

```
notify-rust (public API)
  └─ Notification::send_async() / send_blocking()
       └─ mac-notification-sys  un::Notification
            └─ UNUserNotificationCenter  ← requires bundle
```

### Design constraints

1. **Non-blocking by design.** The calling thread must never block. A dedicated background thread pumps `NSRunLoop` in short slices — architecturally required and invisible to the caller.
2. **Bundle required on macOS.** If the process has no bundle, `check_bundle()` returns `Err(Error::NoBundleIdentifier)` with a clear message.
3. **No Objective-C swizzling.** The fake-bundle trick does not work with `UNUserNotificationCenter`.
4. **`NSUserNotificationCenter` as legacy fallback.** The existing blocking path is kept for users who do not need action responses or who are on macOS < 10.14.

---

## Implementation Plan

### ✅ Phase 1 — Minimal `UNUserNotificationCenter` demo

**Goal:** Prove the new API works end-to-end. A bundled Rust binary sends one notification using `UNUserNotificationCenter` and exits.

| Task | Status | Notes |
| --- | --- | --- |
| Add `objc2-user-notifications` dependency | ✅ | Powers the `un` module |
| Write `examples/un_simple.rs` | ✅ | Requests permission, posts one notification, sleeps briefly then exits |
| Add `[package.metadata.bundle]` to `Cargo.toml` | ✅ | Identifier `com.github.h4llow3En.mac-notification-sys.example` |
| Document build/run steps | ✅ | In example doc comment and Development Setup section below |

### ✅ Phase 2a — Worker + minimal `send` / `request_auth`

The new API lives in `src/un/`. The existing `notification` module is left **unchanged**.

#### Architecture: one worker thread, two function flavours per operation

Every `un` operation is plumbed through a **single, lazily-spawned worker thread** (`un::worker`) that pumps an `NSRunLoop` continuously in ~50 ms slices. The worker is content-agnostic. Every operation has:

- `*(...)` — canonical `async fn`: dispatches a closure to the worker, awaits a `oneshot` channel.
- `*_blocking(...) -> Result<...>` — convenience: wraps the async fn with `futures_lite::future::block_on`.

| Task | Status | Notes |
| --- | --- | --- |
| `un::worker` module | ✅ | Background thread, `dispatch(FnOnce + Send)`, NSRunLoop pump, per-task `catch_unwind` |
| `send` / `send_blocking` | ✅ | Async fn + blocking wrapper |
| `request_auth` / `request_auth_blocking` | ✅ | Lives in `src/un/auth.rs` |
| `examples/un_simple.rs`, `examples/un_async.rs` | ✅ | Updated to use new API |

### ✅ Phase 2b — Delegate + actions + response handling

| Task | Status | Notes |
| --- | --- | --- |
| `define_class!` delegate (`NotificationDelegate`) | ✅ | Implements `didReceiveNotificationResponse:withCompletionHandler:` |
| Delegate installed at worker thread startup | ✅ | Single instance; install thread does not affect callback delivery thread |
| Global `PENDING` map: request-id → `oneshot::Sender<NotificationResponse>` | ✅ | `Mutex<HashMap<String, Sender>>` behind `OnceLock` |
| `PendingGuard` RAII type | ✅ | Deregisters sender from `PENDING` on drop (timeout / cancel path) |
| `deregister_response_sender` | ✅ | Called by `PendingGuard::drop`; prevents map leaks on timeout |
| `Action` builder (`Action::new`, `Action::destructive`, `Action::requires_authentication`) | ✅ | Public; builds `UNNotificationAction` |
| `Action::reply` + `ReplyConfig` | ✅ | Builds `UNTextInputNotificationAction`; `.into_super()` for safe upcast |
| `ActionCategory` (crate-internal) | ✅ | Synthesised from sorted action IDs; registered on the worker before scheduling |
| `CustomDismissAction` always set on category | ✅ | Makes dismiss interactions observable via delegate |
| `Notification::action(Action)` builder setter | ✅ | Appends to `Vec<Action>`; no manual category registration needed |
| `Notification::timeout(Duration)` builder setter | ✅ | Stored as `action_timeout: Option<Duration>`; read by send functions |
| `send_with_actions` (async) | ✅ | Races response future against `sleep_future`; `PendingGuard` cleans up on timeout |
| `send_with_actions_blocking` | ✅ | `block_on(send_with_actions(...))` — no main-thread involvement |
| `Notification::send_async` / `send_blocking` | ✅ | Unified entry point; branches on `actions.is_empty()` automatically |
| `NotificationResponse::action_identifier` | ✅ | Identifier of the tapped action |
| `NotificationResponse::reply_text: Option<String>` | ✅ | Set when response is `UNTextInputNotificationResponse`; downcast via `AnyObject::downcast_ref` |
| `NotificationResponse::is_default_action` / `is_dismiss_action` / `is_reply` | ✅ | Convenience predicates |
| `Error::ResponseTimeout` | ✅ | Returned when deadline passes before user interaction |

### ✅ Threading model — main run loop requirement documented and solved

This was the most important architectural discovery of the session.

**The constraint:** `UNUserNotificationCenter` always delivers `didReceiveNotificationResponse` on the **main thread's run loop**, regardless of which thread the delegate was installed from. This is documented Apple behaviour.

**Consequence:** any process where the main thread does not pump `NSRunLoop` (e.g. `#[tokio::main]`, bare `future::block_on`) will never receive callbacks.

**Not a problem for GUI apps:** Tauri, winit, AppKit, SwiftUI — all pump `NSRunLoop` on the main thread automatically via `NSApplicationMain` / `NSApp.run()`. `send_async` works out of the box.

**For CLI tools:** two helpers are provided:

| Helper | Use case |
| --- | --- |
| `un::block_on_main(future)` | Drop-in replacement for `futures_lite::future::block_on`; polls the future while pumping `NSRunLoop::mainRunLoop()`. Works with any `Future` including `tokio::task::JoinHandle`. |
| `un::run_main_loop_while(fn)` | For advanced cases (Tokio runtime on background threads, custom shutdown signalling). |

The module-level doc comment in `src/un.rs` contains a complete Tokio example pattern.

### ✅ Examples

| Example | Description |
| --- | --- |
| `un_simple.rs` | Fire-and-forget notification, blocking |
| `un_async.rs` | Fire-and-forget, `futures_lite` async |
| `un_actions.rs` | Action buttons, blocking (`send_blocking`) — main thread pumped manually |
| `un_actions_async.rs` | Action buttons, `futures_lite` async via `block_on_main` |
| `un_actions_tokio.rs` | Action buttons, Tokio multi-thread runtime via `block_on_main(rt.spawn(...))` |
| `un_actions2.rs` | Text-input (reply) actions mixed with regular buttons; two notifications |

### Phase 3 — `notify-rust`: non-blocking public API (not started)

| Task | Status | Notes |
| --- | --- | --- |
| Wire `un` module into `notify-rust` macOS backend | ❌ | |
| Expose async action response API | ❌ | `send_async` / `send_blocking` returning `Option<NotificationResponse>` |
| Feature flag or cfg gate | ❌ | `#[cfg(target_os = "macos")]` |
| Document bundle requirement in `notify-rust` README | ❌ | Tauri / GUI apps: nothing to do. CLI: use `block_on_main`. |

---

## Development Setup: Running as an App Bundle

To use `UNUserNotificationCenter` during development, the binary must be wrapped in a `.app` bundle.

### 1. Install `cargo-bundle`

```
cargo install cargo-bundle
```

### 2. Add bundle metadata to `Cargo.toml`

```toml
[package.metadata.bundle]
name              = "mac-notification-sys-example"
identifier        = "com.github.h4llow3En.mac-notification-sys.example"
version           = "0.1.0"
short_description = "mac-notification-sys example app"
```

### 3. Build and run

```sh
cargo bundle --example un_actions
open target/debug/bundle/osx/mac-notification-sys-example.app
```

> **First launch:** macOS will prompt to allow notifications. Accept in System Settings → Notifications.

### 4. For examples and integration tests

```sh
cargo bundle --example un_actions2
open target/debug/bundle/osx/mac-notification-sys-example.app
```

---

## Key Facts Summary

| Fact | Detail |
| --- | --- |
| `UNUserNotificationCenter` requires bundle | Hard requirement — `check_bundle()` returns `Err` without one |
| Delegate callbacks always on main thread | Apple-documented; install thread is irrelevant |
| GUI apps (Tauri, winit, AppKit) | Main run loop pumped automatically — `send_async` just works |
| CLI tools with async runtime | Use `block_on_main` or `run_main_loop_while` |
| Timeout on "Clear All" | `Notification::timeout(Duration)` — `PendingGuard` cleans up PENDING map |
| Reply actions | `Action::reply(id, title, button, placeholder)` — `UNTextInputNotificationAction` |
| Mixed actions | Regular buttons and reply input can coexist in one notification |
| `block_on_main` works with Tokio | Polls `JoinHandle` via noop waker; response arrives within one 50 ms tick |

---

## Architectural Notes (Hard-Won)

### Callback delivery thread

`UNUserNotificationCenter` delivers `didReceiveNotificationResponse` on the **main thread's run loop**. Moving `delegate::install()` to the worker thread (tried) does not change this. The worker thread's `NSRunLoop` is irrelevant for callback delivery.

### `DynBlock` vs `Block` in delegate method signature

The delegate override for `withCompletionHandler:` must use `&block2::DynBlock<dyn Fn()>`, not `&block2::Block<dyn Fn()>`. Using the wrong type causes macOS to silently skip the override entirely — no compile error, no runtime error, just no callbacks.

### `block_on_main` noop waker

`block_on_main` uses `Waker::noop()` and polls the future every ~50 ms alongside `runMode_beforeDate`. This is sufficient for interactive notification responses (human timescales >> 50 ms). It works for `tokio::task::JoinHandle` because the handle's `poll` checks an atomic flag that Tokio sets when the task completes.

### `oslog` log levels

`log::info!` maps to `OS_LOG_TYPE_INFO` which is suppressed in Console.app by default. Use `log::warn!` and above for anything you want to see without changing Console configuration.

---

## What Was Previously Explored

### Option A: `UNUserNotificationCenter` — first attempt

Crashed with `NSInternalInconsistencyException: bundleProxyForCurrentProcess` from a CLI binary. Incorrectly concluded incompatible with CLI use. **Correct conclusion:** requires a bundle.

### Option B: `NSUserNotificationCenter` + GCD

GCD threads have no persistent run loop. Delegate callbacks never fired. **Dead end.**

### Option C: `NSUserNotificationCenter` + `NSThread`

Worked for a single notification; race condition when concurrent sends overwrote the singleton delegate. **Partially correct.**

### Option D: `NSUserNotificationCenter` + `NSThread` + serialization mutex

`NOTIFICATION_SEND_LOCK` serializes sends. Works but: blocking, no sound on macOS 14+, deprecated API, does not compose with async. **Kept as legacy fallback.**

---

## Current State (as of latest commit)

### Source files

| File | Purpose |
| --- | --- |
| `src/un/worker.rs` | Worker thread, `dispatch()`, NSRunLoop pump, `catch_unwind` per task; installs delegate at startup |
| `src/un/auth.rs` | `request_auth` (async) + `request_auth_blocking` |
| `src/un/action.rs` | `Action` builder (`new`, `reply`, `destructive`, `requires_authentication`); `ReplyConfig`; `ActionCategory` (crate-internal) |
| `src/un/delegate.rs` | `NotificationDelegate` (`define_class!`); `PENDING` map; `register_response_sender`; `deregister_response_sender`; `install()` |
| `src/un/response.rs` | `NotificationResponse` (`action_identifier`, `reply_text`, `is_default_action`, `is_dismiss_action`, `is_reply`) |
| `src/un/send.rs` | `schedule_inner`; `PendingGuard`; `sleep_future`; `send`; `send_blocking`; `send_with_actions`; `send_with_actions_blocking` |
| `src/un.rs` | `Notification` builder (`title`, `message`, `subtitle`, `sound`, `action`, `timeout`); `check_bundle`; `block_on_main`; `run_main_loop_while`; `Error` |

### Next step

Phase 3: wire the `un` module into `notify-rust` as the macOS backend for async notification responses.
