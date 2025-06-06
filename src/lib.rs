/*!
# tonic-mock

A comprehensive testing utility for [tonic](https://docs.rs/tonic) gRPC services in Rust.

This crate helps you test gRPC services with minimal effort by providing utilities to:
- Create and manipulate streaming requests and responses
- Process streaming responses with timeouts
- Test bidirectional streaming interactions
- Mock gRPC clients for isolated testing
- Add interceptors to modify requests

## Core Functionality

Seven main functions are provided:

- [`streaming_request`]: Build streaming requests based on a vector of messages.
- [`streaming_request_with_interceptor`]: Build streaming requests with an interceptor function.
- [`request_with_interceptor`]: Create a standard (non-streaming) request with an interceptor.
- [`process_streaming_response`]: Iterate the streaming response and call the closure user provided.
- [`process_streaming_response_with_timeout`]: Iterate the streaming response with a timeout for each message.
- [`stream_to_vec`]: Iterate the streaming response and generate a vector for further processing.
- [`stream_to_vec_with_timeout`]: Iterate the streaming response with a timeout and generate a vector.

Additionally, [`BidirectionalStreamingTest`] provides utilities for fine-grained testing of bidirectional streaming services,
and the [`client_mock`] module allows mocking gRPC clients.

## Basic Example

```rust
use tonic::{Request, Response, Status};
use tonic_mock::{streaming_request, process_streaming_response};

#[tokio::test]
async fn service_push_works() -> Result<(), Box<dyn std::error::Error>> {
    // Create test data
    let events = vec![
        RequestPush::new("1", "data1"),
        RequestPush::new("2", "data2"),
        RequestPush::new("3", "data3"),
    ];

    // Create a streaming request
    let req = streaming_request(events);

    // Call your service
    let res = your_service.push(req).await?;

    // Process the streaming response
    process_streaming_response(res, |msg, i| {
        assert!(msg.is_ok());
        assert_eq!(msg.as_ref().unwrap().code, i as i32);
    })
    .await;

    Ok(())
}
```

## Testing Client Streaming

For client streaming (multiple requests, single response), use [`streaming_request`] to create a stream of messages:

```rust
use tonic_mock::streaming_request;

// Create a vector of messages to send
let messages = vec![
    StreamRequest { id: 1, data: "first".to_string() },
    StreamRequest { id: 2, data: "second".to_string() },
];

// Create the streaming request
let request = streaming_request(messages);

// Call your service
let response = your_service.client_streaming_method(request).await?;

// Check the response
let response = response.into_inner();
assert_eq!(response.count, 2);
```

## Testing Server Streaming

For server streaming (single request, multiple responses), use [`process_streaming_response`] or [`stream_to_vec`]:

```rust
use tonic_mock::{process_streaming_response, stream_to_vec};

// Create the request
let request = Request::new(ServerStreamRequest { count: 3 });

// Call your service
let response = your_service.server_streaming_method(request).await?;

// Process responses with a callback
process_streaming_response(response, |msg, idx| {
    assert!(msg.is_ok());
    let response = msg.unwrap();
    assert_eq!(response.index, idx as i32);
}).await;

// Or convert the stream to a vector
let request = Request::new(ServerStreamRequest { count: 3 });
let response = your_service.server_streaming_method(request).await?;
let results = stream_to_vec(response).await;

assert_eq!(results.len(), 3);
for (i, result) in results.iter().enumerate() {
    assert!(result.is_ok());
    let response = result.as_ref().unwrap();
    assert_eq!(response.index, i as i32);
}
```

## Bidirectional Streaming

Use [`BidirectionalStreamingTest`] for testing bidirectional streaming services:

```rust
use std::time::Duration;
use tonic_mock::BidirectionalStreamingTest;

#[tokio::test]
async fn test_bidirectional_service() {
    // Create a test context with your service function
    let mut test = BidirectionalStreamingTest::new(my_bidirectional_service);

    // Send messages one by one
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
}
```

## Mocking gRPC Clients

The [`client_mock`] module provides the [`MockableGrpcClient`] for mocking gRPC clients:

```rust
use tonic_mock::client_mock::{MockableGrpcClient, MockResponseDefinition, GrpcClientExt};
use tonic::{Request, Status, Code};
use prost::Message;

// Define a client type that will use the mock
#[derive(Clone)]
struct UserServiceClient<T> {
    inner: T,
}

// Implement the GrpcClientExt trait for your client
impl GrpcClientExt<UserServiceClient<MockableGrpcClient>> for UserServiceClient<MockableGrpcClient> {
    fn with_mock(mock: MockableGrpcClient) -> Self {
        Self { inner: mock }
    }
}

#[tokio::test]
async fn test_user_service_client() {
    // Create a mock client
    let mock = MockableGrpcClient::new();

    // Configure mock responses
    mock.mock::<UserRequest, UserResponse>("user.UserService", "GetUser")
        .respond_when(
            |req| req.user_id == "existing",
            MockResponseDefinition::ok(UserResponse {
                name: "Existing User".to_string(),
            })
        )
        .await
        .respond_with(
            MockResponseDefinition::err(Status::new(Code::NotFound, "User not found"))
        )
        .await;

    // Create a client with the mock
    let mut client = UserServiceClient::with_mock(mock.clone());

    // Make a request
    let request = Request::new(UserRequest { user_id: "existing".to_string() });
    let response = client.get_user(request).await.unwrap();

    // Verify the response
    assert_eq!(response.get_ref().name, "Existing User");

    // Reset mock when done
    mock.reset().await;
}
```

## Request Interceptors

Use interceptors to modify requests before they are sent:

```rust
use tonic::metadata::MetadataValue;
use tonic_mock::streaming_request_with_interceptor;

// Create a streaming request with an interceptor
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
```

## Timeout Support

Handle timeouts in streaming responses:

```rust
use std::time::Duration;
use tonic_mock::process_streaming_response_with_timeout;

// Process response with a 1-second timeout for each message
process_streaming_response_with_timeout(
    response,
    Duration::from_secs(1),
    |msg, idx| {
        if msg.is_ok() {
            // Handle successful message
            let response = msg.as_ref().unwrap();
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

## Test Utilities

The crate provides optional test utilities with the `test-utils` feature (enabled by default):

- [`TestRequest`] and [`TestResponse`]: Simple message types for testing
- [`create_test_messages`]: Generate test messages with sequential IDs
- [`create_stream_response`]: Create a streaming response from a vector of messages
- [`create_stream_response_with_errors`]: Create a streaming response with errors at specified indices
- [`assert_message_eq`]: Assert that a message matches expected values
- [`assert_response_eq`]: Assert that a response matches expected values

## gRPC Mock Utilities

The [`grpc_mock`] module provides low-level utilities for mocking gRPC messages:

- [`encode_grpc_request`](grpc_mock::encode_grpc_request): Encode a request message
- [`encode_grpc_response`](grpc_mock::encode_grpc_response): Encode a response message
- [`decode_grpc_message`](grpc_mock::decode_grpc_message): Decode a gRPC message
- [`mock_grpc_call`](grpc_mock::mock_grpc_call): Mock a gRPC call with a handler function
- [`create_grpc_uri`](grpc_mock::create_grpc_uri): Create a gRPC URI for a service method

*/

