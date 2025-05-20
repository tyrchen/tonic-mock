# API Expansion Plan: Phase 2

## Overview

This phase focuses on extending the core API with new capabilities while maintaining backward compatibility and API simplicity. All new features will be implemented as separate functions to avoid disrupting existing code.

## 1. Error Injection Functionality

### 1.1 New Types and Implementation

Create a new module `src/error.rs`:

```rust
use std::sync::{Arc, Mutex};
use tonic::Status;

/// Defines various patterns for injecting errors into mock streams
#[derive(Clone, Debug)]
pub enum ErrorPattern {
    /// Inject error after specified number of messages
    AfterMessages(usize),

    /// Inject error randomly with given probability (0.0-1.0)
    Random(f32),

    /// Alternate between successful messages and errors
    Alternating,

    /// Custom error pattern using a closure
    Custom(Arc<Mutex<dyn Fn(usize) -> bool + Send>>),
}

/// Error to inject into the stream
#[derive(Clone, Debug)]
pub struct ErrorInjection {
    /// The error pattern to use
    pub pattern: ErrorPattern,

    /// The status code and message to use for errors
    pub status: Status,
}

impl ErrorInjection {
    /// Create a new error injection with the given pattern and status
    pub fn new(pattern: ErrorPattern, status: Status) -> Self {
        Self { pattern, status }
    }

    /// Creates a new error injection that occurs after a specific number of messages
    pub fn after_messages(count: usize, status: Status) -> Self {
        Self {
            pattern: ErrorPattern::AfterMessages(count),
            status,
        }
    }

    /// Creates a new error injection with random failures
    pub fn random(probability: f32, status: Status) -> Self {
        Self {
            pattern: ErrorPattern::Random(probability),
            status,
        }
    }

    /// Creates a new error injection that alternates between success and failure
    pub fn alternating(status: Status) -> Self {
        Self {
            pattern: ErrorPattern::Alternating,
            status,
        }
    }

    /// Creates a new error injection with a custom pattern
    pub fn custom<F>(f: F, status: Status) -> Self
    where
        F: Fn(usize) -> bool + Send + 'static,
    {
        Self {
            pattern: ErrorPattern::Custom(Arc::new(Mutex::new(f))),
            status,
        }
    }

    /// Determines if an error should be injected at the given index
    pub fn should_inject(&self, index: usize) -> bool {
        match &self.pattern {
            ErrorPattern::AfterMessages(count) => index >= *count,
            ErrorPattern::Random(probability) => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                rng.gen::<f32>() < *probability
            }
            ErrorPattern::Alternating => index % 2 == 1,
            ErrorPattern::Custom(f) => {
                if let Ok(guard) = f.lock() {
                    guard(index)
                } else {
                    false
                }
            }
        }
    }
}
```

### 1.2 Stream Modifications in `src/mock.rs`

Add error injection to `MockBody`:

```rust
use crate::error::ErrorInjection;

pub struct MockBodyWithErrors {
    data: VecDeque<Bytes>,
    error_injection: Option<ErrorInjection>,
    current_index: usize,
}

impl MockBodyWithErrors {
    pub fn new(data: Vec<impl Message>, error_injection: Option<ErrorInjection>) -> Self {
        let mut queue: VecDeque<Bytes> = VecDeque::with_capacity(16);
        for msg in data {
            let buf = Self::encode(msg);
            queue.push_back(buf);
        }

        MockBodyWithErrors {
            data: queue,
            error_injection,
            current_index: 0
        }
    }

    // Rest of implementation...
}

impl Body for MockBodyWithErrors {
    type Data = Bytes;
    type Error = Status;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        // Implementation with error injection logic
        if let Some(ref error_injection) = self.error_injection {
            if error_injection.should_inject(self.current_index) {
                self.current_index += 1;
                return Poll::Ready(Some(Err(error_injection.status.clone())));
            }
        }

        // Existing implementation...
        self.current_index += 1;
        // ...
    }
}
```

### 1.3 Public API in `src/lib.rs`

Add public function for error injection:

```rust
/// Generate streaming request with error injection for GRPC testing
///
/// This function extends `streaming_request` with the ability to inject errors
/// according to specified patterns. This is useful for testing error handling
/// in your gRPC service implementation.
///
/// # Example
///
/// ```
/// use tonic::{Status, Code};
/// use tonic_mock::{streaming_request_with_errors, ErrorInjection, ErrorPattern};
///
/// // Define your message type...
///
/// let messages = vec![/* your messages */];
/// let error = ErrorInjection::after_messages(
///     2,
///     Status::new(Code::Internal, "Simulated error")
/// );
///
/// let stream = streaming_request_with_errors(messages, error);
/// ```
pub fn streaming_request_with_errors<T>(
    messages: Vec<T>,
    error_injection: ErrorInjection,
) -> Request<Streaming<T>>
where
    T: Message + Default + 'static,
{
    let body = MockBodyWithErrors::new(messages, Some(error_injection));
    let decoder: ProstDecoder<T> = ProstDecoder::new();
    let stream = Streaming::new_request(decoder, body, None, None);

    Request::new(stream)
}
```

## 2. Timeout Configuration

### 2.1 New Module `src/timeout.rs`

```rust
use std::time::Duration;
use tonic::{Status, Code};

/// Configuration for timeout behavior in streaming operations
#[derive(Clone, Debug)]
pub struct TimeoutConfig {
    /// Duration after which the stream should time out
    pub duration: Duration,

    /// Status to return when timeout occurs
    pub status: Status,
}

impl TimeoutConfig {
    /// Create a new timeout configuration
    pub fn new(duration: Duration, status: Status) -> Self {
        Self { duration, status }
    }

    /// Create a new timeout configuration with default error message
    pub fn with_duration(duration: Duration) -> Self {
        Self {
            duration,
            status: Status::new(
                Code::DeadlineExceeded,
                format!("Stream timed out after {:?}", duration),
            ),
        }
    }
}
```

### 2.2 Integration with Streaming Request

Modify `MockBody` to support timeouts or create a specialized version:

```rust
pub struct MockBodyWithTimeout {
    data: VecDeque<Bytes>,
    timeout: Option<TimeoutConfig>,
    start_time: std::time::Instant,
}

impl MockBodyWithTimeout {
    pub fn new(data: Vec<impl Message>, timeout: Option<TimeoutConfig>) -> Self {
        let mut queue: VecDeque<Bytes> = VecDeque::with_capacity(16);
        for msg in data {
            let buf = Self::encode(msg);
            queue.push_back(buf);
        }

        MockBodyWithTimeout {
            data: queue,
            timeout,
            start_time: std::time::Instant::now(),
        }
    }

    // Rest of implementation...
}

impl Body for MockBodyWithTimeout {
    type Data = Bytes;
    type Error = Status;

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        // Check for timeout
        if let Some(ref timeout) = self.timeout {
            if self.start_time.elapsed() >= timeout.duration {
                return Poll::Ready(Some(Err(timeout.status.clone())));
            }
        }

        // Existing implementation...
    }
}
```

### 2.3 Public API in `src/lib.rs`

```rust
/// Generate streaming request with timeout for GRPC testing
///
/// This function allows you to simulate timeouts in your tests, which is
/// useful for verifying your service's timeout handling logic.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use tonic_mock::{streaming_request_with_timeout, TimeoutConfig};
///
/// // Define your message type...
///
/// let messages = vec![/* your messages */];
/// let timeout = TimeoutConfig::with_duration(Duration::from_millis(200));
///
/// let stream = streaming_request_with_timeout(messages, timeout);
/// ```
pub fn streaming_request_with_timeout<T>(
    messages: Vec<T>,
    timeout: TimeoutConfig,
) -> Request<Streaming<T>>
where
    T: Message + Default + 'static,
{
    let body = MockBodyWithTimeout::new(messages, Some(timeout));
    let decoder: ProstDecoder<T> = ProstDecoder::new();
    let stream = Streaming::new_request(decoder, body, None, None);

    Request::new(stream)
}
```

## 3. Interceptor Support

### 3.1 New Module `src/interceptor.rs`

```rust
use tonic::service::Interceptor;
use tonic::{Request, Status};

/// Wrapper for testing interceptors
#[derive(Debug, Clone)]
pub struct InterceptorTester<I: Interceptor> {
    interceptor: I,
}

impl<I: Interceptor> InterceptorTester<I> {
    /// Create a new interceptor tester
    pub fn new(interceptor: I) -> Self {
        Self { interceptor }
    }

