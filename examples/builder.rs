use mac_notification_sys::Notification;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Notification::new()
        .title("builder pattern")
        .subtitle("built by bob")
        .message("this looks more functional")
        .show()?;
    Ok(())
}