use futures::{Stream, StreamExt};
use prost::Message;
use std::{fmt::Debug, pin::Pin, time::Duration};
use tokio::time::timeout;
use tonic::{Request, Response, Status, Streaming};

pub mod client_mock;
pub mod grpc_mock;
mod mock;

pub use client_mock::{GrpcClientExt, MockResponseDefinition, MockableGrpcClient};
pub use mock::{MockBody, ProstDecoder};

#[cfg(feature = "test-utils")]
pub mod test_utils;

#[cfg(feature = "test-utils")]
pub use test_utils::*;

/// Type alias for convenience
///
/// The inner type of a streaming response
pub type StreamResponseInner<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + 'static>>;

/// Type alias for convenience
///
/// A streaming response from a mock service
pub type StreamResponse<T> = Response<StreamResponseInner<T>>;

/// Type alias for a request interceptor function
pub type RequestInterceptor<T> = Box<dyn FnMut(&mut Request<T>) + Send>;

/// Generate streaming request for GRPC
///
/// When testing streaming RPC implemented with tonic, it is pretty clumsy
/// to build the streaming request, this function extracted test code and prost
/// decoder from tonic source code and wrap it with a nice interface. With it,
/// testing your streaming RPC implementation is much easier.
///
/// Usage:
/// ```
/// use bytes::Bytes;
/// use prost::Message;
/// use tonic_mock::streaming_request;
///
/// // normally this should be generated from protos with prost
/// #[derive(Clone, PartialEq, Message)]
/// pub struct Event {
///     #[prost(bytes = "bytes", tag = "1")]
///     pub id: Bytes,
///     #[prost(bytes = "bytes", tag = "2")]
///     pub data: Bytes,
/// }
///
/// let event = Event { id: Bytes::from("1"), data: Bytes::from("a".repeat(10)) };
/// let mut events = vec![event.clone(), event.clone(), event];
/// let stream = tonic_mock::streaming_request(events);
///
pub fn streaming_request<T>(messages: Vec<T>) -> Request<Streaming<T>>
where
    T: Message + Default + Send + 'static,
{
    let body = MockBody::<T>::new(messages);
    let decoder: ProstDecoder<T> = ProstDecoder::new();
    let stream = Streaming::new_request(decoder, body, None, None);

    Request::new(stream)
}

