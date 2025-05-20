# tonic-mock Tutorial

This tutorial demonstrates how to use `tonic-mock` to test gRPC services in Rust. The library provides utilities for testing all types of gRPC interactions, from simple unary calls to complex bidirectional streaming.

## Table of Contents

- [Testing Unary Calls](#testing-unary-calls)
- [Testing Client Streaming](#testing-client-streaming)
- [Testing Server Streaming](#testing-server-streaming)
- [Testing Bidirectional Streaming](#testing-bidirectional-streaming)
- [Request Interceptors](#request-interceptors)
- [Timeout Handling](#timeout-handling)
- [Error Testing](#error-testing)

## Testing Unary Calls

For unary calls (single request, single response), you can use `tonic::Request` directly:

```rust
use tonic::{Request, Response, Status};
use tonic_mock::request_with_interceptor;
use your_proto::YourRequest;

// Create a request
let request = Request::new(YourRequest { /* fields */ });

// Call your service
let response = your_service.unary_method(request).await?;

// Check the response
assert_eq!(response.into_inner().some_field, expected_value);
```

You can also use the `request_with_interceptor` function to modify the request before sending it:

```rust
let request = request_with_interceptor(YourRequest { /* fields */ }, |req| {
    // Add metadata or modify the request
    req.metadata_mut().insert("auth", "Bearer token".parse().unwrap());
});
```

## Testing Client Streaming

For client streaming (multiple requests, single response), use `streaming_request` to create a stream of messages:

```rust
use tonic_mock::streaming_request;
use your_proto::{StreamRequest, StreamResponse};

// Create a vector of messages to send
let messages = vec![
    StreamRequest { id: 1, data: "first".to_string() },
    StreamRequest { id: 2, data: "second".to_string() },
    StreamRequest { id: 3, data: "third".to_string() },
];

// Create the streaming request
let request = streaming_request(messages);

// Call your service
let response = your_service.client_streaming_method(request).await?;

// Check the response
let response = response.into_inner();
assert_eq!(response.count, 3); // Assuming the response includes the count of messages
```

## Testing Server Streaming

For server streaming (single request, multiple responses), you can use the `process_streaming_response` or `stream_to_vec` functions to process the stream:

```rust
use tonic_mock::{process_streaming_response, stream_to_vec};
use your_proto::{ServerStreamRequest, ServerStreamResponse};

// Create the request
let request = Request::new(ServerStreamRequest { count: 3 });

// Call your service
let response = your_service.server_streaming_method(request).await?;

// Option 1: Process responses with a callback
process_streaming_response(response, |msg, idx| {
    assert!(msg.is_ok());
    let response = msg.unwrap();
    assert_eq!(response.index, idx as i32);
    // More assertions...
}).await;

// Option 2: Convert the stream to a vector
let request = Request::new(ServerStreamRequest { count: 3 });
let response = your_service.server_streaming_method(request).await?;
let results = stream_to_vec(response).await;

assert_eq!(results.len(), 3);
for (i, result) in results.iter().enumerate() {
    assert!(result.is_ok());
    let response = result.as_ref().unwrap();
    assert_eq!(response.index, i as i32);
    // More assertions...
}
```

## Testing Bidirectional Streaming

For bidirectional streaming (multiple requests, multiple responses), there are two main approaches:

### Approach 1: Using `streaming_request` for collected messages

```rust
use futures::StreamExt;
use tonic_mock::streaming_request;
use your_proto::{BidiRequest, BidiResponse};

// Create a vector of messages to send
let messages = vec![
    BidiRequest { id: 1, data: "first".to_string() },
    BidiRequest { id: 2, data: "second".to_string() },
];

// Create the streaming request
let request = streaming_request(messages);

// Call your service
let response = your_service.bidirectional_streaming_method(request).await?;

// Process the responses
let mut stream = response.into_inner();
let mut responses = Vec::new();

while let Some(result) = stream.next().await {
    responses.push(result?);
}

assert_eq!(responses.len(), 2);
assert_eq!(responses[0].id, 1);
assert_eq!(responses[1].id, 2);
```

### Approach 2: Using tokio channels for interactive testing

This approach allows you to send messages and receive responses interactively:

```rust
use futures::StreamExt;
use tokio::sync::mpsc;
use tonic_mock::streaming_request;
use your_proto::{BidiRequest, BidiResponse};

// Create channels for the client and service
let (client_tx, mut client_rx) = mpsc::channel::<BidiRequest>(10);
let (service_tx, mut service_rx) = mpsc::channel::<BidiResponse>(10);

// Create a task to handle the service
let service_task = tokio::spawn(async move {
    // Collect client messages
    let mut messages = Vec::new();
    while let Some(msg) = client_rx.recv().await {
        messages.push(msg);
    }

    // Create a request and call the service
    let request = streaming_request(messages);
    let response = your_service.bidirectional_streaming_method(request).await.unwrap();

    // Send responses back through the channel
    let mut stream = response.into_inner();
    while let Some(result) = stream.next().await {
        if let Ok(resp) = result {
            let _ = service_tx.send(resp).await;
        }
    }
});

// Send a message
client_tx.send(BidiRequest { id: 1, data: "test".to_string() }).await.unwrap();

// Get the response
let response = service_rx.recv().await.unwrap();
assert_eq!(response.id, 1);

// Send another message
client_tx.send(BidiRequest { id: 2, data: "more".to_string() }).await.unwrap();

// Get the response
let response = service_rx.recv().await.unwrap();
assert_eq!(response.id, 2);

// Close the client channel to signal we're done
drop(client_tx);

// Wait for the service task to complete
service_task.await.unwrap();
```

## Request Interceptors

Request interceptors allow you to modify requests before they are sent to the service. This is useful for adding metadata, headers, or other customizations:

```rust
use tonic::{metadata::MetadataValue, Request};
use tonic_mock::streaming_request_with_interceptor;
use your_proto::StreamRequest;

let messages = vec![
    StreamRequest { id: 1, data: "first".to_string() },
    StreamRequest { id: 2, data: "second".to_string() },
];

// Create a request with an interceptor
let request = streaming_request_with_interceptor(messages, |req| {
    // Add authorization header
    req.metadata_mut().insert(
        "authorization",
        MetadataValue::from_static("Bearer token123"),
    );

    // Add trace ID
    req.metadata_mut().insert(
        "x-trace-id",
        MetadataValue::from_static("trace-456"),
    );
});

// The request now has the metadata set by the interceptor
assert_eq!(
    request.metadata().get("authorization").unwrap(),
    "Bearer token123"
);
```

## Timeout Handling

The library provides functions for handling timeouts in streaming responses:

```rust
use std::time::Duration;
use tonic_mock::{process_streaming_response_with_timeout, stream_to_vec_with_timeout};
use your_proto::ServerStreamRequest;

// Create the request
let request = Request::new(ServerStreamRequest { count: 3 });

// Call your service
let response = your_service.server_streaming_method(request).await?;

// Process with timeout
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
```

## Error Testing

To test error handling, you can create services that return errors in specific situations:

```rust
use tonic::{Code, Status};
use your_proto::ServerStreamRequest;

// Test a service method that should return an error for invalid input
let request = Request::new(ServerStreamRequest { count: -1 });
let result = your_service.server_streaming_method(request).await;

assert!(result.is_err());
let status = result.err().unwrap();
assert_eq!(status.code(), Code::InvalidArgument);
assert!(status.message().contains("negative count"));
```

For testing errors in streaming responses, you can use the `create_stream_response_with_errors` function from the test_utils module:

```rust
use tonic::{Response, Status};
use tonic_mock::test_utils::{TestResponse, create_stream_response_with_errors};
use tonic_mock::stream_to_vec;

// Create test responses
let responses = vec![
    TestResponse::new(0, "Response 0"),
    TestResponse::new(1, "Response 1"),
    TestResponse::new(2, "Response 2"),
];

// Create an error status
let error_status = Status::internal("Test error");

// Create a streaming response with errors at specific indices
let stream_response = create_stream_response_with_errors(
    responses,
    vec![1], // Error at index 1
    error_status
);

// Process the stream
let results = stream_to_vec(stream_response).await;

// Check results
assert_eq!(results.len(), 2); // Only get up to the error
assert!(results[0].is_ok());
assert!(results[1].is_err());
assert_eq!(results[1].as_ref().err().unwrap().code(), Code::Internal);
```

## Conclusion

The `tonic-mock` library provides a rich set of utilities for testing gRPC services in Rust. By using these tools, you can thoroughly test your services without the complexity of setting up a full gRPC environment.

For more examples, check out the [examples directory](https://github.com/tyrchen/tonic-mock/tree/main/examples) in the repository.
