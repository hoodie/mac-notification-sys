//! `UNUserNotificationCenterDelegate` — receives user responses to notifications.
//!
//! # Design
//!
//! `NotificationDelegate` is a single Objective-C class instance that lives for
//! the entire lifetime of the process.  It is installed on the
//! `UNUserNotificationCenter` once, during worker startup, via [`install`].
//!
//! When a caller wants to know what the user did with a particular notification
//! it registers a `oneshot::Sender<NotificationResponse>` keyed by the
//! notification's request identifier using [`register_response_sender`].  When
//! macOS fires `didReceiveNotificationResponse`, the delegate looks up that
//! sender, builds a [`NotificationResponse`] and fires it.
//!
//! Senders for notifications that were never interacted with (e.g. the app
//! quit) are simply dropped — the corresponding `oneshot::Receiver` resolves
//! to `Err(Canceled)`, which callers map to a timeout / cancelled state.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use futures_channel::oneshot;
use objc2::rc::Retained;
use objc2::{AnyThread, define_class};
use objc2_foundation::{NSObject, NSObjectProtocol};
use objc2_user_notifications::{
    UNNotificationResponse, UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};

use crate::un::response::NotificationResponse;

// ── Shared sender map ─────────────────────────────────────────────────────────

/// Global map: request-id → oneshot sender waiting for the user's response.
static PENDING: OnceLock<Mutex<HashMap<String, oneshot::Sender<NotificationResponse>>>> =
    OnceLock::new();

fn pending() -> &'static Mutex<HashMap<String, oneshot::Sender<NotificationResponse>>> {
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a sender that will be resolved once the user responds to the
/// notification identified by `request_id`.
pub(super) fn register_response_sender(
    request_id: String,
    tx: oneshot::Sender<NotificationResponse>,
) {
    pending()
        .lock()
        .expect("pending map poisoned")
        .insert(request_id, tx);
}

// ── Objective-C delegate class ────────────────────────────────────────────────

define_class!(
    // SAFETY:
    // - Superclass is NSObject, which has no subclassing invariants.
    // - We do not implement Drop.
    // - The ivars type is `()` (default).
    #[unsafe(super(NSObject))]
    #[name = "MacNotificationSysDelegate"]
    pub struct NotificationDelegate;

    unsafe impl NSObjectProtocol for NotificationDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for NotificationDelegate {
        /// Called when the user responds to a notification (taps an action,
        /// clicks the body, or dismisses).
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive_response(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion_handler: &block2::DynBlock<dyn Fn()>,
        ) {
            let request_id = response.notification().request().identifier().to_string();
            let action_id = response.actionIdentifier().to_string();

            log::debug!(
                "un::delegate: didReceiveNotificationResponse \
                 request_id={request_id:?} action={action_id:?}"
            );

            if let Some(tx) = pending()
                .lock()
                .expect("pending map poisoned")
                .remove(&request_id)
            {
                let resp = NotificationResponse {
                    action_identifier: action_id,
                };
                if tx.send(resp).is_err() {
                    log::warn!(
                        "un::delegate: receiver for request {request_id:?} \
                         was already dropped"
                    );
                }
            } else {
                log::debug!(
                    "un::delegate: no pending sender for request {request_id:?} \
                     (fire-and-forget notification)"
                );
            }

            // macOS requires us to call the completion handler.
            completion_handler.call(());
        }
    }
);

impl NotificationDelegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        unsafe { objc2::msg_send![super(this), init] }
    }
}

// ── Installation ──────────────────────────────────────────────────────────────

/// Install the delegate on `UNUserNotificationCenter`.
///
/// Must be called from the main thread. macOS delivers
/// `didReceiveNotificationResponse` on the main thread's run loop, so the
/// delegate must be installed there for callbacks to fire.
///
/// Called once from [`send_with_actions_blocking`] before pumping the main
/// run loop, and from the async path before awaiting the response.
pub(super) fn install() {
    static DELEGATE: OnceLock<Retained<NotificationDelegate>> = OnceLock::new();
    DELEGATE.get_or_init(|| {
        log::debug!("un::delegate: installing NotificationDelegate");
        let delegate = NotificationDelegate::new();
        let center = UNUserNotificationCenter::currentNotificationCenter();
        // SAFETY: `delegate` satisfies UNUserNotificationCenterDelegate.
        center.setDelegate(Some(objc2::runtime::ProtocolObject::from_ref(&*delegate)));
        delegate
    });
}