/// Generate streaming request for GRPC with an interceptor
///
/// This function is similar to `streaming_request` but allows specifying an interceptor
/// function that can modify the request before it's returned. This is useful for adding
/// metadata, headers, or other customizations to the request.
///
/// # Arguments
/// * `messages` - The vector of messages to include in the request
/// * `interceptor` - A function that can modify the request (e.g., to add metadata)
///
/// # Returns
/// A `Request<Streaming<T>>` with the interceptor applied
///
/// # Example
/// ```
/// use bytes::Bytes;
/// use prost::Message;
/// use tonic::{metadata::MetadataValue, Request};
/// use tonic_mock::streaming_request_with_interceptor;
///
/// #[derive(Clone, PartialEq, Message)]
/// pub struct Event {
///     #[prost(bytes = "bytes", tag = "1")]
///     pub id: Bytes,
///     #[prost(bytes = "bytes", tag = "2")]
///     pub data: Bytes,
/// }
///
/// let event = Event { id: Bytes::from("1"), data: Bytes::from("test") };
/// let events = vec![event.clone(), event.clone()];
///
/// // Create a request with an interceptor that adds metadata
/// let request = streaming_request_with_interceptor(events, |req| {
///     req.metadata_mut().insert(
///         "authorization",
///         MetadataValue::from_static("Bearer token123"),
///     );
///     req.metadata_mut().insert(
///         "x-request-id",
///         MetadataValue::from_static("test-request-id"),
///     );
/// });
///
/// // The request now has the metadata set by the interceptor
/// assert_eq!(
///     request.metadata().get("authorization").unwrap(),
///     "Bearer token123"
/// );
/// ```
pub fn streaming_request_with_interceptor<T, F>(
    messages: Vec<T>,
    mut interceptor: F,
) -> Request<Streaming<T>>
where
    T: Message + Default + 'static,
    F: FnMut(&mut Request<Streaming<T>>) + Send + 'static,
{
    let mut request = streaming_request(messages);
    interceptor(&mut request);
    request
}

/// Create a regular (non-streaming) request with an interceptor
///
/// This function creates a standard tonic Request and applies the provided interceptor
/// function to it. This is useful for adding metadata, headers, or other customizations
/// to regular (non-streaming) requests.
///
/// # Arguments
/// * `message` - The message to include in the request
/// * `interceptor` - A function that can modify the request (e.g., to add metadata)
///
/// # Returns
/// A `Request<T>` with the interceptor applied
///
/// # Example
/// ```
/// use bytes::Bytes;
/// use prost::Message;
/// use tonic::{metadata::MetadataValue, Request};
/// use tonic_mock::request_with_interceptor;
///
/// #[derive(Clone, PartialEq, Message)]
/// pub struct GetUserRequest {
///     #[prost(string, tag = "1")]
///     pub user_id: String,
/// }
///
/// let request_msg = GetUserRequest { user_id: "user123".to_string() };
///
/// // Create a request with an interceptor that adds metadata
/// let request = request_with_interceptor(request_msg, |req| {
///     req.metadata_mut().insert(
///         "authorization",
///         MetadataValue::from_static("Bearer token123"),
///     );
/// });
///
/// // The request now has the authorization metadata
/// assert_eq!(
///     request.metadata().get("authorization").unwrap(),
///     "Bearer token123"
/// );
/// ```
pub fn request_with_interceptor<T, F>(message: T, mut interceptor: F) -> Request<T>
where
    T: Debug + Send + 'static,
    F: FnMut(&mut Request<T>) + Send + 'static,
{
    let mut request = Request::new(message);
    interceptor(&mut request);
    request
}

