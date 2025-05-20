# Documentation Plan: Phase 3

## Overview

This phase focuses on significantly enhancing the documentation of tonic-mock to make it more accessible, comprehensive, and useful for developers. Good documentation is critical for testing libraries, as it helps users understand how to properly test their applications and avoid common pitfalls.

## 1. README Enhancement

### 1.1 Improved Structure

Restructure the README.md to make it more comprehensive:

```markdown
# tonic-mock

[![Crates.io](https://img.shields.io/crates/v/tonic-mock.svg)](https://crates.io/crates/tonic-mock)
[![Documentation](https://docs.rs/tonic-mock/badge.svg)](https://docs.rs/tonic-mock)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE.md)
[![Build Status](https://github.com/tyrchen/tonic-mock/workflows/CI/badge.svg)](https://github.com/tyrchen/tonic-mock/actions)

Test utilities for easy mocking of tonic streaming interfaces.

## Features

- 🔄 **Mock Streaming Requests**: Easily create mock gRPC streaming requests
- ✅ **Process Responses**: Validate streaming responses with simple callbacks
- 🛠️ **Error Injection**: Test error handling by injecting errors into streams
- ⏱️ **Timeout Simulation**: Test timeout handling with configurable timeouts
- 🔌 **Interceptor Testing**: Test your tonic interceptors with streaming interfaces
- 🔄 **Bidirectional Testing**: Test bidirectional streaming with request handlers

## Installation

```toml
[dependencies]
tonic-mock = "0.4.0"
```

## Quick Start

Testing a simple streaming RPC:

```rust
#[tokio::test]
async fn service_push_works() -> anyhow::Result<()> {
    // Create mock request data
    let mut events: Vec<RequestPush> = Vec::with_capacity(3);
    for i in 0..3 {
        events.push(RequestPush::new(
            Bytes::from(i.to_string()),
            Bytes::from("a".repeat(10))
        ));
    }

    // Create streaming request
    let req = tonic_mock::streaming_request(events);

    // Call service
    let res = server.push(req).await?;

    // Process and validate response
    tonic_mock::process_streaming_response(res, |msg, i| {
        assert!(msg.is_ok());
        assert_eq!(msg.as_ref().unwrap().code, i as i32);
    })
    .await;

    Ok(())
}
```

## Usage Guide

### Basic Usage

- [Creating Streaming Requests](#creating-streaming-requests)
- [Processing Streaming Responses](#processing-streaming-responses)
- [Converting Streams to Vectors](#converting-streams-to-vectors)

### Advanced Features

- [Error Injection](#error-injection)
- [Timeout Configuration](#timeout-configuration)
- [Interceptor Testing](#interceptor-testing)
- [Bidirectional Streaming](#bidirectional-streaming)
- [Builder Pattern](#builder-pattern)

### Examples

- [Simple gRPC Service Testing](#simple-grpc-service-testing)
- [Error Handling Testing](#error-handling-testing)
- [Complex Streaming Scenarios](#complex-streaming-scenarios)

## Detailed Documentation

### Creating Streaming Requests

...

### Processing Streaming Responses

...

### Converting Streams to Vectors

...

### Error Injection

...

### Timeout Configuration

...

### Interceptor Testing

...

### Bidirectional Streaming

...

### Builder Pattern

...

## Examples

### Simple gRPC Service Testing

...

### Error Handling Testing

...

### Complex Streaming Scenarios

...

## FAQ

...

## Contributing

...

## License

`tonic-mock` is distributed under the terms of MIT.

See [LICENSE](LICENSE.md) for details.

Copyright 2021 Tyr Chen
```

## 2. API Documentation Enhancement

### 2.1 Improved Crate-Level Documentation

Update `src/lib.rs` with comprehensive crate-level documentation:

