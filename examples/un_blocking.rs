use std::{thread, time::Duration};

use mac_notification_sys::un::{self, Notification, check_bundle, request_auth_blocking};

fn main() {
    oslog::OsLogger::new("mac-notification-sys")
        .level_filter(log::LevelFilter::Debug)
        .init()
        .unwrap();

    if let Err(error) = check_bundle() {
        eprintln!("Error: {}", error);
        return;
    }

    log::info!("un_simple example: starting");

    match request_auth_blocking() {
        Ok(true) => println!("Notification permission granted."),
        Ok(false) => {
            log::warn!(
                "Notification permission denied. \
                 Allow it in System Settings -> Notifications."
            );
            return;
        }
        Err(error) => {
            log::error!("Authorization error: {error}");
            return;
        }
    }

    let content = Notification::new()
        .title("Hello from un::send")
        .subtitle("blocking wrapper")
        .message("This notification was scheduled using un::send_blocking()")
        .sound("Submarine");

    match un::send_blocking(content) {
        Ok(()) => log::info!("send: scheduled."),
        Err(e) => log::error!("send failed: {e}"),
    }

    thread::sleep(Duration::from_secs(2));

    log::info!("un_async example: exiting");
}