/// a simple wrapper to process and validate streaming response
///
/// Usage:
/// ```
/// use tonic::{Response, Status};
/// use futures::Stream;
/// use std::pin::Pin;
///
/// #[derive(Clone, PartialEq, ::prost::Message)]
/// pub struct ResponsePush {
///     #[prost(int32, tag = "1")]
///     pub code: i32,
/// }
///
/// // below code is to mimic a stream response from a GRPC service
/// let output = async_stream::try_stream! {
///     yield ResponsePush { code: 0 };
///     yield ResponsePush { code: 1 };
///     yield ResponsePush { code: 2 };
/// };
/// let response = Response::new(Box::pin(output) as tonic_mock::StreamResponseInner<ResponsePush>);
/// let rt = tokio::runtime::Runtime::new().unwrap();
///
/// // now we process the events
/// rt.block_on(async {
///     tonic_mock::process_streaming_response(response, |msg, i| {
///         assert!(msg.is_ok());
///         assert_eq!(msg.as_ref().unwrap().code, i as i32);
///     }).await;
/// });
/// ```
pub async fn process_streaming_response<T, F>(response: StreamResponse<T>, f: F)
where
    T: Message + Default + 'static,
    F: Fn(Result<T, Status>, usize),
{
    let mut i: usize = 0;
    let mut messages = response.into_inner();
    while let Some(v) = messages.next().await {
        f(v, i);
        i += 1;
    }
}

/// Process a streaming response with a configurable timeout
///
/// This function is similar to `process_streaming_response` but adds a timeout for each message.
/// If a message is not received within the specified timeout, the callback will be invoked with
/// a `Status::deadline_exceeded` error.
///
/// # Arguments
/// * `response` - The streaming response to process
/// * `timeout_duration` - The maximum time to wait for each message
/// * `f` - A callback function that receives each message result and its index
///
/// # Example
/// ```
/// use tonic::{Response, Status};
/// use futures::Stream;
/// use std::{pin::Pin, time::Duration};
///
/// #[derive(Clone, PartialEq, ::prost::Message)]
/// pub struct ResponsePush {
///     #[prost(int32, tag = "1")]
///     pub code: i32,
/// }
///
/// // below code is to mimic a stream response from a GRPC service
/// let output = async_stream::try_stream! {
///     yield ResponsePush { code: 0 };
///     yield ResponsePush { code: 1 };
///     yield ResponsePush { code: 2 };
/// };
/// let response = Response::new(Box::pin(output) as tonic_mock::StreamResponseInner<ResponsePush>);
/// let rt = tokio::runtime::Runtime::new().unwrap();
///
/// // now we process the events with a timeout
/// rt.block_on(async {
///     tonic_mock::process_streaming_response_with_timeout(
///         response,
///         Duration::from_secs(1),
///         |msg, i| {
///             assert!(msg.is_ok());
///             assert_eq!(msg.as_ref().unwrap().code, i as i32);
///         }
///     ).await;
/// });
/// ```
pub async fn process_streaming_response_with_timeout<T, F>(
    response: StreamResponse<T>,
    timeout_duration: Duration,
    f: F,
) where
    T: Message + Default + 'static,
    F: Fn(Result<T, Status>, usize),
{
    let mut i: usize = 0;
    let mut messages = response.into_inner();
    loop {
        match timeout(timeout_duration, messages.next()).await {
            Ok(Some(v)) => {
                f(v, i);
                i += 1;
            }
            Ok(None) => break, // Stream is done
            Err(_) => {
                // Timeout occurred
                f(
                    Err(Status::deadline_exceeded(format!(
                        "Timeout waiting for message {}: exceeded {:?}",
                        i, timeout_duration
                    ))),
                    i,
                );
                break;
            }
        }
    }
}

