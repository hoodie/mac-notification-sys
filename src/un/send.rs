use super::{Error, Notification, check_bundle};
use crate::un::{action::ActionCategory, delegate, response::NotificationResponse, worker};
use block2::RcBlock;
use futures_channel::oneshot;
use objc2_foundation::{NSError, NSString, NSUUID};
use objc2_user_notifications::{UNNotificationRequest, UNUserNotificationCenter};
use std::{cell::Cell, future::Future, time::Duration};

// ── Pending-response guard ────────────────────────────────────────────────────

/// Couples a `request_id` to a `oneshot::Receiver<NotificationResponse>`.
///
/// When this guard is dropped the corresponding sender is removed from the
/// global `PENDING` map so the map never grows without bound.  The
/// `into_receiver` method consumes the guard *without* triggering
/// deregistration — used by callers that successfully read from the channel.
struct PendingGuard {
    request_id: String,
    rx: Option<oneshot::Receiver<NotificationResponse>>,
}

impl PendingGuard {
    fn new(request_id: String, rx: oneshot::Receiver<NotificationResponse>) -> Self {
        Self {
            request_id,
            rx: Some(rx),
        }
    }

    /// Consume the guard and return the receiver without deregistering.
    ///
    /// The caller is responsible for driving the receiver to completion (or
    /// dropping it, which is fine — the sender side is already gone by then).
    fn into_receiver(mut self) -> oneshot::Receiver<NotificationResponse> {
        self.rx.take().expect("receiver already consumed")
    }
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        // rx is Some when we are dropping without having consumed it — meaning
        // we timed out or the future was cancelled.  Clean up the sender so
        // the PENDING map doesn't leak.
        if self.rx.is_some() {
            delegate::deregister_response_sender(&self.request_id);
        }
    }
}

// ── schedule_inner ────────────────────────────────────────────────────────────

/// Core scheduling logic.
///
/// Dispatches onto the worker thread, registers the optional response sender
/// *before* the request is added to the center (ensuring no race), and returns
/// a `Future` that resolves once macOS has accepted or rejected the request.
///
/// On success the future resolves to `Ok(Some(guard))` when `response_tx` was
/// provided, or `Ok(None)` for fire-and-forget sends.
fn schedule_inner(
    content: Notification,
    response_tx: Option<oneshot::Sender<NotificationResponse>>,
    request_id: String,
) -> impl Future<Output = Result<(), Error>> + Send + 'static {
    let (scheduled_tx, scheduled_rx) = oneshot::channel::<Result<(), Error>>();

    worker::dispatch(move || {
        let (un_content, actions, _timeout) = content.into_parts();
        log::debug!("un::schedule: request_id={request_id:?}");

        if !actions.is_empty() {
            let category_id = {
                let mut ids: Vec<&str> = actions.iter().map(|a| a.identifier.as_str()).collect();
                ids.sort_unstable();
                ids.join(",")
            };
            log::debug!("un::schedule: registering synthesised category {category_id:?}");
            ActionCategory::from_actions(&category_id, actions).register_now();
            un_content.setCategoryIdentifier(&NSString::from_str(&category_id));
        }

        if let Some(tx) = response_tx {
            delegate::register_response_sender(request_id.clone(), tx);
        }

        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&request_id),
            &un_content,
            None,
        );

        let scheduled_tx = Cell::new(Some(scheduled_tx));
        let block = RcBlock::new(move |err: *mut NSError| {
            log::debug!(
                "un::schedule: completion handler fired (err.is_null={})",
                err.is_null()
            );
            if let Some(tx) = scheduled_tx.take() {
                let result = if err.is_null() {
                    Ok(())
                } else {
                    Err(Error::NotificationRejected)
                };
                if tx.send(result).is_err() {
                    log::warn!("un::schedule: scheduled_rx was already dropped");
                }
            }
        });

        UNUserNotificationCenter::currentNotificationCenter()
            .addNotificationRequest_withCompletionHandler(&request, Some(&block));
    });

    async move {
        scheduled_rx
            .await
            .unwrap_or(Err(Error::NotificationRejected))
    }
}