    /// Apply the interceptor to a request and return the result
    pub fn apply<T>(&self, mut request: Request<T>) -> Result<Request<T>, Status> {
        self.interceptor.call(request)
    }
}
```

### 3.2 Public API in `src/lib.rs`

```rust
/// Generate streaming request with interceptor for GRPC testing
///
/// This function allows testing how your interceptors interact with
/// streaming requests.
///
/// # Example
///
/// ```
/// use tonic::{Request, service::Interceptor};
/// use tonic_mock::streaming_request_with_interceptor;
///
/// // Define your interceptor...
///
/// struct MyInterceptor;
///
/// impl Interceptor for MyInterceptor {
///     fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
///         // Your interceptor logic
///         Ok(request)
///     }
/// }
///
/// let messages = vec![/* your messages */];
/// let interceptor = MyInterceptor;
///
/// let stream = streaming_request_with_interceptor(messages, interceptor);
/// ```
pub fn streaming_request_with_interceptor<T, I>(
    messages: Vec<T>,
    interceptor: I,
) -> Result<Request<Streaming<T>>, Status>
where
    T: Message + Default + 'static,
    I: Interceptor,
{
    let request = streaming_request(messages);
    let tester = InterceptorTester::new(interceptor);
    tester.apply(request)
}
```

## 4. Bidirectional Streaming Support

### 4.1 New Module `src/bidirectional.rs`

```rust
use crate::{MockBody, ProstDecoder, StreamResponse, StreamResponseInner};
use futures::{Stream, StreamExt};
use prost::Message;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tonic::{Request, Response, Status, Streaming};

/// Handler for processing requests in a bidirectional stream
pub struct BidirectionalStreamHandler<Req, Resp, F>
where
    Req: Message + Default + 'static,
    Resp: Message + Default + Clone + 'static,
    F: Fn(Req) -> Result<Resp, Status> + Send + 'static,
{
    handler: Arc<Mutex<F>>,
    _req_type: std::marker::PhantomData<Req>,
    _resp_type: std::marker::PhantomData<Resp>,
}

impl<Req, Resp, F> BidirectionalStreamHandler<Req, Resp, F>
where
    Req: Message + Default + 'static,
    Resp: Message + Default + Clone + 'static,
    F: Fn(Req) -> Result<Resp, Status> + Send + 'static,
{
    /// Create a new bidirectional stream handler
    pub fn new(handler: F) -> Self {
        Self {
            handler: Arc::new(Mutex::new(handler)),
            _req_type: std::marker::PhantomData,
            _resp_type: std::marker::PhantomData,
        }
    }

    /// Process a request stream and produce a response stream
    pub fn process(
        &self,
        request: Request<Streaming<Req>>,
    ) -> StreamResponse<Resp> {
        let handler = self.handler.clone();

        // Create stream that processes each request using the handler
        let stream = Box::pin(async_stream::try_stream! {
            let mut in_stream = request.into_inner();

            while let Some(req) = in_stream.next().await {
                let req = req?;

                if let Ok(handler) = handler.lock() {
                    let resp = handler(req)?;
                    yield resp;
                } else {
                    yield Err(Status::internal("Failed to lock handler"))?;
                }
            }
        }) as StreamResponseInner<Resp>;

        Response::new(stream)
    }
}
```

### 4.2 Public API in `src/lib.rs`

```rust
/// Test bidirectional streaming with a custom handler function
///
/// This function allows testing bidirectional streaming by providing
/// a handler function that processes each request message and produces
/// a response. This simulates a bidirectional streaming service.
///
/// # Example
///
/// ```
/// use tonic::Status;
/// use tonic_mock::bidirectional_streaming_test;
///
/// // Define your request and response types...
///
/// let requests = vec![/* your request messages */];
///
/// // Define a handler function
/// let handler = |req: MyRequest| -> Result<MyResponse, Status> {
///     // Process request and return response
///     Ok(MyResponse { /* fields */ })
/// };
///
/// let (req_stream, resp_stream) = bidirectional_streaming_test(requests, handler);
/// ```
pub fn bidirectional_streaming_test<Req, Resp, F>(
    requests: Vec<Req>,
    handler: F,
) -> (Request<Streaming<Req>>, StreamResponse<Resp>)
where
    Req: Message + Default + 'static,
    Resp: Message + Default + Clone + 'static,
    F: Fn(Req) -> Result<Resp, Status> + Send + 'static,
{
    let request = streaming_request(requests);
    let stream_handler = BidirectionalStreamHandler::new(handler);
    let response = stream_handler.process(request.clone());

    (request, response)
}
```

## 5. Builder Pattern for Complex Configuration

### 5.1 New Module `src/builder.rs`

```rust
use crate::{
    error::ErrorInjection,
    timeout::TimeoutConfig,
    MockBody, ProstDecoder,
};
use prost::Message;
use tonic::{Request, Streaming, service::Interceptor, Status};

