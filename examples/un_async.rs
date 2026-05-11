//! This example works without having to worry about NSRunLoop because we don't bother about the response delegate
use std::{thread, time::Duration};

use futures_lite::future;
use mac_notification_sys::un::{Notification, check_bundle, request_auth, send};

fn main() {
    future::block_on(async {
        oslog::OsLogger::new("mac-notification-sys")
            .level_filter(log::LevelFilter::Debug)
            .init()
            .unwrap();

        if let Err(error) = check_bundle() {
            eprintln!("Error: {}", error);
            return;
        }

        log::info!("un_async example: starting");

        match request_auth().await {
            Ok(true) => log::info!("Notification permission granted."),
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
            .title("Hello from un::send_async")
            .subtitle("caller-driven future")
            .message("This notification was scheduled using un::send().await")
            .sound("Submarine");

        match send(content).await {
            Ok(()) => log::info!("send_async: scheduled."),
            Err(e) => log::error!("send_async failed: {e}"),
        }
        thread::sleep(Duration::from_secs(2));

        log::info!("un_async example: exiting");
    });
}