```rust
//! # tonic-mock
//!
//! `tonic-mock` provides testing utilities for [tonic](https://github.com/hyperium/tonic),
//! a gRPC implementation for Rust. It focuses on making it easier to test streaming interfaces,
//! which can be challenging to mock and validate in tests.
//!
//! ## Core Features
//!
//! - **Mock Streaming Requests**: Create test streams from vectors of messages
//! - **Process Responses**: Process and validate streaming responses
//! - **Convert Streams**: Convert streams to vectors for easier testing
//! - **Error Injection**: Test error handling by injecting errors into streams
//! - **Timeout Simulation**: Test timeout handling with configurable timeouts
//! - **Interceptor Testing**: Test your tonic interceptors with streaming
//! - **Bidirectional Streaming**: Test bidirectional streaming interfaces
//!
//! ## Basic Usage
//!
//! ```rust
//! use tonic_mock::{streaming_request, process_streaming_response};
//!
//! #[tokio::test]
//! async fn test_my_streaming_service() {
//!     // Create test messages
//!     let messages = vec![
//!         // Your protocol buffer messages here
//!     ];
//!
//!     // Create a streaming request
//!     let request = streaming_request(messages);
//!
//!     // Call your service
//!     let response = my_service.streaming_method(request).await.unwrap();
//!
//!     // Process and validate the response
//!     process_streaming_response(response, |msg, idx| {
//!         // Verify each message
//!     }).await;
//! }
//! ```
//!
//! ## Advanced Features
//!
//! ### Error Injection
//!
//! ```rust
//! use tonic::{Status, Code};
//! use tonic_mock::{streaming_request_with_errors, ErrorInjection};
//!
//! let error = ErrorInjection::after_messages(
//!     2,
//!     Status::new(Code::Internal, "Simulated error")
//! );
//!
//! let request = streaming_request_with_errors(messages, error);
//! ```
//!
//! ### Timeout Configuration
//!
//! ```rust
//! use std::time::Duration;
//! use tonic_mock::{streaming_request_with_timeout, TimeoutConfig};
//!
//! let timeout = TimeoutConfig::with_duration(Duration::from_millis(500));
//! let request = streaming_request_with_timeout(messages, timeout);
//! ```
//!
//! ### Bidirectional Streaming
//!
//! ```rust
//! use tonic_mock::bidirectional_streaming_test;
//!
//! let handler = |req: MyRequest| -> Result<MyResponse, Status> {
//!     // Process request and return response
//!     Ok(MyResponse { /* fields */ })
//! };
//!
//! let (req_stream, resp_stream) = bidirectional_streaming_test(messages, handler);
//! ```
```

### 2.2 Enhanced Function Documentation

Improve documentation for all existing and new functions with more detailed examples, parameter explanations, and use cases.

Example for `streaming_request`:

```rust
/// Generate streaming request for GRPC testing
///
/// When testing streaming RPC implemented with tonic, building the streaming request
/// can be complex. This function simplifies the process by taking a vector of protocol
/// buffer messages and creating a properly configured streaming request.
///
/// # Arguments
///
/// * `messages` - A vector of protocol buffer messages to include in the stream
///
/// # Returns
///
/// A `Request<Streaming<T>>` that can be passed to a tonic service method
///
/// # Type Parameters
///
/// * `T` - The protocol buffer message type, which must implement `Message + Default + 'static`
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use tonic_mock::streaming_request;
/// use bytes::Bytes;
/// use prost::Message;
///
/// // Define a message type (typically generated from protobuf)
/// #[derive(Clone, PartialEq, Message)]
/// pub struct Event {
///     #[prost(bytes = "bytes", tag = "1")]
///     pub id: Bytes,
///     #[prost(bytes = "bytes", tag = "2")]
///     pub data: Bytes,
/// }
///
/// // Create test messages
/// let mut events = Vec::new();
/// for i in 0..3 {
///     events.push(Event {
///         id: Bytes::from(i.to_string()),
///         data: Bytes::from("test data"),
///     });
/// }
///
/// // Create a streaming request
/// let request = streaming_request(events);
///
/// // Now you can pass this request to your service method
/// // let response = my_service.streaming_method(request).await?;
/// ```
///
/// With empty message list:
///
/// ```
/// use tonic_mock::streaming_request;
/// use bytes::Bytes;
/// use prost::Message;
///
/// #[derive(Clone, PartialEq, Message)]
/// pub struct Event {
///     #[prost(bytes = "bytes", tag = "1")]
///     pub id: Bytes,
///     #[prost(bytes = "bytes", tag = "2")]
///     pub data: Bytes,
/// }
///
/// // Create an empty request stream
/// let empty_request = streaming_request::<Event>(Vec::new());
/// ```
pub fn streaming_request<T>(messages: Vec<T>) -> Request<Streaming<T>>
where
    T: Message + Default + 'static,
{
    // Implementation...
}
```

## 3. Examples Directory

Create an `examples` directory with comprehensive examples:

```
examples/
├── README.md                     // Overview of examples
├── basic/
│   ├── Cargo.toml                // Basic example dependencies
│   └── src/
│       └── main.rs               // Basic streaming example
├── error_handling/
│   ├── Cargo.toml                // Error handling example dependencies
│   └── src/
│       └── main.rs               // Error injection examples
├── timeout/
│   ├── Cargo.toml                // Timeout example dependencies
│   └── src/
│       └── main.rs               // Timeout simulation examples
├── interceptors/
│   ├── Cargo.toml                // Interceptor example dependencies
│   └── src/
│       └── main.rs               // Interceptor testing examples
└── bidirectional/
    ├── Cargo.toml                // Bidirectional example dependencies
    └── src/
        └── main.rs               // Bidirectional streaming examples
