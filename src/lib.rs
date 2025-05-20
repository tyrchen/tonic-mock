use futures::{Stream, StreamExt};
use prost::Message;
use std::{
    fmt::Debug,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};
use tokio::time::timeout;
use tonic::{Request, Response, Status, Streaming};

mod mock;

pub use mock::{MockBody, ProstDecoder};

#[cfg(feature = "test-utils")]
pub mod test_utils;

#[cfg(feature = "test-utils")]
pub use test_utils::*;

/// Type alias for the inner stream of a streaming response
pub type StreamResponseInner<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + Sync>>;
/// Type alias for a streaming response
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
    T: Message + Default + 'static,
{
    let body = MockBody::new(messages);
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
/// A Request<Streaming<T>> with the interceptor applied
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
/// A Request<T> with the interceptor applied
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
/// This utility provides a way to test bidirectional streaming interactions
/// by offering methods to send client messages and receive server responses
/// in a controlled manner.
///
/// # Usage
/// ```ignore
/// use futures::Stream;
/// use prost::Message;
/// use std::pin::Pin;
/// use tokio::runtime::Runtime;
/// use tonic::{Request, Response, Status, Streaming};
/// use tonic_mock::{BidirectionalStreamingTest, test_utils::TestRequest, test_utils::TestResponse};
///
/// // Define your service method (a simplified version for the example)
/// async fn bidirectional_service(
///     request: Request<Streaming<TestRequest>>
/// ) -> Result<Response<Pin<Box<dyn Stream<Item = Result<TestResponse, Status>> + Send + Sync + 'static>>>, Status> {
///     // Real implementation will process the request stream
///     // For this example, we'll create a simple echo service
///     let mut in_stream = request.into_inner();
///
///     let out_stream = async_stream::try_stream! {
///         while let Some(message) = in_stream.message().await? {
///             let response = TestResponse::new(
///                 123,
///                 format!("Echo: {:?}", message.id)
///             );
///             yield response;
///         }
///     };
///
///     Ok(Response::new(Box::pin(out_stream)))
/// }
///
/// // Now test the service
/// let rt = Runtime::new().unwrap();
/// rt.block_on(async {
///     // Create the test context
///     let mut test = BidirectionalStreamingTest::<TestRequest, TestResponse>::new(bidirectional_service);
///
///     // Send a message from client to server
///     test.send_client_message(TestRequest::new("test1", "data1")).await;
///
///     // Get the server's response
///     let response = test.get_server_response().await.unwrap();
///     assert_eq!(response.code, 123);
///
///     // Send another message
///     test.send_client_message(TestRequest::new("test2", "data2")).await;
///
///     // Get the response
///     let response = test.get_server_response().await.unwrap();
///     assert_eq!(response.code, 123);
///
///     // Complete the test
///     test.complete().await;
/// });
/// ```
pub struct BidirectionalStreamingTest<Req, Resp>
where
    Req: Message + Default + Send + 'static,
    Resp: Message + Default + Send + 'static,
{
    client_messages: Arc<Mutex<Vec<Option<Req>>>>,
    server_responses: Option<StreamResponseInner<Resp>>,
    completed: bool,
}

impl<Req, Resp> BidirectionalStreamingTest<Req, Resp>
where
    Req: Message + Default + Send + 'static,
    Resp: Message + Default + Send + 'static,
{
    /// Create a new bidirectional streaming test context with the specified service handler
    ///
    /// # Arguments
    /// * `service_handler` - A function that handles the bidirectional streaming RPC
    ///
    /// # Returns
    /// A new `BidirectionalStreamingTest` instance
    pub fn new<F, Fut>(service_handler: F) -> Self
    where
        F: FnOnce(Request<Streaming<Req>>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<Response<StreamResponseInner<Resp>>, Status>>
            + Send
            + 'static,
    {
        // Create the shared message queue
        let client_messages = Arc::new(Mutex::new(Vec::<Option<Req>>::new()));

        // Create an empty streaming request
        let empty_req_vec: Vec<Req> = Vec::new();
        let streaming_req = streaming_request(empty_req_vec);

        // Start the service handler
        let rt = tokio::runtime::Handle::current();
        let server_fut = rt.spawn(async move {
            // Process the streaming request
            match service_handler(streaming_req).await {
                Ok(response) => Some(response.into_inner()),
                Err(_) => None,
            }
        });

        // Wait for the service to initialize
        let server_responses = match rt.block_on(server_fut) {
            Ok(Some(responses)) => Some(responses),
            _ => None,
        };

        Self {
            client_messages,
            server_responses,
            completed: false,
        }
    }

    /// Send a message from the client to the service
    ///
    /// # Arguments
    /// * `message` - The message to send
    pub async fn send_client_message(&mut self, message: Req) {
        if self.completed {
            panic!("Cannot send message after test has been completed");
        }

        let mut messages = self.client_messages.lock().unwrap();
        messages.push(Some(message));
    }

    /// Get the next response from the service
    ///
    /// # Returns
    /// The next response message or None if there are no more messages
    pub async fn get_server_response(&mut self) -> Option<Resp> {
        if self.completed {
            return None;
        }

        if let Some(ref mut responses) = self.server_responses {
            match responses.next().await {
                Some(Ok(resp)) => Some(resp),
                Some(Err(_)) => None,
                None => None,
            }
        } else {
            None
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
        if self.completed {
            return Ok(None);
        }

        if let Some(ref mut responses) = self.server_responses {
            match timeout(timeout_duration, responses.next()).await {
                Ok(Some(Ok(resp))) => Ok(Some(resp)),
                Ok(Some(Err(status))) => Err(status),
                Ok(None) => Ok(None),
                Err(_) => Err(Status::deadline_exceeded(format!(
                    "Timeout waiting for server response: exceeded {:?}",
                    timeout_duration
                ))),
            }
        } else {
            Ok(None)
        }
    }

    /// Complete the bidirectional streaming test
    ///
    /// This signals that no more client messages will be sent
    pub async fn complete(&mut self) {
        if !self.completed {
            let mut messages = self.client_messages.lock().unwrap();
            messages.push(None); // Signal end of client stream
            self.completed = true;
        }
    }
}

/// A mock stream for client messages in a bidirectional test
struct MockClientStream<T> {
    messages: Arc<Mutex<Vec<Option<T>>>>,
}

impl<T> Stream for MockClientStream<T>
where
    T: Message + Default + Send + 'static,
{
    type Item = Result<T, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut messages = self.messages.lock().unwrap();

        if messages.is_empty() {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        let message = messages.remove(0);
        match message {
            Some(msg) => Poll::Ready(Some(Ok(msg))),
            None => Poll::Ready(None), // End of stream
        }
    }
}
