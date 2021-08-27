use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct CompletionToken<T> {
    inner: Arc<Mutex<Option<T>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

/// CompletionToken
///
/// ```
///     let token = CompletionToken::<()>::new();
///
///     let async_token = token.clone();
///     task::spawn(move || {
///         async_token.set(());
///     });
///
///     token.await;
/// ```
impl<T> CompletionToken<T> {
    pub fn new() -> Self {
        CompletionToken {
            inner: Arc::new(Mutex::new(Option::<T>::None)),
            waker: Arc::new(Mutex::new(Option::<Waker>::None)),
        }
    }

    pub fn set(&self, value: T) {
        let mut inner = self.inner.lock().expect("set inner");
        *inner = Some(value);
        self.wake();
    }

    fn wake(&self) {
        let waker = self.waker.lock().expect("wake waker");

        // If there is a waker, wake it
        if let Some(waker) = &*waker {
            waker.wake_by_ref();
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

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let inner = {
            let mut inner = Mutex::lock(&mut self.inner).expect("poll inner mutex");
            // Calling .take() means that T does not need to be Clone.
            inner.take()
        };

        // Test if the value has been set, this represents a completed future
        match inner {
            // Future is incomplete, so register or reuse a waker
            None => {
                let mut waker = Mutex::lock(&mut self.waker).expect("poll waker mutex");

                // Set a new waker if None
                waker.get_or_insert_with(|| cx.waker().clone());

                // Another task will need to call CompletionToken::wake()
                // to trigger another poll() from the executor
                Poll::Pending
            }

            // The future has completed, take the value
            Some(value) => Poll::Ready(value),
        }
    }
}

impl<T: PartialEq> PartialEq for CompletionToken<T> {
    fn eq(&self, other: &Self) -> bool {
        let this = &*self.inner.lock().expect("eq self inner");
        let that = &*other.inner.lock().expect("eq other inner");

        match (this, that) {
            (Some(this), Some(that)) => this == that,
            _ => false,
        }
    }
}