// ── Timer helper ──────────────────────────────────────────────────────────────

/// Returns a `Future` that resolves after `duration` by sleeping a background
/// thread.  Uses no external timer crate — just a `oneshot` channel and
/// `std::thread::spawn`.
fn sleep_future(duration: Duration) -> impl Future<Output = ()> + Send + 'static {
    let (tx, rx) = oneshot::channel::<()>();
    std::thread::Builder::new()
        .name("un-timeout".into())
        .spawn(move || {
            std::thread::sleep(duration);
            let _ = tx.send(());
        })
        .expect("failed to spawn timeout thread");
    async move {
        let _ = rx.await;
    }
}

// ── Public send functions ─────────────────────────────────────────────────────

/// Schedule a notification for immediate delivery via `UNUserNotificationCenter`.
///
/// Returns a [`Future`] that resolves once macOS accepts or rejects the request.
pub async fn send(content: Notification) -> Result<(), Error> {
    check_bundle()?;
    let request_id = NSUUID::new().UUIDString().to_string();
    schedule_inner(content, None, request_id).await
}

/// Schedule a notification for immediate delivery, blocking the calling thread.
pub fn send_blocking(content: Notification) -> Result<(), Error> {
    check_bundle()?;
    let request_id = NSUUID::new().UUIDString().to_string();
    futures_lite::future::block_on(schedule_inner(content, None, request_id))
}

// ── send_with_actions ─────────────────────────────────────────────────────────

/// Schedule a notification with action buttons and wait for the user's response.
///
/// The delegate is installed on the worker thread (which continuously pumps
/// `NSRunLoop`), so callbacks are delivered there and wake the returned future
/// regardless of which thread or executor the caller uses.  This means the
/// async path works correctly from Tokio tasks, `async-std`, bare `block_on`,
/// or any other executor — no main-thread involvement required.
///
/// `timeout` is taken from [`Notification::timeout`].  Pass `None` to wait
/// indefinitely (not recommended — "Clear All" in Notification Center will
/// cause the future to never resolve).
///
/// Returns `Err(`[`Error::ResponseTimeout`]`)` if the deadline passes before
/// the user interacts with the notification.
pub async fn send_with_actions(content: Notification) -> Result<NotificationResponse, Error> {
    check_bundle()?;
    // Delegate is already installed on the worker thread at worker startup.
    // No main-thread interaction needed.

    let request_id = NSUUID::new().UUIDString().to_string();
    let (response_tx, response_rx) = oneshot::channel();
    let timeout = content.action_timeout;
    let guard = PendingGuard::new(request_id.clone(), response_rx);

    schedule_inner(content, Some(response_tx), request_id).await?;

    match timeout {
        None => guard
            .into_receiver()
            .await
            .map_err(|_| Error::NotificationRejected),
        Some(duration) => {
            futures_lite::future::or(
                async {
                    guard
                        .into_receiver()
                        .await
                        .map_err(|_| Error::NotificationRejected)
                },
                async move {
                    sleep_future(duration).await;
                    Err(Error::ResponseTimeout)
                },
            )
            .await
        }
    }
}

/// Schedule a notification with action buttons, blocking until the user
/// responds or the timeout elapses.
///
/// Blocks the calling thread using `futures_lite::future::block_on`.  The
/// delegate fires on the worker thread so the main thread is never touched;
/// this function is safe to call from any thread including Tokio's
/// `spawn_blocking` pool.
///
/// Returns `Err(`[`Error::ResponseTimeout`]`)` if the deadline passes before
/// the user interacts with the notification.
pub fn send_with_actions_blocking(content: Notification) -> Result<NotificationResponse, Error> {
    check_bundle()?;
    futures_lite::future::block_on(send_with_actions(content))
}
