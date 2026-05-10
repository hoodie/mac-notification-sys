use super::{Error, Notification, check_bundle};
use crate::un::{action::ActionCategory, delegate, response::NotificationResponse, worker};
use block2::RcBlock;
use futures_channel::oneshot;
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSError, NSRunLoop, NSString, NSUUID};
use objc2_user_notifications::{UNNotificationRequest, UNUserNotificationCenter};
use std::{cell::Cell, future::Future};

/// Schedule a notification on the worker thread.
///
/// Resolves once macOS accepts or rejects the request. If `response_tx` is
/// `Some`, it is registered with the delegate before scheduling so the caller
/// can also wait for the user's interaction.
fn schedule_inner(
    content: Notification,
    response_tx: Option<oneshot::Sender<NotificationResponse>>,
) -> impl Future<Output = Result<(), Error>> + Send + 'static {
    let (scheduled_tx, scheduled_rx) = oneshot::channel::<Result<(), Error>>();

    worker::dispatch(move || {
        let (un_content, actions) = content.into_parts();
        let request_id = NSUUID::new().UUIDString().to_string();
        log::debug!("un::schedule: request_id={request_id:?}");

        // If the notification carries action buttons, synthesise a category
        // identifier from the sorted action IDs and register it on the center
        // before scheduling. Categories are process-local and not persisted
        // between launches, so we always re-register.
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
            // Register before scheduling — the delegate must never fire before
            // the sender is in the map.
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

/// Schedule a notification for immediate delivery via `UNUserNotificationCenter`.
///
/// Returns a [`Future`] that resolves once macOS accepts or rejects the request.
pub async fn send(content: Notification) -> Result<(), Error> {
    check_bundle()?;
    schedule_inner(content, None).await
}

/// Schedule a notification for immediate delivery, blocking the calling thread.
pub fn send_blocking(content: Notification) -> Result<(), Error> {
    check_bundle()?;
    futures_lite::future::block_on(schedule_inner(content, None))
}

// ── send_with_actions ─────────────────────────────────────────────────────────

/// Schedule a notification with action buttons and wait for the user's response.
///
/// [`ActionCategory`]: crate::un::action::ActionCategory
pub async fn send_with_actions(content: Notification) -> Result<NotificationResponse, Error> {
    check_bundle()?;
    delegate::install();
    let (response_tx, response_rx) = oneshot::channel();
    schedule_inner(content, Some(response_tx)).await?;
    response_rx.await.map_err(|_| Error::NotificationRejected)
}

/// Schedule a notification with action buttons, blocking until the user responds.
///
/// Installs the delegate on the main thread and pumps the main run loop while
/// waiting, so macOS can deliver the delegate callback.
pub fn send_with_actions_blocking(content: Notification) -> Result<NotificationResponse, Error> {
    check_bundle()?;
    delegate::install();
    let (response_tx, mut response_rx) = oneshot::channel();
    let mut fut = std::pin::pin!(schedule_inner(content, Some(response_tx)));

    // Drive the future once to kick off the worker dispatch, which also
    // resolves the scheduling half (accepted/rejected by macOS).
    let result =
        futures_lite::future::block_on(async { futures_lite::future::poll_once(&mut fut).await });
    if let Some(result) = result {
        // Scheduling itself failed — no point waiting for a response.
        return result.and(Err(Error::NotificationRejected));
    }

    // Scheduling accepted. Now pump the main run loop so macOS can deliver
    // didReceiveNotificationResponse on the main thread, and poll response_rx
    // after each tick.
    let run_loop = NSRunLoop::currentRunLoop();
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(&waker);
    loop {
        let until = NSDate::dateWithTimeIntervalSinceNow(0.05);
        unsafe { run_loop.runMode_beforeDate(NSDefaultRunLoopMode, &until) };
        if let std::task::Poll::Ready(result) = std::pin::Pin::new(&mut response_rx).poll(&mut cx) {
            return result.map_err(|_| Error::NotificationRejected);
        }
    }
}
