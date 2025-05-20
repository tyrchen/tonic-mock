# tonic-mock

[tonic](https://docs.rs/tonic) is a great crate to build GRPC applications. However, testing RPC built with tonic is not straightforward, especially for the streaming interface. If you have an RPC like this:

```protobuf
rpc Push(stream RequestPush) returns (stream ResponsePush);
```

Testing it usually involves lots of effort on properly mocking the data. This little crate helps to make it easier to mock the incoming data and to manipulate the response so that you could focus on testing the logic itself. For example:

```rust
#[tokio::test]
async fn service_push_works() -> anyhow::Result<()> {
    let mut events: Vec<RequestPush> = Vec::with_capacity(3);
    for i in 0..3 {
        events.push(RequestPush::new(id: Bytes::from(i.to_string), data: Bytes::from("a".repeat(10))));
    }

    // preparing the streaming request
    let req = tonic_mock::streaming_request(events);

    let server = start_server();

    // call the service
    let res = server.push(req).await?;

    // iterate the response and assert the result
    tonic_mock::process_streaming_response(result, |msg, i| {
        assert!(msg.is_ok());
        assert_eq!(msg.as_ref().unwrap().code, i as i32);
    })
    .await;

    Ok(())
}
```

## Features

### Core Functionality

Seven main functions provided:

- `streaming_request`: build streaming requests based on a vector of messages.
- `streaming_request_with_interceptor`: build streaming requests with an interceptor function.
- `request_with_interceptor`: create a standard (non-streaming) request with an interceptor.
- `process_streaming_response`: iterate the streaming response and call the closure user provided.
- `process_streaming_response_with_timeout`: iterate the streaming response with a timeout for each message.
- `stream_to_vec`: iterate the streaming response and generate a vector for further processing.
- `stream_to_vec_with_timeout`: iterate the streaming response with a timeout and generate a vector.

### Bidirectional Streaming Testing

The crate provides a `BidirectionalStreamingTest` utility for fine-grained testing of bidirectional streaming services:

```rust
use std::time::Duration;
use tokio::runtime::Runtime;
use tonic_mock::{BidirectionalStreamingTest, test_utils::TestRequest, test_utils::TestResponse};

// Create a runtime
let rt = Runtime::new().unwrap();

rt.block_on(async {
    // Create a test context with your service function
    let mut test = BidirectionalStreamingTest::new(my_bidirectional_service);

    // Send messages one by one and check responses
    test.send_client_message(TestRequest::new("msg1", "First message")).await;

    // Get response
    if let Some(response) = test.get_server_response().await {
        // Verify response
        assert_eq!(response.code, 200);
        assert!(response.message.contains("First message"));
    }

    // You can also use timeouts
    match test.get_server_response_with_timeout(Duration::from_millis(100)).await {
        Ok(Some(resp)) => {
            // Handle response
        },
        Ok(None) => {
            // No more responses
        },
        Err(status) => {
            // Timeout or other error
        },
    }

    // Complete the test when done
    test.complete().await;
});
```

Key features of `BidirectionalStreamingTest`:

- Send messages one by one to the service
- Receive and test responses individually
- Support for timeouts when receiving responses
- More control over message flow compared to using a pre-collected vector

For a complete example, see `examples/bidirectional_streaming_test_api.rs`.

### Request Interceptors

The crate provides support for request interceptors, which allow you to modify requests before they are sent. This is useful for adding metadata, headers, or performing other customizations:

```rust
use tonic::metadata::MetadataValue;
use tonic_mock::streaming_request_with_interceptor;

// Create a streaming request with an interceptor that adds headers
let request = streaming_request_with_interceptor(messages, |req| {
    // Add authentication header
    req.metadata_mut().insert(
        "authorization",
        MetadataValue::from_static("Bearer token123")
    );

    // Add tracing header
    req.metadata_mut().insert(
        "x-request-id",
        MetadataValue::from_static("trace-456")
    );
});

// For non-streaming requests
let request = request_with_interceptor(message, |req| {
    req.metadata_mut().insert(
        "authorization",
        MetadataValue::from_static("Bearer token123")
    );
});
```

### Timeout Support

The crate provides functions for handling timeouts in streaming responses:

```rust
// Process streaming response with a 1-second timeout for each message
process_streaming_response_with_timeout(
    response,
    Duration::from_secs(1),
    |msg, idx| {
        if msg.is_ok() {
            // Handle successful message
            let response = msg.as_ref().unwrap();
            // ...
        } else {
            // Handle error (could be timeout or other error)
            let error = msg.as_ref().err().unwrap();
            if error.code() == tonic::Code::DeadlineExceeded {
                println!("Timeout occurred: {}", error.message());
            }
        }
    }
).await;

// Convert stream to vector with timeout
let results = stream_to_vec_with_timeout(response, Duration::from_secs(1)).await;
for result in results {
    // Check for timeout or other errors
    if let Err(status) = &result {
        if status.code() == tonic::Code::DeadlineExceeded {
            println!("Timeout occurred: {}", status.message());
            break;
        }
    }
}
```

### Test Utilities

The crate also provides optional test utilities to help with testing gRPC services. Enable these with the `test-utils` feature (enabled by default):

```toml
[dependencies]
tonic-mock = "0.4"  # Test utilities are included by default

# If you want to disable test utilities
tonic-mock = { version = "0.4", default-features = false }
```

Test utilities include:

- `TestRequest` and `TestResponse` types: Simple message types for testing
- `create_test_messages`: Generate test messages with sequential IDs
- `create_stream_response`: Create a streaming response from a vector of messages
- `create_stream_response_with_errors`: Create a streaming response with errors at specified indices
- `assert_message_eq`: Assert that a message matches expected values
- `assert_response_eq`: Assert that a response matches expected values

Example using test utilities:

```rust
use tonic_mock::test_utils::{TestRequest, TestResponse, create_test_messages, create_stream_response};
use tonic_mock::{streaming_request, process_streaming_response};

#[tokio::test]
async fn test_my_service() {
    // Create test messages
    let messages = create_test_messages(5);

    // Create a streaming request
    let request = streaming_request(messages);

    // Call your service
    let response = my_service.call(request).await.unwrap();

    // Process the response
    process_streaming_response(response, |msg: Result<TestResponse, Status>, idx| {
        assert!(msg.is_ok());
        // Assertions on the response
    }).await;
}
```

For a more complete example, check the `examples/grpc_test_demo.rs` file which demonstrates:

- Testing client streaming RPC (client sends multiple messages, server sends one response)
- Testing server streaming RPC (client sends one message, server sends multiple responses)
- Testing bidirectional streaming RPC (client and server both send multiple messages)
- Testing error handling in streaming RPCs
- Testing timeouts in streaming responses
- Using request interceptors to modify requests

Note these functions are for testing purpose only. DO NOT use them in other cases.

## License

`tonic-mock` is distributed under the terms of MIT.

See [LICENSE](LICENSE.md) for details.

Copyright 2021 Tyr Chen
