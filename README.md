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

Five main functions provided:

- `streaming_request`: build streaming requests based on a vector of messages.
- `process_streaming_response`: iterate the streaming response and call the closure user provided.
- `process_streaming_response_with_timeout`: iterate the streaming response with a timeout for each message.
- `stream_to_vec`: iterate the streaming response and generate a vector for further processing.
- `stream_to_vec_with_timeout`: iterate the streaming response with a timeout and generate a vector.

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

Note these functions are for testing purpose only. DO NOT use them in other cases.

## License

`tonic-mock` is distributed under the terms of MIT.

See [LICENSE](LICENSE.md) for details.

Copyright 2021 Tyr Chen
