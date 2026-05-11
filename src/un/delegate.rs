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
use objc2::runtime::AnyObject;
use objc2::{AnyThread, define_class};
use objc2_foundation::{NSObject, NSObjectProtocol};
use objc2_user_notifications::{
    UNNotificationResponse, UNTextInputNotificationResponse, UNUserNotificationCenter,
    UNUserNotificationCenterDelegate,
};

use crate::un::response::NotificationResponse;

// ── Shared sender map ─────────────────────────────────────────────────────────

/// Global map: request-id → oneshot sender waiting for the user's response.
///
/// Entries are removed when `didReceiveNotificationResponse` fires — which
/// covers all explicit interactions including dismiss (via `CustomDismissAction`).
/// The one case that does NOT fire the delegate is the user clicking
/// "Clear All" in the notification center, which leaves the sender in this
/// map until the process exits. For typical use (a small number of concurrent
/// actionable notifications) this is acceptable.
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

/// Remove and drop any pending sender for `request_id`.
///
/// Called when a caller gives up waiting (timeout / future dropped) so the map
/// does not grow without bound. Dropping the sender causes the corresponding
/// `oneshot::Receiver` to resolve to `Err(Canceled)`, which callers should
/// never observe because they already decided to stop waiting.
pub(super) fn deregister_response_sender(request_id: &str) {
    let removed = pending()
        .lock()
        .expect("pending map poisoned")
        .remove(request_id);
    if removed.is_some() {
        log::debug!("un::delegate: deregistered pending sender for {request_id:?}");
    }
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

            // Try to downcast to UNTextInputNotificationResponse to capture
            // any text the user typed in a reply action.
            // downcast_ref calls isKindOfClass: internally and is safe.
            let reply_text: Option<String> = {
                let any: &AnyObject = response.as_ref();
                any.downcast_ref::<UNTextInputNotificationResponse>()
                    .map(|tr| tr.userText().to_string())
            };
            if let Some(ref text) = reply_text {
                log::debug!("un::delegate: reply text = {text:?}");
            }

            if let Some(tx) = pending()
                .lock()
                .expect("pending map poisoned")
                .remove(&request_id)
            {
                let resp = NotificationResponse {
                    action_identifier: action_id,
                    reply_text,
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
/// **`UNUserNotificationCenter` always delivers `didReceiveNotificationResponse`
/// on the main thread's run loop**, regardless of which thread this function is
/// called from.  The main thread must therefore pump `NSRunLoop` while a
/// response is awaited — see [`run_main_loop_while`] in the parent module.
///
/// We call this from the worker thread at startup so the delegate is ready
/// before any notification is scheduled.  The install thread does not affect
/// callback delivery.
///
/// Safe to call multiple times — the `OnceLock` ensures a single install.
///
/// [`run_main_loop_while`]: crate::un::run_main_loop_while
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
