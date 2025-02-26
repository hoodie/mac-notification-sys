use icrate::block2::Block;
use icrate::Foundation::NSObject;
use icrate::UserNotifications::{
    UNNotificationResponse, UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};
use objc2::msg_send_id;
use objc2::rc::Id;
use objc2::runtime::NSObjectProtocol;
use objc2::{declare_class, mutability::InteriorMutable, ClassType};

declare_class! {
    #[derive(Debug)]
    pub(super) struct RustNotificationDelegate {

    }

    unsafe impl ClassType for RustNotificationDelegate {
        type Super = NSObject;
        type Mutability = InteriorMutable;
        const NAME: &'static str = "RustNotificationDelegate";
    }

    unsafe impl UNUserNotificationCenterDelegate for RustNotificationDelegate {
        #[method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:)]
        #[allow(non_snake_case)]
        unsafe fn userNotificationCenter_didReceiveNotificationResponse_withCompletionHandler(
            &self,
            _center: &UNUserNotificationCenter,
            _response: &UNNotificationResponse,
            _completion_handler: &Block<(), ()>,
        ) {
            println!("Received Response");
        }
    }
}

unsafe impl NSObjectProtocol for RustNotificationDelegate {}

impl RustNotificationDelegate {
    pub fn new() -> Id<Self> {
        unsafe { msg_send_id![Self::alloc(), init] }
    }
}