/// Builder for creating complex stream configurations
pub struct StreamRequestBuilder<T>
where
    T: Message + Default + 'static,
{
    messages: Vec<T>,
    error_injection: Option<ErrorInjection>,
    timeout: Option<TimeoutConfig>,
    metadata: Option<tonic::metadata::MetadataMap>,
}

impl<T> StreamRequestBuilder<T>
where
    T: Message + Default + 'static,
{
    /// Create a new stream request builder with the given messages
    pub fn new(messages: Vec<T>) -> Self {
        Self {
            messages,
            error_injection: None,
            timeout: None,
            metadata: None,
        }
    }

    /// Add error injection to the stream
    pub fn with_error(mut self, error_injection: ErrorInjection) -> Self {
        self.error_injection = Some(error_injection);
        self
    }

    /// Add timeout to the stream
    pub fn with_timeout(mut self, timeout: TimeoutConfig) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add metadata to the request
    pub fn with_metadata(mut self, metadata: tonic::metadata::MetadataMap) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Build the request
    pub fn build(self) -> Request<Streaming<T>> {
        // Implementation combines all configurations
        // and creates the appropriate stream
        // ...
    }

    /// Apply an interceptor to the request
    pub fn apply_interceptor<I: Interceptor>(
        self,
        interceptor: I,
    ) -> Result<Request<Streaming<T>>, Status> {
        let request = self.build();
        let mut interceptor = interceptor;
        interceptor.call(request)
    }
}
```

### 5.2 Public API in `src/lib.rs`

```rust
/// Create a builder for complex stream configurations
///
/// This function provides a builder pattern for creating streams with
/// complex configurations including error injection, timeouts, and metadata.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use tonic::{Status, Code};
/// use tonic_mock::{
///     stream_request_builder,
///     error::ErrorInjection,
///     timeout::TimeoutConfig,
/// };
///
/// // Define your message type...
///
/// let messages = vec![/* your messages */];
///
/// let request = stream_request_builder(messages)
///     .with_error(ErrorInjection::after_messages(
///         2,
///         Status::new(Code::Internal, "Simulated error")
///     ))
///     .with_timeout(TimeoutConfig::with_duration(Duration::from_secs(5)))
///     .build();
/// ```
pub fn stream_request_builder<T>(messages: Vec<T>) -> StreamRequestBuilder<T>
where
    T: Message + Default + 'static,
{
    StreamRequestBuilder::new(messages)
}
```

## Implementation Timeline

1. Error Injection Implementation: 3 days
   - Create error.rs module
   - Implement MockBodyWithErrors
   - Add public API function
   - Add tests

2. Timeout Configuration: 2 days
   - Create timeout.rs module
   - Implement MockBodyWithTimeout
   - Add public API function
   - Add tests

3. Interceptor Support: 2 days
   - Create interceptor.rs module
   - Implement InterceptorTester
   - Add public API function
   - Add tests

4. Bidirectional Streaming: 3 days
   - Create bidirectional.rs module
   - Implement BidirectionalStreamHandler
   - Add public API function
   - Add tests

5. Builder Pattern: 2 days
   - Create builder.rs module
   - Implement StreamRequestBuilder
   - Add public API function
   - Add tests

Total estimated time: 12 days

## Testing Strategy

For each new feature:

1. Unit tests to verify expected behavior
2. Tests for edge cases and error conditions
3. Integration tests with sample gRPC service
4. Documentation examples that serve as tests

## Documentation Strategy

1. Create detailed documentation for each new feature
2. Add examples demonstrating common use cases
3. Update README with new feature information
4. Create tutorials for complex testing scenarios
