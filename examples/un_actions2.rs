//! Demonstrates text-input (reply) actions alongside regular buttons.
//!
//! Sends two notifications:
//!
//! 1. **Mixed** — Reply (text-input), 👍 Like, and Trash (destructive) on the
//!    same notification, showing that action types can be freely combined.
//! 2. **Reply-only** — a single text-input action, no other buttons.
//!
//! Run with:
//!   cargo bundle --example un_actions2 && \
//!     open target/debug/bundle/osx/mac-notification-sys-example.app

use mac_notification_sys::un::{Action, Notification, check_bundle, request_auth_blocking};

const ACTION_REPLY: &str = "action.reply";
const ACTION_LIKE: &str = "action.like";
const ACTION_TRASH: &str = "action.trash";
const ACTION_QUICK_REPLY: &str = "action.quick_reply";

fn main() {
    oslog::OsLogger::new("mac-notification-sys")
        .level_filter(log::LevelFilter::Debug)
        .init()
        .unwrap();

    if let Err(e) = check_bundle() {
        log::error!("check_bundle failed: {e}");
        log::error!("Run via `cargo bundle --example un_actions2` and open the .app.");
        return;
    }

    match request_auth_blocking() {
        Ok(true) => log::warn!("permission granted"),
        Ok(false) => {
            log::error!("permission denied — allow in System Settings → Notifications");
            return;
        }
        Err(e) => {
            log::error!("auth error: {e}");
            return;
        }
    }

    // ── Notification 1: reply + like + trash ──────────────────────────────────

    log::warn!("sending notification 1 (mixed actions)…");

    match Notification::new()
        .title("New message from Alice")
        .message("Hey, are you coming tonight?")
        .action(Action::reply(
            ACTION_REPLY,
            "Reply",
            "Send",
            "Type a reply…",
        ))
        .action(Action::new(ACTION_LIKE, "👍 Like"))
        .action(Action::new(ACTION_TRASH, "Trash").destructive())
        .timeout(std::time::Duration::from_secs(60))
        .send_blocking()
    {
        Ok(Some(ref r)) if r.is_default_action() => {
            log::warn!("opened");
            notify_back("Opened", "You clicked the notification body.");
        }
        Ok(Some(ref r)) if r.is_dismiss_action() => {
            log::warn!("dismissed");
            notify_back("Dismissed", "You dismissed without choosing an action.");
        }
        Ok(Some(ref r)) if r.action_identifier == ACTION_REPLY => {
            let text = r.reply_text.as_deref().unwrap_or("<empty>");
            log::warn!("replied: {text:?}");
            notify_back("Message sent ✉️", &format!("You replied: \"{text}\""));
        }
        Ok(Some(ref r)) if r.action_identifier == ACTION_LIKE => {
            log::warn!("liked");
            notify_back("Liked 👍", "Alice's message was liked.");
        }
        Ok(Some(ref r)) if r.action_identifier == ACTION_TRASH => {
            log::warn!("trashed");
            notify_back("Trashed 🗑", "Message moved to trash.");
        }
        Ok(Some(ref r)) => log::warn!("unknown action: {}", r.action_identifier),
        Ok(None) => {}
        Err(mac_notification_sys::un::Error::ResponseTimeout) => {
            log::warn!("notification 1 timed out");
        }
        Err(e) => log::error!("notification 1 error: {e}"),
    }

    // ── Notification 2: reply-only ────────────────────────────────────────────

    log::warn!("sending notification 2 (reply-only)…");

    match Notification::new()
        .title("Quick question 🍕")
        .message("What's your favourite pizza topping?")
        .action(Action::reply(
            ACTION_QUICK_REPLY,
            "Answer",
            "Submit",
            "e.g. Mushrooms…",
        ))
        .timeout(std::time::Duration::from_secs(60))
        .send_blocking()
    {
        Ok(Some(ref r)) if r.action_identifier == ACTION_QUICK_REPLY => {
            let text = r.reply_text.as_deref().unwrap_or("<empty>");
            log::warn!("pizza answer: {text:?}");
            notify_back("Great taste! 🍕", &format!("\"{text}\" sounds delicious."));
        }
        Ok(Some(ref r)) if r.is_dismiss_action() => {
            log::warn!("dismissed pizza question");
            notify_back("No answer", "You didn't share your pizza preference.");
        }
        Ok(Some(ref r)) if r.is_default_action() => {
            log::warn!("opened pizza notification");
        }
        Ok(Some(ref r)) => log::warn!("unknown action: {}", r.action_identifier),
        Ok(None) => {}
        Err(mac_notification_sys::un::Error::ResponseTimeout) => {
            log::warn!("notification 2 timed out");
        }
        Err(e) => log::error!("notification 2 error: {e}"),
    }

    log::warn!("done");
}

fn notify_back(title: &str, message: &str) {
    if let Err(e) = Notification::new()
        .title(title)
        .message(message)
        .send_blocking()
    {
        log::error!("notify_back failed: {e}");
    }
}
