use mac_notification_sys::un::*;

fn main() {
    if let Err(error) = check_bundle() {
        eprintln!("Error: {}", error);
        return;
    }

    Notification::default()
        .title("Danger")
        .subtitle("Will Robinson")
        .message("Run away as fast as you can")
        .send_blocking()
        .unwrap();

    Notification::default()
        .title("NOW")
        .message("Without subtitle")
        .sound("Submarine")
        .send_blocking()
        .unwrap();
}
