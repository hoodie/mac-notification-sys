//! Request macOS permission to display alerts and play sounds.

use std::{cell::Cell, future::Future};

use block2::RcBlock;
use futures_channel::oneshot;
use objc2::runtime::Bool;
use objc2_user_notifications::{UNAuthorizationOptions, UNUserNotificationCenter};

use super::{Error, check_bundle, worker};

fn request_auth_inner() -> impl Future<Output = Result<bool, Error>> + Send + 'static {
    log::debug!("un::request_authorization: dispatching to worker");
    let (tx, rx) = oneshot::channel::<Result<bool, Error>>();
    worker::dispatch(move || {
        log::debug!("un::request_authorization: closure entered on worker");
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let tx = Cell::new(Some(tx));
        let block = RcBlock::new(move |granted: Bool, err: *mut objc2_foundation::NSError| {
            log::debug!(
                "un::request_authorization: completion handler fired (granted={}, err.is_null={})",
                granted.as_bool(),
                err.is_null()
            );
            if let Some(tx) = tx.take() {
                if tx.send(Ok(granted.as_bool())).is_err() {
                    log::warn!("un::request_authorization: receiver dropped before completion");
                }
            } else {
                log::warn!("un::request_authorization: completion fired twice");
            }
        });
        log::debug!("un::request_authorization: calling requestAuthorizationWithOptions");
        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &block,
        );
        log::debug!("un::request_authorization: requestAuthorizationWithOptions returned");
    });
    async move { rx.await.unwrap_or(Err(Error::NotificationRejected)) }
}

/// Ask macOS for permission to display alerts and play sounds.
///
/// Returns a [`Future`] that resolves once the user answers the permission
/// prompt.  Returns `Ok(true)` if granted, `Ok(false)` if denied.
///
/// Prefer this over [`request_auth_blocking`] from within an async context.
pub async fn request_auth() -> Result<bool, Error> {
    check_bundle()?;
    request_auth_inner().await
}

/// Ask macOS for permission to display alerts and play sounds, blocking the
/// calling thread.
///
/// Returns `Ok(true)` if granted, `Ok(false)` if denied.
pub fn request_auth_blocking() -> Result<bool, Error> {
    check_bundle()?;
    futures_lite::future::block_on(request_auth_inner())
}