```

### 3.1 Basic Example

```rust
// examples/basic/src/main.rs
use bytes::Bytes;
use prost::Message;
use tonic::{Request, Response, Status};
use tonic_mock::{process_streaming_response, streaming_request, stream_to_vec};

// Define protocol buffer message types (normally generated)
#[derive(Clone, PartialEq, Message)]
pub struct RequestMessage {
    #[prost(bytes = "bytes", tag = "1")]
    pub id: Bytes,
    #[prost(bytes = "bytes", tag = "2")]
    pub data: Bytes,
}

#[derive(Clone, PartialEq, Message)]
pub struct ResponseMessage {
    #[prost(int32, tag = "1")]
    pub code: i32,
    #[prost(string, tag = "2")]
    pub message: String,
}

// Mock service
struct MockService;

impl MockService {
    async fn process_stream(
        &self,
        request: Request<tonic::Streaming<RequestMessage>>,
    ) -> Result<Response<tonic_mock::StreamResponseInner<ResponseMessage>>, Status> {
        // Create response stream from request
        let output = async_stream::try_stream! {
            let mut in_stream = request.into_inner();
            let mut count = 0;

            while let Some(message) = in_stream.message().await? {
                yield ResponseMessage {
                    code: count,
                    message: format!("Processed message with ID: {:?}", message.id),
                };
                count += 1;
            }
        };

        Ok(Response::new(Box::pin(output) as tonic_mock::StreamResponseInner<ResponseMessage>))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Basic tonic-mock example");

    // Create test messages
    let mut requests = Vec::new();
    for i in 0..3 {
        requests.push(RequestMessage {
            id: Bytes::from(i.to_string()),
            data: Bytes::from(format!("Test data {}", i)),
        });
    }

    // Create service
    let service = MockService;

    // Example 1: Basic streaming request and process response
    {
        println!("\nExample 1: Basic streaming");

        // Create streaming request
        let request = streaming_request(requests.clone());

        // Call service
        let response = service.process_stream(request).await?;

        // Process responses with callback
        process_streaming_response(response, |msg, idx| {
            if let Ok(msg) = &msg {
                println!("Response {}: code={}, message='{}'", idx, msg.code, msg.message);
            } else {
                println!("Response {}: Error: {:?}", idx, msg.err());
            }
        }).await;
    }

    // Example 2: Convert stream to vector
    {
        println!("\nExample 2: Stream to vector");

        // Create streaming request
        let request = streaming_request(requests.clone());

        // Call service
        let response = service.process_stream(request).await?;

        // Convert stream to vector
        let results = stream_to_vec(response).await;

        // Process vector
        for (i, result) in results.iter().enumerate() {
            if let Ok(msg) = result {
                println!("Vector item {}: code={}, message='{}'", i, msg.code, msg.message);
            } else {
                println!("Vector item {}: Error: {:?}", i, result.as_ref().err());
            }
        }
    }

    Ok(())
}
```

### 3.2 Error Handling Example

```rust
// examples/error_handling/src/main.rs
use bytes::Bytes;
use prost::Message;
use tonic::{Code, Request, Response, Status};
use tonic_mock::{
    error::ErrorInjection, error::ErrorPattern, process_streaming_response,
    streaming_request_with_errors, StreamResponseInner,
};

// Define protocol buffer message types
#[derive(Clone, PartialEq, Message)]
pub struct RequestMessage {
    #[prost(bytes = "bytes", tag = "1")]
    pub id: Bytes,
    #[prost(bytes = "bytes", tag = "2")]
    pub data: Bytes,
}

#[derive(Clone, PartialEq, Message)]
pub struct ResponseMessage {
    #[prost(int32, tag = "1")]
    pub code: i32,
    #[prost(string, tag = "2")]
    pub message: String,
}

// Mock service that should handle errors
struct ErrorHandlingService;

impl ErrorHandlingService {
    async fn process_stream(
        &self,
        request: Request<tonic::Streaming<RequestMessage>>,
    ) -> Result<Response<StreamResponseInner<ResponseMessage>>, Status> {
        // Create response stream from request, handling errors
        let output = async_stream::try_stream! {
            let mut in_stream = request.into_inner();
            let mut count = 0;

            loop {
                match in_stream.message().await {
                    Ok(Some(message)) => {
                        yield ResponseMessage {
                            code: count,
                            message: format!("Processed message with ID: {:?}", message.id),
                        };
                        count += 1;
                    },
                    Ok(None) => break, // End of stream
                    Err(e) => {
                        println!("Service received error: {:?}", e);
                        // We could handle the error and continue
                        // or propagate it
                        yield Err(e)?;
                    }
                }
            }
        };

        Ok(Response::new(Box::pin(output) as StreamResponseInner<ResponseMessage>))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Error injection example");

    // Create test messages
    let mut requests = Vec::new();
    for i in 0..5 {
        requests.push(RequestMessage {
            id: Bytes::from(i.to_string()),
            data: Bytes::from(format!("Test data {}", i)),
        });
    }

    // Create service
    let service = ErrorHandlingService;

    // Example 1: Error after specific number of messages
    {
        println!("\nExample 1: Error after 2 messages");

        // Create error injection
        let error = ErrorInjection::after_messages(
            2,
            Status::new(Code::Internal, "Simulated error after 2 messages"),
        );

        // Create streaming request with error
        let request = streaming_request_with_errors(requests.clone(), error);

        // Call service (which should handle the error)
        let response = service.process_stream(request).await?;

        // Process responses
        process_streaming_response(response, |msg, idx| {
            if let Ok(msg) = &msg {
                println!("Response {}: Success: code={}, message='{}'", idx, msg.code, msg.message);
            } else {
                println!("Response {}: Error: {:?}", idx, msg.err());
            }
        }).await;
    }

    // Example 2: Alternating errors
    {
        println!("\nExample 2: Alternating errors");

        // Create error injection
        let error = ErrorInjection::alternating(
            Status::new(Code::Internal, "Alternating error"),
        );

        // Create streaming request with error
        let request = streaming_request_with_errors(requests.clone(), error);

        // Call service
        let response = service.process_stream(request).await?;

        // Process responses
        process_streaming_response(response, |msg, idx| {
            if let Ok(msg) = &msg {
                println!("Response {}: Success: code={}, message='{}'", idx, msg.code, msg.message);
            } else {
                println!("Response {}: Error: {:?}", idx, msg.err());
            }
        }).await;
    }

    // Example 3: Custom error pattern
    {
        println!("\nExample 3: Custom error pattern (errors on prime indices)");

        // Create error injection with custom pattern
        let error = ErrorInjection::custom(
            |idx| {
                // Inject error on prime indices
                match idx {
                    2 | 3 | 5 | 7 | 11 | 13 => true,
                    _ => false,
                }
            },
            Status::new(Code::Internal, "Error on prime index"),
        );

        // Create streaming request with error
        let request = streaming_request_with_errors(requests.clone(), error);

        // Call service
        let response = service.process_stream(request).await?;

        // Process responses
        process_streaming_response(response, |msg, idx| {
            if let Ok(msg) = &msg {
                println!("Response {}: Success: code={}, message='{}'", idx, msg.code, msg.message);
            } else {
                println!("Response {}: Error: {:?}", idx, msg.err());
            }
        }).await;
    }

    Ok(())
}
```

## 4. Documentation Website

Consider creating a small documentation website (using mdBook or similar) with:

- Getting started guide
- API documentation
- Example code
- Testing best practices
- Migration guides
- FAQ section

## 5. Rustdoc Improvements

Ensure all public items have comprehensive documentation:

- All public functions
- All public types
- All public modules
- Crate-level documentation
- All examples are tested and working

## Implementation Timeline

1. README Enhancement: 1 day
2. API Documentation Enhancement: 2 days
3. Examples Directory Creation: 3 days
4. Documentation Website: 2 days (optional)
5. Rustdoc Improvements: 1 day

Total estimated time: 7-9 days

## Success Criteria

1. All public API functions have comprehensive documentation with examples
2. README provides clear guidance for both basic and advanced usage
3. Examples demonstrate all major features with runnable code
4. Documentation is accessible, well-structured, and easy to navigate
5. User feedback indicates documentation meets their needs
