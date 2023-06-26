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
    // let (response_thread, rx) = block_for_notification_in_background(handle);
    let response = block_for_notification(handle);
    println!("...this only shows once you acknowledge the notification because we blocked the main thread");

    // for _ in 0..10 {
    //     std::thread::sleep(std::time::Duration::from_secs(1));

    //     match rx.try_recv() {
    //         Ok(response) => {
    //             println!("{:?}", response);
    //             break;
    //         }
    //         Err(std::sync::mpsc::TryRecvError::Disconnected) => {
    //             println!("disconnected")
    //         }
    //         Err(std::sync::mpsc::TryRecvError::Empty) => {
    //             println!("empty")
    //         }
    //     }
    // }

    // let response = rx.recv().unwrap();
    // let response = response_thread.join().unwrap();

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
        Ok(NotificationResponse::None) => println!("No interaction with the notification occured"),
        _ => {}
    };

    // let _ = dbg!(response_thread.join());
}
