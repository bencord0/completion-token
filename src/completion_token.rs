use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use async_lock::Lock;

#[derive(Debug, Clone)]
pub struct CompletionToken<T> {
    inner: Lock<Option<T>>,
    waker: Lock<Option<Waker>>,
}

/// CompletionToken
///
/// ```
///     let token = CompletionToken::<()>::new();
///
///     let async_token = token.clone();
///     task::spawn(move || {
///         async_token.set(()).await;
///     });
///
///     token.await;
/// ```
impl<T> CompletionToken<T> {
    pub fn new() -> Self {
        CompletionToken {
            inner: Lock::new(Option::<T>::None),
            waker: Lock::new(None),
        }
    }

    pub async fn set(&self, value: T) {
        let mut inner = self.inner.lock().await;
        *inner = Some(value);
        self.wake().await;
    }

    async fn wake(&self) {
        let waker = self.waker.lock().await;

        // If there is a waker, wake it
        if let Some(waker) = &*waker {
            waker.clone().wake();
        }
    }
}

impl<T> Default for CompletionToken<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Future for CompletionToken<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        // Calling .take() means that T does not need to be Clone.
        let inner = self.inner.try_lock().map(|mut t| t.take());

        // Test if the value has been set, this represents a completed future
        match inner.flatten() {
            // Future is incomplete, so register a waker
            // Another task will need to call CompletionToken::wake()
            // to trigger another poll() from the executor
            None => {
                if let Some(mut waker) = self.waker.try_lock() {
                    *waker = Some(cx.waker().clone());
                }
                Poll::Pending
            }

            // The future has completed, take the value
            Some(value) => Poll::Ready(value),
        }
    }
}

impl<T: PartialEq> PartialEq for CompletionToken<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self.inner.try_lock(), other.inner.try_lock()) {
            (Some(s), Some(o)) => *s == *o,
            _ => false,
        }
    }
}
