/*!
# Test Utilities

This module provides helper types and functions to simplify testing gRPC services
with streaming interfaces. It is enabled by default but can be disabled by setting
`default-features = false` when including the crate.

## Core Components

- [`TestRequest`]: A simple request message type for testing
- [`TestResponse`]: A simple response message type for testing
- [`create_test_messages`]: Create a vector of test messages with sequential IDs
- [`create_stream_response`]: Create a streaming response from a vector of messages
- [`create_stream_response_with_errors`]: Create a streaming response with errors at specified indices
- [`assert_message_eq`]: Assert that a message matches expected values
- [`assert_response_eq`]: Assert that a response matches expected values

## Example Usage

```rust
use tonic::{Request, Response, Status, Code};
use tonic_mock::{streaming_request, process_streaming_response};
use tonic_mock::test_utils::{
    TestRequest, TestResponse, create_test_messages,
    create_stream_response, assert_response_eq
};

// Create test messages
let messages = create_test_messages(5);
assert_eq!(messages.len(), 5);

// Create a streaming request
let request = streaming_request(messages);

// Call your service (or mock it for this example)
let responses = vec![
    TestResponse::new(200, "OK: 0"),
    TestResponse::new(200, "OK: 1"),
    TestResponse::new(200, "OK: 2"),
    TestResponse::new(200, "OK: 3"),
    TestResponse::new(200, "OK: 4"),
];
let response = create_stream_response(responses);

// Process the streaming response
process_streaming_response(response, |result, index| {
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_response_eq(&response, 200, format!("OK: {}", index));
}).await;

// Test error handling
let responses = vec![
    TestResponse::new(200, "OK: 0"),
    TestResponse::new(200, "OK: 1"),
    TestResponse::new(200, "OK: 2"),
];
let error_status = Status::new(Code::Internal, "Simulated error");
let response = create_stream_response_with_errors(
    responses,
    vec![1], // Error at index 1
    error_status
);

// Process response with errors
process_streaming_response(response, |result, index| {
    match index {
        1 => {
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().code(), Code::Internal);
        },
        _ => {
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.code, 200);
        }
    }
}).await;
```

These utilities make it easier to test gRPC streaming services by providing
ready-to-use message types and helper functions for common testing patterns.
*/

use crate::StreamResponseInner;
use bytes::Bytes;
use prost::Message;
use tonic::{Response, Status};

/// Test request message for use in gRPC service tests
///
/// This provides a simple message type that implements the required traits
/// for use with tonic and can be used for testing streaming requests.
#[derive(Clone, PartialEq, Message)]
pub struct TestRequest {
    #[prost(bytes = "bytes", tag = "1")]
    pub id: Bytes,
    #[prost(bytes = "bytes", tag = "2")]
    pub data: Bytes,
}

impl TestRequest {
    /// Create a new test request with the given ID and data
    pub fn new(id: impl Into<Bytes>, data: impl Into<Bytes>) -> Self {
        Self {
            id: id.into(),
            data: data.into(),
        }
    }
}

/// Test response message for use in gRPC service tests
///
/// This provides a simple response type that can be used for testing
/// streaming responses from gRPC services.
#[derive(Clone, PartialEq, Message)]
pub struct TestResponse {
    #[prost(int32, tag = "1")]
    pub code: i32,
    #[prost(string, tag = "2")]
    pub message: String,
}

impl TestResponse {
    /// Create a new test response with the given code and message
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Create a vector of test messages with sequential IDs
///
/// This is useful for generating a batch of test messages to use
/// with the streaming_request function.
///
/// # Example
/// ```
/// # use tonic_mock::test_utils::{create_test_messages, TestRequest};
/// let messages = create_test_messages(5);
/// assert_eq!(messages.len(), 5);
/// ```
pub fn create_test_messages(count: usize) -> Vec<TestRequest> {
    let mut messages = Vec::with_capacity(count);
    for i in 0..count {
        messages.push(TestRequest::new(i.to_string(), format!("test_data_{}", i)));
    }
    messages
}

/// Create a streaming response from a vector of response messages
///
/// This is useful for simulating streaming responses in test code.
///
/// # Example
/// ```
/// # use tonic_mock::test_utils::{create_stream_response, TestResponse};
/// let responses = vec![
///     TestResponse::new(0, "Response 0"),
///     TestResponse::new(1, "Response 1"),
/// ];
/// let stream_response = create_stream_response(responses);
/// ```
pub fn create_stream_response<T>(responses: Vec<T>) -> Response<StreamResponseInner<T>>
where
    T: Clone + Send + Sync + 'static,
{
    #[cfg(feature = "test-utils")]
    {
        let stream = async_stream::try_stream! {
            for response in responses {
                yield response;
            }
        };

        Response::new(Box::pin(stream))
    }

    #[cfg(not(feature = "test-utils"))]
    {
        unimplemented!("This function requires the test-utils feature")
    }
}

/// Create a streaming response with errors at specified indices
///
/// This is useful for testing error handling in code that processes
/// streaming responses.
///
/// # Example
/// ```
/// # use tonic_mock::test_utils::{create_stream_response_with_errors, TestResponse};
/// # use tonic::{Status, Code};
/// let responses = vec![
///     TestResponse::new(0, "Response 0"),
///     TestResponse::new(1, "Response 1"),
///     TestResponse::new(2, "Response 2"),
/// ];
/// let error_status = Status::new(Code::Internal, "Test error");
/// let stream_response = create_stream_response_with_errors(
///     responses,
///     vec![1],
///     error_status
/// );
/// ```
pub fn create_stream_response_with_errors<T>(
    responses: Vec<T>,
    error_indices: Vec<usize>,
    error_status: Status,
) -> Response<StreamResponseInner<T>>
where
    T: Clone + Send + Sync + 'static,
{
    #[cfg(feature = "test-utils")]
    {
        let stream = async_stream::try_stream! {
            for (i, response) in responses.into_iter().enumerate() {
                if error_indices.contains(&i) {
                    yield Err(error_status.clone())?;
                } else {
                    yield response;
                }
            }
        };

        Response::new(Box::pin(stream))
    }

    #[cfg(not(feature = "test-utils"))]
    {
        unimplemented!("This function requires the test-utils feature")
    }
}

/// Assert that a test message matches the expected ID and data
///
/// This is a convenience function for testing that a message's content
/// matches the expected values.
///
/// # Example
/// ```
/// # use tonic_mock::test_utils::{assert_message_eq, TestRequest};
/// let message = TestRequest::new("test_id", "test_data");
/// assert_message_eq(&message, "test_id", "test_data");
/// ```
pub fn assert_message_eq(message: &TestRequest, id: impl AsRef<str>, data: impl AsRef<str>) {
    let id_bytes = Bytes::from(id.as_ref().to_string());
    let data_bytes = Bytes::from(data.as_ref().to_string());
    assert_eq!(message.id, id_bytes);
    assert_eq!(message.data, data_bytes);
}

/// Assert that a test response matches the expected code and message
///
/// This is a convenience function for testing that a response's content
/// matches the expected values.
///
/// # Example
/// ```
/// # use tonic_mock::test_utils::{assert_response_eq, TestResponse};
/// let response = TestResponse::new(200, "OK");
/// assert_response_eq(&response, 200, "OK");
/// ```
pub fn assert_response_eq(response: &TestResponse, code: i32, message: impl AsRef<str>) {
    assert_eq!(response.code, code);
    assert_eq!(response.message, message.as_ref());
}
