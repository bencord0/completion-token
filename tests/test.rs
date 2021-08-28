#[macro_use]
extern crate rstest;

mod stubs;
use stubs::{Request, Response, State};

use completion_token::CompletionToken;
use core::time::Duration;
use smol_timeout::TimeoutExt;

use std::error::Error;

#[rstest]
async fn test_state() -> Result<(), Box<dyn Error>> {
    let state = State::new();

    let worker_state = state.clone();
    let worker = async_std::task::spawn(async move { worker_state.worker().await });

    // Send the request
    let request_state = state.clone();
    let response = async_std::task::spawn(async move {
        let request = Request::new("everything is wonderful");
        request_state.make_request(request).await
    });

    let _ = worker
        .timeout(Duration::from_secs(1))
        .await
        .expect("timeout exceeded");

    // Add a timeout, to prevent tests from hanging
    let response = response
        .timeout(Duration::from_secs(1))
        .await
        .expect("timeout exceeded");

    // Check for consistency
    assert_eq!(state.get_response().await?, response);

    // Check the value
    let value = response.value;
    assert_eq!(value, "everything is wonderful");

    Ok(())
}

#[rstest]
async fn test_completion_token() -> Result<(), Box<dyn Error>> {
    let token = CompletionToken::<&str>::new();

    // None means we hit the timeout, token is not complete
    {
        let token = token.clone();
        let result = token.timeout(Duration::from_millis(1)).await;
        assert!(result.is_none());
    }

    // Set the value...
    token.set("Hello World!");

    // ... and let the token can now complete
    let result = token.timeout(Duration::from_secs(1)).await;

    assert_eq!(result, Some("Hello World!"));
    Ok(())
}

#[rstest]
async fn test_cloned_completion_token() -> Result<(), Box<dyn Error>> {
    let token = CompletionToken::<&str>::new();

    {
        // Set the value on the clone...
        let token = token.clone();
        token.set("Hello World!");
    }

    // ... and the original token can now complete
    let result = token.timeout(Duration::from_secs(1)).await;

    assert_eq!(result, Some("Hello World!"));
    Ok(())
}

#[rstest]
async fn test_threaded_token() -> Result<(), Box<dyn Error>> {
    let token = CompletionToken::<&str>::new();

    let thread_token = token.clone();
    std::thread::spawn(move || thread_token.set("Hello World!"));

    let result = token.timeout(Duration::from_secs(1)).await;

    assert_eq!(result, Some("Hello World!"));
    Ok(())
}

#[rstest]
async fn test_asyncstd_token() -> Result<(), Box<dyn Error>> {
    let token = CompletionToken::<&str>::new();

    let async_token = token.clone();
    async_std::task::spawn(async move { async_token.set("Hello World!") }).await;

    let result = token
        .timeout(Duration::from_secs(1))
        .await
        .expect("timeout exceeded");

    assert_eq!(result, "Hello World!");
    Ok(())
}

#[rstest]
async fn test_tokio_token() -> Result<(), Box<dyn Error>> {
    let token = CompletionToken::<&str>::new();

    let tokio_token = token.clone();
    tokio::task::spawn(async move { tokio_token.set("Hello World!") });

    let result = token
        .timeout(Duration::from_secs(1))
        .await
        .expect("timeout exceeded");

    assert_eq!(result, "Hello World!");
    Ok(())
}

#[rstest]
async fn test_take_twice() -> Result<(), Box<dyn Error>> {
    let token = CompletionToken::<&str>::new();

    let t1 = token.clone();
    let t2 = token.clone();

    token.set("Hello");

    assert_eq!(
        "Hello",
        t1.timeout(Duration::from_secs(1))
            .await
            .expect("timeout 1 exceeded")
    );

    token.set("World");
    assert_eq!(
        "World",
        t2.timeout(Duration::from_secs(1))
            .await
            .expect("timeout 2 exceeded")
    );

    Ok(())
}

#[rstest]
async fn test_set_twice() -> Result<(), Box<dyn Error>> {
    let token = CompletionToken::<&str>::new();

    token.set("Hello");
    token.set("World");

    assert_eq!(
        "World",
        token
            .timeout(Duration::from_secs(1))
            .await
            .expect("timeout exceeded")
    );

    Ok(())
}