/// convert a streaming response to a Vec for simplified testing
///
/// Usage:
/// ```
/// use tonic::{Response, Status};
/// use futures::Stream;
/// use std::pin::Pin;
///
/// #[derive(Clone, PartialEq, ::prost::Message)]
/// pub struct ResponsePush {
///     #[prost(int32, tag = "1")]
///     pub code: i32,
/// }
///
/// // below code is to mimic a stream response from a GRPC service
/// let output = async_stream::try_stream! {
///     yield ResponsePush { code: 0 };
///     yield ResponsePush { code: 1 };
///     yield ResponsePush { code: 2 };
/// };
/// let response = Response::new(Box::pin(output) as tonic_mock::StreamResponseInner<ResponsePush>);
/// let rt = tokio::runtime::Runtime::new().unwrap();
///
/// // now we convert response to vec
/// let result: Vec<Result<ResponsePush, Status>> = rt.block_on(async { tonic_mock::stream_to_vec(response).await });
/// for (i, v) in result.iter().enumerate() {
///     assert!(v.is_ok());
///     assert_eq!(v.as_ref().unwrap().code, i as i32);
/// }
/// ```
pub async fn stream_to_vec<T>(response: StreamResponse<T>) -> Vec<Result<T, Status>>
where
    T: Message + Default + 'static,
{
    let mut result = Vec::new();
    let mut messages = response.into_inner();
    while let Some(v) = messages.next().await {
        result.push(v)
    }
    result
}

/// Convert a streaming response to a Vec with timeout support
///
/// This function is similar to `stream_to_vec` but adds a timeout for each message.
/// If a message is not received within the specified timeout, a `Status::deadline_exceeded` error
/// will be added to the result vector and processing will stop.
///
/// # Arguments
/// * `response` - The streaming response to process
/// * `timeout_duration` - The maximum time to wait for each message
///
/// # Returns
/// A vector of message results, potentially including a timeout error
///
/// # Example
/// ```
/// use tonic::{Response, Status};
/// use futures::Stream;
/// use std::{pin::Pin, time::Duration};
///
/// #[derive(Clone, PartialEq, ::prost::Message)]
/// pub struct ResponsePush {
///     #[prost(int32, tag = "1")]
///     pub code: i32,
/// }
///
/// // below code is to mimic a stream response from a GRPC service
/// let output = async_stream::try_stream! {
///     yield ResponsePush { code: 0 };
///     yield ResponsePush { code: 1 };
///     yield ResponsePush { code: 2 };
/// };
/// let response = Response::new(Box::pin(output) as tonic_mock::StreamResponseInner<ResponsePush>);
/// let rt = tokio::runtime::Runtime::new().unwrap();
///
/// // now we convert response to vec with a timeout
/// let result = rt.block_on(async {
///     tonic_mock::stream_to_vec_with_timeout(response, Duration::from_secs(1)).await
/// });
/// for (i, v) in result.iter().enumerate() {
///     if i < 3 {
///         assert!(v.is_ok());
///         assert_eq!(v.as_ref().unwrap().code, i as i32);
///     }
/// }
/// ```
pub async fn stream_to_vec_with_timeout<T>(
    response: StreamResponse<T>,
    timeout_duration: Duration,
) -> Vec<Result<T, Status>>
where
    T: Message + Default + 'static,
{
    let mut result = Vec::new();
    let mut messages = response.into_inner();
    loop {
        match timeout(timeout_duration, messages.next()).await {
            Ok(Some(v)) => result.push(v),
            Ok(None) => break, // Stream is done
            Err(_) => {
                // Timeout occurred
                result.push(Err(Status::deadline_exceeded(format!(
                    "Timeout waiting for message {}: exceeded {:?}",
                    result.len(),
                    timeout_duration
                ))));
                break;
            }
        }
    }
    result
}

