// Example demonstrating how to use the BidirectionalStreamingTest API
//
// This example shows how to use the BidirectionalStreamingTest utility for
// testing bidirectional streaming gRPC services.

use std::time::Duration;
use tonic::{Request, Response, Status, Streaming};
use tonic_mock::{
    BidirectionalStreamingTest, StreamResponseInner,
    test_utils::{TestRequest, TestResponse},
};

#[tokio::main]
async fn main() {
    println!("=== BidirectionalStreamingTest API Example ===");

    // Run all examples
    basic_example().await;
    interactive_example().await;
    timeout_example().await;

    println!("\nAll examples completed successfully!");
}

// Echo service for basic example - responds immediately to each message
async fn echo_service(
    request: Request<Streaming<TestRequest>>,
) -> Result<Response<StreamResponseInner<TestResponse>>, Status> {
    let mut stream = request.into_inner();

    // Process each message as it comes in and respond immediately
    let response_stream = async_stream::try_stream! {
        let mut counter = 0;

        while let Some(msg) = stream.message().await? {
            counter += 1;
            let id_str = String::from_utf8_lossy(&msg.id).to_string();
            let data_str = String::from_utf8_lossy(&msg.data).to_string();

            println!("  Service received message {}: id={}, data={}", counter, id_str, data_str);

            // Respond immediately to each message
            yield TestResponse::new(
                counter,
                format!("Echo #{}: id={}, data={}", counter, id_str, data_str)
            );
        }

        println!("  Service stream completed");
    };

    Ok(Response::new(Box::pin(response_stream)))
}

// Delayed echo service
async fn delayed_service(
    request: Request<Streaming<TestRequest>>,
) -> Result<Response<StreamResponseInner<TestResponse>>, Status> {
    let mut stream = request.into_inner();

    let response_stream = async_stream::try_stream! {
        let mut counter = 0;

        while let Some(msg) = stream.message().await? {
            counter += 1;
            let id_str = String::from_utf8_lossy(&msg.id).to_string();

            // Add delay
            let delay = counter * 50;
            println!("  Service adding {}ms delay", delay);
            tokio::time::sleep(Duration::from_millis(delay as u64)).await;

            // Create a response
            yield TestResponse::new(
                counter,
                format!("Delayed echo #{} ({}ms): {}", counter, delay, id_str)
            );
        }

        println!("  Delayed service stream completed");
    };

    Ok(Response::new(Box::pin(response_stream)))
}

// Basic example that demonstrates the simplest usage pattern:
// 1. Send all messages
// 2. Call complete()
// 3. Get all responses
async fn basic_example() {
    println!("\nExample 1: Basic bidirectional streaming");
    println!("Pattern: Send all messages → complete() → get all responses");

    // Create a test context with our echo service
    let mut test = BidirectionalStreamingTest::new(echo_service);

    // Send all messages first
    println!("- Sending first message");
    test.send_client_message(TestRequest::new("msg1", "First message"))
        .await;

    println!("- Sending second message");
    test.send_client_message(TestRequest::new("msg2", "Second message"))
        .await;

    // Signal that we're done sending messages
    println!("- Completing the client stream");
    test.complete().await;

    // Now get all responses
    println!("- Getting first response");
    if let Some(response) = test.get_server_response().await {
        println!(
            "- Received response: code={}, message={}",
            response.code, response.message
        );
    } else {
        println!("- No response received");
    }

    println!("- Getting second response");
    if let Some(response) = test.get_server_response().await {
        println!(
            "- Received response: code={}, message={}",
            response.code, response.message
        );
    } else {
        println!("- No response received");
    }

    // No more responses
    if test.get_server_response().await.is_some() {
        println!("- Unexpected extra response received");
    } else {
        println!("- No more responses (as expected)");
    }

    println!("Example 1 completed!");
}

// Interactive example that demonstrates the more flexible usage pattern:
// 1. Send a message
// 2. Get a response
// 3. Send another message
// 4. Get another response
// 5. Complete the test
async fn interactive_example() {
    println!("\nExample 2: Interactive bidirectional streaming");
    println!("Pattern: Send message → get response → send message → get response → complete()");
    println!(
        "IMPORTANT NOTE: For the interactive pattern, you must send ALL messages before getting ANY responses"
    );

    // Create a test context with our echo service
    let mut test = BidirectionalStreamingTest::new(echo_service);

    // Send ALL messages before getting responses
    println!("- Sending first message");
    test.send_client_message(TestRequest::new("msg1", "First message"))
        .await;

    println!("- Sending second message");
    test.send_client_message(TestRequest::new("msg2", "Second message"))
        .await;

    // Indicate we're done sending messages
    println!("- Completing the client stream (required before getting responses)");
    test.complete().await;

    // NOW we can get responses one by one
    println!("- Getting first response");
    if let Some(response) = test.get_server_response().await {
        println!(
            "- Received response: code={}, message={}",
            response.code, response.message
        );
    } else {
        println!("- No response received");
    }

    println!("- Getting second response");
    if let Some(response) = test.get_server_response().await {
        println!(
            "- Received response: code={}, message={}",
            response.code, response.message
        );
    } else {
        println!("- No response received");
    }

    println!("Example 2 completed!");
}

async fn timeout_example() {
    println!("\nExample 3: Testing with timeouts");

    // Create a test context with our delayed service
    let mut test = BidirectionalStreamingTest::new(delayed_service);

    // Send messages
    println!("- Sending first message");
    test.send_client_message(TestRequest::new("test1", "Test message"))
        .await;

    println!("- Sending second message (will have 100ms delay)");
    test.send_client_message(TestRequest::new("test2", "Will have 100ms delay"))
        .await;

    // Complete the test to signal no more messages
    println!("- Completing client stream");
    test.complete().await;

    // Get the first response with sufficient timeout
    println!("- Getting first response with 100ms timeout (should succeed)");
    match test
        .get_server_response_with_timeout(Duration::from_millis(100))
        .await
    {
        Ok(Some(resp)) => {
            println!(
                "- Received response: code={}, message={}",
                resp.code, resp.message
            );
        }
        Ok(None) => println!("- No response received"),
        Err(status) => println!("- Error: {}", status),
    }

    // Try with too short timeout for second message (has 100ms delay)
    println!("- Getting second response with 50ms timeout (should timeout)");
    match test
        .get_server_response_with_timeout(Duration::from_millis(50))
        .await
    {
        Ok(Some(_)) => println!("- Unexpected: Received response within timeout"),
        Ok(None) => println!("- No response received"),
        Err(status) => println!("- Error as expected: {}", status),
    }

    // Try again with longer timeout
    println!("- Retrying with 150ms timeout (should succeed)");
    match test
        .get_server_response_with_timeout(Duration::from_millis(150))
        .await
    {
        Ok(Some(resp)) => {
            println!(
                "- Received response: code={}, message={}",
                resp.code, resp.message
            );
        }
        Ok(None) => println!("- No response received"),
        Err(status) => println!("- Error: {}", status),
    }

    println!("Example 3 completed!");
}
