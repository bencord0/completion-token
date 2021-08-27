use crate::{Request, Response};
use async_channel::{Receiver, Sender};
use completion_token::CompletionToken;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct State {
    inner: Arc<Mutex<Option<Response>>>,

    tx: Sender<(Request, CompletionToken<Response>)>,
    rx: Receiver<(Request, CompletionToken<Response>)>,
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("can't get response")]
    GetResponseError,
}

impl Default for State {
    fn default() -> State {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        let (tx, rx) = async_channel::unbounded();
        Self {
            inner: Arc::new(Mutex::new(None)),
            tx,
            rx,
        }
    }

    async fn set(&self, value: Response) {
        let mut inner = self.inner.lock().unwrap();
        *inner = Some(value);
    }

    // Spawn thie worker in an executor
    pub async fn worker(&self) -> Result<(), ()> {
        if let Ok((request, token)) = self.rx.recv().await {
            // Create a response from the request content
            let mut response = Response::new();
            response.value = request.value;

            // Store the value in state
            self.set(response.clone()).await;

            // Also Store the value of in the completion token
            // This wakes any callers to `token.await`
            token.set(response.clone());
        }

        Ok(())
    }

    async fn send(&self, request: Request, token: CompletionToken<Response>) {
        // Send work to the worker
        let _ = self.tx.send((request, token)).await;
    }

    pub async fn make_request(&self, request: Request) -> Response {
        let token = CompletionToken::new();

        // Send the request
        self.send(request, token.clone()).await;

        // When the request has been processed, unblock the caller.
        // This won't complete unless `worker()` is executing in a parallel task.
        token.await
    }

    pub async fn get_response(&self) -> Result<Response, StateError> {
        let mut inner = self.inner.lock().unwrap();
        match inner.take() {
            Some(response) => Ok(response),
            None => Err(StateError::GetResponseError),
        }
    }
}