/// A bidirectional streaming test context that allows controlled message exchange
///
/// This utility provides a powerful way to test bidirectional streaming interactions
/// for gRPC services. It offers a simple interface for sending client messages to a service
/// and receiving server responses in a controlled manner.
///
/// # Key Features
///
/// - **Simplified Testing**: Test bidirectional streaming without complex setup
/// - **Controlled Message Flow**: Send messages and receive responses one by one
/// - **Timeout Support**: Set timeouts for receiving responses to test timing behavior
/// - **Clean Teardown**: Properly complete streams when testing is finished
///
/// # Usage Patterns
///
/// This utility supports two main usage patterns:
///
/// 1. **Sequential Pattern**: Send all messages, call complete(), then get all responses
/// 2. **Interactive Pattern**: Send all messages, call complete(), then get responses one by one
///
/// # Important Usage Notes
///
/// - You **MUST** call `complete()` before trying to get any responses
/// - For proper operation, send all client messages before calling `complete()`
/// - After calling `complete()`, you cannot send more messages
///
/// # Example
///
/// ```no_run
/// use std::time::Duration;
/// use tonic::{Request, Response, Status, Streaming};
/// use tonic_mock::{BidirectionalStreamingTest, StreamResponseInner, test_utils::TestRequest, test_utils::TestResponse};
///
/// # async fn example() {
/// // Define a simple echo service for testing
/// async fn echo_service(
///     request: Request<Streaming<TestRequest>>
/// ) -> Result<Response<StreamResponseInner<TestResponse>>, Status> {
///     let mut stream = request.into_inner();
///     let response_stream = async_stream::try_stream! {
///         while let Some(msg) = stream.message().await? {
///             let id_str = String::from_utf8_lossy(&msg.id).to_string();
///             yield TestResponse::new(200, format!("Echo: {}", id_str));
///         }
///     };
///     Ok(Response::new(Box::pin(response_stream)))
/// }
///
/// // Pattern 1: Send all messages, then get all responses
/// let mut test = BidirectionalStreamingTest::new(echo_service);
/// test.send_client_message(TestRequest::new("msg1", "data1")).await;
/// test.send_client_message(TestRequest::new("msg2", "data2")).await;
/// test.complete().await;  // MUST call complete() before getting responses
///
/// let response1 = test.get_server_response().await;
/// let response2 = test.get_server_response().await;
///
/// // Pattern 2: Send all messages, then get responses one by one (interactive)
/// let mut test2 = BidirectionalStreamingTest::new(echo_service);
/// test2.send_client_message(TestRequest::new("msg1", "data1")).await;
/// test2.send_client_message(TestRequest::new("msg2", "data2")).await;
/// test2.complete().await;  // MUST call complete() before getting responses
///
/// // Now get responses one by one
/// let response1 = test2.get_server_response().await;
/// println!("Got first response: {:?}", response1);
///
/// let response2 = test2.get_server_response().await;
/// println!("Got second response: {:?}", response2);
/// # }
/// ```
pub struct BidirectionalStreamingTest<Req, Resp>
where
    Req: Message + Default + Send + 'static,
    Resp: Message + Default + Send + 'static,
{
    // Channel for sending client messages to the service
    client_tx: Option<tokio::sync::mpsc::Sender<Req>>,

    // Signal to indicate the client is done sending messages
    client_done_tx: Option<tokio::sync::oneshot::Sender<()>>,

    // Receiver for server responses
    server_rx: Option<tokio::sync::mpsc::Receiver<Result<Resp, Status>>>,

    // Flag to indicate if the test is completed
    completed: bool,
}

