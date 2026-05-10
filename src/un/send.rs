use super::{Error, Notification, check_bundle};
use crate::un::worker;
use block2::RcBlock;
use objc2_foundation::NSUUID;
use objc2_user_notifications::{UNNotificationRequest, UNUserNotificationCenter};
use std::{cell::Cell, future::Future};

fn send_inner(content: Notification) -> impl Future<Output = Result<(), Error>> + Send + 'static {
    log::debug!("un::send: dispatching to worker");
    let (tx, rx) = futures_channel::oneshot::channel::<Result<(), Error>>();
    worker::dispatch(move || {
        log::debug!("un::send: closure entered on worker");
        let un_content = content.build();
        let request_id = NSUUID::new().UUIDString();
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &request_id,
            &un_content,
            None,
        );
        let tx = Cell::new(Some(tx));
        let block = RcBlock::new(move |err: *mut objc2_foundation::NSError| {
            log::debug!(
                "un::send: completion handler fired (err.is_null={})",
                err.is_null()
            );
            if let Some(tx) = tx.take() {
                let result = if err.is_null() {
                    Ok(())
                } else {
                    Err(Error::NotificationRejected)
                };
                if tx.send(result).is_err() {
                    log::warn!("un::send: receiver dropped before completion");
                }
            } else {
                log::warn!("un::send: completion fired twice");
            }
        });
        log::debug!("un::send: calling addNotificationRequest");
        UNUserNotificationCenter::currentNotificationCenter()
            .addNotificationRequest_withCompletionHandler(&request, Some(&block));
        log::debug!("un::send: addNotificationRequest returned");
    });
    async move { rx.await.unwrap_or(Err(Error::NotificationRejected)) }
}

/// Schedule a notification for immediate delivery via `UNUserNotificationCenter`.
///
/// Returns a [`Future`] that resolves once macOS accepts or rejects the
/// request.  Safe to `.await` from any async executor (Tokio, async-std, …);
/// the calling task parks efficiently while the internal worker thread handles
/// the Objective-C work.
///
/// Use [`send_blocking`] only when you are on a thread that is allowed to
/// block.  If you need a fire-and-forget call from synchronous code inside a
/// Tokio runtime, wrap it: `tokio::task::spawn_blocking(|| send_blocking(n))`.
///
/// # Re-entrancy / deadlock warning
///
/// Do **not** call [`send_blocking`] (or any other `un::*_blocking` function)
/// from inside a closure that is already running on the `un` worker thread
/// (i.e. from within a future you dispatched there yourself).  The worker
/// would block waiting for itself to deliver a completion callback — deadlock.
pub async fn send(content: Notification) -> Result<(), Error> {
    check_bundle()?;
    send_inner(content).await
}

/// Schedule a notification for immediate delivery, blocking the calling thread.
///
/// Runs [`send`] to completion using `futures_lite::block_on`.  Safe to
/// call from plain threads, `std::thread::spawn` closures, test bodies, or
/// `tokio::task::spawn_blocking` closures.
///
/// **Do not call from a Tokio (or async-std / smol) worker task directly.**
/// It will block the executor thread for the duration of the macOS round-trip,
/// starving other tasks.  Use [`send`] and `.await` instead.
pub fn send_blocking(content: Notification) -> Result<(), Error> {
    check_bundle()?;
    futures_lite::future::block_on(send_inner(content))
}
