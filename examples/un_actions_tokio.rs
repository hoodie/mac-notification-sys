//! Async actions example, with Tokio.
//!
//! Demonstrates running real Tokio tasks (multi-threaded runtime) while keeping
//! the main thread free to pump `NSRunLoop` for `UNUserNotificationCenter`
//! callback delivery.
//!
//! Pattern: build the runtime manually (do NOT use `#[tokio::main]`), spawn the
//! async work onto it, then `block_on_main` the resulting `JoinHandle`.  The
//! main thread polls the handle while pumping `NSRunLoop`; Tokio runs entirely
//! on background threads.
//!
//! Run with:
//!   cargo bundle --example un_actions_tokio && \
//!     open target/debug/bundle/osx/mac-notification-sys-example.app

use mac_notification_sys::un::{
    Action, Error, Notification, block_on_main, check_bundle, request_auth,
};

const ACTION_REPLY: &str = "action.reply";
const ACTION_ARCHIVE: &str = "action.archive";

fn main() {
    oslog::OsLogger::new("mac-notification-sys")
        .level_filter(log::LevelFilter::Debug)
        .init()
        .unwrap();

    if let Err(e) = check_bundle() {
        log::error!("check_bundle failed: {e}");
        return;
    }

    // Multi-thread runtime lives entirely on background threads.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build Tokio runtime");

    // Spawn the async work onto Tokio, then await its JoinHandle on the main
    // thread while NSRunLoop is being pumped.
    let handle = rt.spawn(run());
    if let Err(e) = block_on_main(handle) {
        log::error!("tokio task panicked: {e}");
    }
}

async fn run() {
    match request_auth().await {
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

    let content = Notification::new()
        .title("New message")
        .message("Tap an action or click to open.")
        .action(Action::new(ACTION_REPLY, "Reply"))
        .action(Action::new(ACTION_ARCHIVE, "Archive").destructive())
        .timeout(std::time::Duration::from_secs(60));

    log::warn!("sending notification, waiting for user response…");

    match content.send_async().await {
        Ok(Some(response)) if response.is_default_action() => {
            log::warn!("user clicked the notification body");
            notify_back("Opened", "You clicked the notification body.").await;
        }
        Ok(Some(response)) if response.is_dismiss_action() => {
            log::warn!("user dismissed the notification");
            notify_back("Dismissed", "You dismissed the notification.").await;
        }
        Ok(Some(response)) if response.action_identifier == ACTION_REPLY => {
            log::warn!("user chose: reply");
            notify_back("Reply", "You chose: Reply").await;
        }
        Ok(Some(response)) if response.action_identifier == ACTION_ARCHIVE => {
            log::warn!("user chose: archive");
            notify_back("Archive", "You chose: Archive").await;
        }
        Ok(Some(response)) => {
            log::warn!("unknown action: {}", response.action_identifier);
            notify_back("Unknown Action", "You chose an unknown action.").await;
        }
        Ok(None) => {
            log::warn!("notification sent (no actions)");
        }
        Err(Error::ResponseTimeout) => {
            log::warn!("timed out waiting for user response");
        }
        Err(e) => {
            log::error!("send_async failed: {e}");
            notify_back("Error", "An error occurred while processing your request.").await;
        }
    }

    log::warn!("done");
}

async fn notify_back(title: &str, message: &str) {
    Notification::new()
        .title(title)
        .message(message)
        .send_async()
        .await
        .unwrap();
}