impl<Req, Resp> BidirectionalStreamingTest<Req, Resp>
where
    Req: Message + Default + Send + 'static,
    Resp: Message + Default + Send + 'static,
{
    /// Create a new bidirectional streaming test context with the specified service handler
    ///
    /// This method takes a service handler function that implements a bidirectional streaming
    /// gRPC service and creates a test context for it.
    ///
    /// # Arguments
    /// * `service_handler` - A function that handles the bidirectional streaming RPC.
    ///
    /// # Returns
    /// A new `BidirectionalStreamingTest` instance that you can use to interact with the service.
    pub fn new<F, Fut>(service_handler: F) -> Self
    where
        F: FnOnce(Request<Streaming<Req>>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<Response<StreamResponseInner<Resp>>, Status>>
            + Send
            + 'static,
    {
        // Create a channel for client messages
        let (client_tx, client_rx) = tokio::sync::mpsc::channel::<Req>(32);

        // Create a oneshot channel to signal when client is done sending
        let (client_done_tx, client_done_rx) = tokio::sync::oneshot::channel();

        // Create a channel for server responses
        let (server_tx, server_rx) = tokio::sync::mpsc::channel::<Result<Resp, Status>>(32);

        // Create a task to handle the service call
        tokio::spawn(async move {
            // Create the MockBody from the client_rx channel
            let body = MockBody::from_channel(client_rx);
            let decoder: ProstDecoder<Req> = ProstDecoder::new();
            let stream = Streaming::new_request(decoder, body, None, None);

            // Call the service with the request
            let request = Request::new(stream);
            match service_handler(request).await {
                Ok(response) => {
                    // Get the response stream
                    let mut response_stream = response.into_inner();

                    // Spawn a task to listen for the done signal
                    tokio::spawn(async move {
                        // Wait for done signal
                        let _ = client_done_rx.await;
                        // Once done, the task will exit and the channel will be closed
                    });

                    // Process all responses
                    while let Some(resp) = response_stream.next().await {
                        if server_tx.send(resp).await.is_err() {
                            // The receiver has been dropped, stop processing
                            break;
                        }
                    }
                }
                Err(status) => {
                    // Service returned an error, forward it
                    let _ = server_tx.send(Err(status)).await;
                }
            }

            // When the task ends, the server_tx will be dropped, signaling the end of responses
        });

        Self {
            client_tx: Some(client_tx),
            client_done_tx: Some(client_done_tx),
            server_rx: Some(server_rx),
            completed: false,
        }
    }

    /// Send a message from the client to the service
    ///
    /// This method allows you to send a single message from the client to the service
    /// under test. The message will be delivered to the service handler, which can then
    /// process it and potentially generate a response.
    ///
    /// # Arguments
    /// * `message` - The message to send to the service
    ///
    /// # Panics
    /// This method will panic if:
    /// - It is called after `complete()` has been called
    /// - The channel to the service is closed (which may indicate that the service has exited)
    pub async fn send_client_message(&mut self, message: Req) {
        if self.completed {
            panic!("Cannot send message after test has been completed");
        }

        match &self.client_tx {
            Some(tx) => {
                if tx.send(message).await.is_err() {
                    // The channel is closed, meaning the service has exited
                    panic!("Failed to send message to service: channel closed");
                }
            }
            None => {
                panic!("Cannot send message after test has been completed");
            }
        }
    }

    /// Get the next response from the service
    ///
    /// This method retrieves the next response from the service.
    ///
    /// # Returns
    /// The next response message or None if there are no more messages
    pub async fn get_server_response(&mut self) -> Option<Resp> {
        match &mut self.server_rx {
            Some(rx) => match rx.recv().await {
                Some(Ok(resp)) => Some(resp),
                Some(Err(status)) => {
                    eprintln!("Service returned error: {}", status);
                    None
                }
                None => None,
            },
            None => None,
        }
    }

    /// Get the next response with a timeout
    ///
    /// # Arguments
    /// * `timeout_duration` - Maximum time to wait for a response
    ///
    /// # Returns
    /// The next response message, None if there are no more messages, or an error if timeout occurs
    pub async fn get_server_response_with_timeout(
        &mut self,
        timeout_duration: Duration,
    ) -> Result<Option<Resp>, Status> {
        match &mut self.server_rx {
            Some(rx) => match timeout(timeout_duration, rx.recv()).await {
                Ok(Some(Ok(resp))) => Ok(Some(resp)),
                Ok(Some(Err(status))) => Err(status),
                Ok(None) => Ok(None),
                Err(_) => Err(Status::deadline_exceeded(format!(
                    "Timeout waiting for server response: exceeded {:?}",
                    timeout_duration
                ))),
            },
            None => Ok(None),
        }
    }

    /// Complete the bidirectional streaming test
    ///
    /// This signals that no more client messages will be sent. When this method is called,
    /// the client stream is closed, allowing the service to complete its processing.
    ///
    /// **IMPORTANT**: You must call this method before trying to get any responses.
    /// After calling this method, you cannot send more messages.
    pub async fn complete(&mut self) {
        if !self.completed {
            // Drop the client channel to signal no more messages
            self.client_tx = None;

            // Signal end of client stream
            if let Some(done_tx) = self.client_done_tx.take() {
                let _ = done_tx.send(());
            }

            self.completed = true;
        }
    }

    /// Explicitly drop this test instance and clean up resources
    ///
    /// This will close all channels and signal completion.
    /// It's automatically called when the test instance is dropped.
    pub fn dispose(&mut self) {
        // Drop all channels
        self.client_tx = None;
        self.client_done_tx = None;
        self.server_rx = None;
        self.completed = true;
    }
}

impl<Req, Resp> Drop for BidirectionalStreamingTest<Req, Resp>
where
    Req: Message + Default + Send + 'static,
    Resp: Message + Default + Send + 'static,
{
    fn drop(&mut self) {
        self.dispose();
    }
}
