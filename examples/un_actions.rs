use mac_notification_sys::un::{
    Action, Notification, check_bundle, request_auth_blocking, send_with_actions_blocking,
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

    let content = Notification::new()
        .title("New message")
        .message("Tap an action or click to open.")
        .action(Action::new(ACTION_REPLY, "Reply"))
        .action(Action::new(ACTION_ARCHIVE, "Archive").destructive());

    log::warn!("sending notification, waiting for user response…");

    match send_with_actions_blocking(content) {
        Ok(response) if response.is_default_action() => {
            log::warn!("user clicked the notification body");
            notify_back("Opened", "You clicked the notification body.");
        }
        Ok(response) if response.is_dismiss_action() => {
            log::warn!("user dismissed the notification");
            notify_back("Dismissed", "You dismissed the notification.");
        }
        Ok(response) if response.action_identifier == ACTION_REPLY => {
            log::warn!("user chose: reply");
            notify_back("Reply", "You chose: Reply");
        }
        Ok(response) if response.action_identifier == ACTION_ARCHIVE => {
            log::warn!("user chose: archive");
            notify_back("Archive", "You chose: Archive");
        }
        Ok(response) => {
            log::warn!("unknown action: {}", response.action_identifier);
            notify_back("Unknown Action", "You chose an unknown action.");
        }
        Err(e) => {
            log::error!("send_with_actions_blocking failed: {e}");
            notify_back("Error", "An error occurred while processing your request.");
        }
    }

    log::warn!("done");
}

fn notify_back(title: &str, message: &str) {
    Notification::new()
        .title(title)
        .message(message)
        .send_blocking()
        .unwrap();
}
