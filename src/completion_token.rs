use core::{
    future::Future,
    mem,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
enum Token<T> {
    New,
    Pending(Waker),
    Complete(T),
}

#[derive(Debug, Clone)]
pub struct CompletionToken<T> {
    inner: Arc<Mutex<Token<T>>>,
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
            inner: Arc::new(Mutex::new(Token::New)),
        }
    }

    pub fn set(&self, value: T) {
        let inner = &mut *self.inner.lock().expect("set inner");

        let mut token = Token::Complete(value);
        match inner {
            Token::New => {
                mem::swap(inner, &mut token);
            }
            Token::Pending(_waker) => {
                mem::swap(inner, &mut token);

                if let Token::Pending(waker) = token {
                    waker.wake();
                }
            }
            Token::Complete(_old_value) => {
                mem::swap(inner, &mut token);
            }
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
        let inner = &mut *self.inner.lock().expect("poll inner");

        match inner {
            // Future is incomplete, so register a waker
            Token::New => {
                let mut token = Token::Pending(cx.waker().clone());
                mem::swap(inner, &mut token);

                // Another task will need to call CompletionToken::wake()
                // to trigger another poll() from the executor
                Poll::Pending
            }

            // Future is already being polled
            Token::Pending(_waker) => Poll::Pending,

            // The future has completed, take the value
            Token::Complete(_value) => {
                // Reset to new
                let mut token = Token::New;
                mem::swap(inner, &mut token);

                // We hold the lock on inner,
                // and have already matched it as Complete.
                if let Token::Complete(value) = token {
                    Poll::Ready(value)
                } else {
                    // Rare if this occurs, possible race?
                    let mut token = Token::Pending(cx.waker().clone());
                    mem::swap(inner, &mut token);
                    Poll::Pending
                }
            },
        }
    }
}

impl<T: PartialEq> PartialEq for CompletionToken<T> {
    fn eq(&self, other: &Self) -> bool {
        let this = &*self.inner.lock().expect("eq self inner");
        let that = &*other.inner.lock().expect("eq other inner");

        match (this, that) {
            (Token::New, Token::New) => true,

            // Compare pointers
            (Token::Pending(_), Token::Pending(_)) => {
                let thisp = Arc::as_ptr(&self.inner);
                let thatp = Arc::as_ptr(&other.inner);

                thisp == thatp
            }

            // Compare content
            (Token::Complete(this), Token::Complete(that)) => this == that,

            // Mixed states are false
            _ => false,
        }
    }
}
