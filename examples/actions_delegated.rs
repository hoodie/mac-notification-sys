#![allow(unused_variables)]
use mac_notification_sys::*;

fn main() {
    let handle = Notification::new()
        .title("Danger")
        .subtitle("Will Robinson")
        .message("Run away as fast as you can")
        .main_button(MainButton::DropdownActions(
            "Dropdown",
            &["Action 1", "Action 2"],
        ))
        .close_button("Nevermind...")
        .send_delegated()
        .unwrap();

    println!("I just sent a notification and I can still do stuff...");

    let response = handle.wait();
    println!("...this only shows once you acknowledge the notification because we blocked the main thread");

    match response {
        // Requires main_button to be a MainButton::SingleAction or MainButton::DropdownActions
        Ok(NotificationResponse::ActionButton(action_name)) => {
            if action_name == "Action 1" {
                println!("Clicked on Action 1")
            } else if action_name == "Action 2" {
                println!("Clicked on Action 2")
            }
        }
        Ok(NotificationResponse::Click) => println!("Clicked on the notification itself"),
        Ok(NotificationResponse::CloseButton(close_name)) => println!(
            "Dismissed the notification with the close button called {}",
            close_name
        ),
        // Requires main_button to be a MainButton::Response
        Ok(NotificationResponse::Reply(response)) => {
            println!("Replied to the notification with {}", response)
        }
        Ok(NotificationResponse::None) => {
            println!("No interaction with the notification occured")
        }
        _ => {}
    };
}
