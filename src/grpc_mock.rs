/*!
# gRPC Mock Utilities

This module provides low-level utilities for mocking gRPC messages and calls. These utilities
give you fine-grained control over gRPC message encoding, decoding, and handling, which is
useful for:

- Testing gRPC handlers directly without setting up a full service
- Implementing custom gRPC client/server logic
- Debugging gRPC message format issues
- Creating mock implementations that work with the wire format

## Core Functions

- [`encode_grpc_request`]: Encodes a message into the gRPC wire format for requests
- [`encode_grpc_response`]: Encodes a message into the gRPC wire format for responses
- [`decode_grpc_message`]: Decodes a message from the gRPC wire format
- [`mock_grpc_call`]: Simulates a gRPC method call with a handler function
- [`create_grpc_uri`]: Creates a URI for a gRPC service method

## Example: Encoding and Decoding

```rust
use tonic_mock::grpc_mock::{encode_grpc_request, decode_grpc_message};
use prost::Message;

// Define a message type
#[derive(Clone, PartialEq, Message)]
pub struct ExampleRequest {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub data: String,
}

// Encode a request
let request = ExampleRequest {
    id: "request-123".to_string(),
    data: "test data".to_string(),
};
let encoded = encode_grpc_request(request.clone());

// Decode the request
let decoded: ExampleRequest = decode_grpc_message(&encoded).unwrap();
assert_eq!(decoded, request);
```

## Example: Mocking a gRPC Call

```rust
use tonic_mock::grpc_mock::mock_grpc_call;
use tonic::Status;
use prost::Message;

// Define message types
#[derive(Clone, PartialEq, Message)]
pub struct GreetRequest {
    #[prost(string, tag = "1")]
    pub name: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct GreetResponse {
    #[prost(string, tag = "1")]
    pub message: String,
}

// Mock a gRPC call
let request = GreetRequest { name: "Alice".to_string() };
let response = mock_grpc_call(
    "greeter.GreeterService",
    "Greet",
    request,
    |req: GreetRequest| {
        Ok(GreetResponse {
            message: format!("Hello, {}!", req.name),
        })
    }
).unwrap();

assert_eq!(response.message, "Hello, Alice!");
```

These utilities are primarily intended for internal use by the [`client_mock`](crate::client_mock)
module, but are exposed for advanced use cases where direct control over the gRPC wire format
is needed.
*/

use bytes::{Bytes, BytesMut};
use http::{Uri, uri::PathAndQuery};
use prost::Message;
use std::fmt::Debug;
use tonic::{Code, Status};

/// Creates a gRPC HTTP request for a specific service method and request message.
///
/// # Arguments
/// * `service_name` - The full gRPC service name, e.g., "package.ServiceName"
/// * `method_name` - The method name to call
/// * `request` - The request message to send
///
/// # Returns
/// The encoded request for use in testing
///
/// # Example
/// ```
/// use tonic_mock::grpc_mock::encode_grpc_request;
/// use tonic_mock::test_utils::TestRequest;
///
/// let request = TestRequest::new("test-id", "test-data");
/// let encoded = encode_grpc_request(request);
/// ```
pub fn encode_grpc_request<T>(request: T) -> Bytes
where
    T: Message + Default + Send + 'static,
{
    // Encode the request message
    let mut buf = BytesMut::with_capacity(request.encoded_len() + 5);
    buf.resize(5, 0); // 5 bytes for the gRPC header

    // Encode the message payload
    request.encode(&mut buf).unwrap();

    // Set the gRPC header (compression flag and message length)
    let message_len = buf.len() - 5;
    buf[0] = 0; // No compression
    buf[1..5].copy_from_slice(&(message_len as u32).to_be_bytes());

    buf.freeze()
}

/// Creates a gRPC HTTP response for a specific response message.
///
/// # Arguments
/// * `response` - The response message to send
///
/// # Returns
/// The encoded response for testing
///
/// # Example
/// ```
/// use tonic_mock::grpc_mock::encode_grpc_response;
/// use tonic_mock::test_utils::TestResponse;
///
/// let response = TestResponse::new(200, "OK");
/// let encoded = encode_grpc_response(response);
/// ```
pub fn encode_grpc_response<T>(response: T) -> Bytes
where
    T: Message + Default + Send + 'static,
{
    // Encode the response message
    let mut buf = BytesMut::with_capacity(response.encoded_len() + 5);
    buf.resize(5, 0); // 5 bytes for the gRPC header

    // Encode the message payload
    response.encode(&mut buf).unwrap();

    // Set the gRPC header (compression flag and message length)
    let message_len = buf.len() - 5;
    buf[0] = 0; // No compression
    buf[1..5].copy_from_slice(&(message_len as u32).to_be_bytes());

    buf.freeze()
}

/// Decodes a gRPC request body into a message.
///
/// # Arguments
/// * `bytes` - The raw bytes of the gRPC request body
///
/// # Returns
/// The decoded message or an error
///
/// # Example
/// ```
/// use tonic_mock::grpc_mock::{encode_grpc_request, decode_grpc_message};
/// use tonic_mock::test_utils::TestRequest;
///
/// // Create a request and encode it
/// let request = TestRequest::new("test-id", "test-data");
/// let encoded = encode_grpc_request(request);
///
/// // Decode the message
/// let decoded: TestRequest = decode_grpc_message(&encoded).unwrap();
/// assert_eq!(decoded.id, "test-id".as_bytes());
/// ```
pub fn decode_grpc_message<T>(bytes: &[u8]) -> Result<T, Status>
where
    T: Message + Default + Debug,
{
    if bytes.len() < 5 {
        return Err(Status::new(Code::InvalidArgument, "Message too short"));
    }

    // Parse the gRPC header
    let compression_flag = bytes[0];
    if compression_flag != 0 {
        return Err(Status::new(
            Code::Unimplemented,
            "Compression not supported",
        ));
    }

    let message_len = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;

    if bytes.len() < 5 + message_len {
        return Err(Status::new(
            Code::InvalidArgument,
            format!(
                "Message incomplete: expected {} bytes, got {}",
                message_len,
                bytes.len() - 5
            ),
        ));
    }

    // Decode the message
    match T::decode(&bytes[5..5 + message_len]) {
        Ok(message) => Ok(message),
        Err(err) => Err(Status::new(
            Code::InvalidArgument,
            format!("Failed to decode message: {}", err),
        )),
    }
}

/// Constructs a URI for a gRPC service method.
///
/// # Arguments
/// * `service_name` - The full gRPC service name, e.g., "package.ServiceName"
/// * `method_name` - The method name to call
///
/// # Returns
/// A URI for the gRPC method
///
/// # Example
/// ```
/// use tonic_mock::grpc_mock::create_grpc_uri;
///
/// let uri = create_grpc_uri("example.TestService", "TestMethod");
/// assert_eq!(uri.path(), "/example.TestService/TestMethod");
/// ```
pub fn create_grpc_uri(service_name: &str, method_name: &str) -> Uri {
    let path_str = format!("/{}/{}", service_name, method_name);
    Uri::builder()
        .scheme("http")
        .authority("localhost")
        .path_and_query(PathAndQuery::from_maybe_shared(path_str).unwrap())
        .build()
        .unwrap()
}

/// A simple helper function to mock a gRPC service call.
///
/// # Arguments
/// * `service_name` - The full gRPC service name, e.g., "package.ServiceName"
/// * `method_name` - The method name to call
/// * `request` - The request message to send
/// * `handler` - A function that takes a request and returns a response or error
///
/// # Returns
/// The response from the handler
///
/// # Example
/// ```
/// use tonic_mock::grpc_mock::mock_grpc_call;
/// use tonic_mock::test_utils::{TestRequest, TestResponse};
/// use tonic::Status;
///
/// let request = TestRequest::new("test-id", "test-data");
///
/// let response = mock_grpc_call(
///     "example.TestService",
///     "TestMethod",
///     request,
///     |req: TestRequest| {
///         Ok(TestResponse::new(200, format!("Processed: {}", String::from_utf8_lossy(&req.id))))
///     }
/// ).unwrap();
///
/// assert_eq!(response.code, 200);
/// ```
pub fn mock_grpc_call<Req, Resp, F>(
    _service_name: &str,
    _method_name: &str,
    request: Req,
    handler: F,
) -> Result<Resp, Status>
where
    Req: Message + Default + Send + Clone + 'static,
    Resp: Message + Default + Send + 'static,
    F: FnOnce(Req) -> Result<Resp, Status>,
{
    // In a real implementation, we'd use a body stream
    // For simplicity, we'll just use the original request since we know what it is
    handler(request)
}
