/*!
# Client Mocking

This module provides utilities for mocking gRPC clients in tests. The [`MockableGrpcClient`] allows
you to configure mock responses for specific service methods, enabling isolated testing of client code
without actual gRPC calls.

## Core Components

- [`MockableGrpcClient`]: The main mock client that handles requests
- [`MockResponseDefinition`]: Defines mock responses with optional metadata, delays, and errors
- [`MockBuilder`]: Builder for configuring responses for a specific method
- [`GrpcClientExt`]: Extension trait to implement for client types to enable mocking

## Basic Usage

```rust
use tonic_mock::client_mock::{MockableGrpcClient, MockResponseDefinition, GrpcClientExt};
use tonic::{Request, Response, Status, Code};
use prost::Message;

// Define message types (normally generated from protobuf)
#[derive(Clone, PartialEq, Message)]
pub struct GetUserRequest {
    #[prost(string, tag = "1")]
    pub user_id: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct User {
    #[prost(string, tag = "1")]
    pub name: String,
}

// Define a client that will use the mock
#[derive(Clone)]
struct UserServiceClient<T> {
    inner: T,
}

// Implement the GrpcClientExt trait for your client
impl GrpcClientExt<UserServiceClient<MockableGrpcClient>>
    for UserServiceClient<MockableGrpcClient>
{
    fn with_mock(mock: MockableGrpcClient) -> Self {
        Self { inner: mock }
    }
}

// Implement your client methods
impl UserServiceClient<MockableGrpcClient> {
    pub async fn get_user(
        &mut self,
        request: Request<GetUserRequest>
    ) -> Result<Response<User>, Status> {
        // Extract request data
        let request_data = request.into_inner();

        // Encode the request
        let encoded = tonic_mock::grpc_mock::encode_grpc_request(request_data);

        // Call the mock service
        let (response_bytes, headers) = self.inner
            .handle_request("user.UserService", "GetUser", &encoded)
            .await?;

        // Decode the response
        let response: User =
            tonic_mock::grpc_mock::decode_grpc_message(&response_bytes)?;

        // Return the response
        Ok(Response::new(response))
    }
}

#[tokio::test]
async fn test_user_service() {
    // Create a mock client
    let mock = MockableGrpcClient::new();

    // Configure mock responses - note the await for async methods
    mock.mock::<GetUserRequest, User>("user.UserService", "GetUser")
        .respond_with(MockResponseDefinition::ok(User {
            name: "Test User".to_string(),
        }))
        .await;

    // Create a client with the mock
    let mut client = UserServiceClient::with_mock(mock);

    // Test the client
    let request = Request::new(GetUserRequest {
        user_id: "user-123".to_string()
    });

    let response = client.get_user(request).await.unwrap();
    assert_eq!(response.get_ref().name, "Test User");
}
```

## Conditional Responses

You can configure the mock to return different responses based on request content:

```rust
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# use tonic_mock::client_mock::{MockableGrpcClient, MockResponseDefinition};
# use tonic::{Code, Status};
# use prost::Message;
#
# #[derive(Clone, PartialEq, Message)]
# pub struct GetUserRequest {
#     #[prost(string, tag = "1")]
#     pub user_id: String,
# }
#
# #[derive(Clone, PartialEq, Message)]
# pub struct User {
#     #[prost(string, tag = "1")]
#     pub name: String,
# }
#
let mock = MockableGrpcClient::new();

// Configure different responses for different conditions
mock.mock::<GetUserRequest, User>("user.UserService", "GetUser")
    .respond_when(
        |req| req.user_id == "admin",
        MockResponseDefinition::ok(User {
            name: "Administrator".to_string(),
        })
    )
    .await
    .respond_when(
        |req| req.user_id == "guest",
        MockResponseDefinition::ok(User {
            name: "Guest User".to_string(),
        })
    )
    .await
    .respond_with(
        MockResponseDefinition::err(Status::new(Code::NotFound, "User not found"))
    )
    .await;
# Ok(())
# }
```

## Adding Metadata and Delays

```rust
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# use tonic_mock::client_mock::{MockableGrpcClient, MockResponseDefinition};
# use prost::Message;
#
# #[derive(Clone, PartialEq, Message)]
# pub struct GetUserRequest {
#     #[prost(string, tag = "1")]
#     pub user_id: String,
# }
#
# #[derive(Clone, PartialEq, Message)]
# pub struct User {
#     #[prost(string, tag = "1")]
#     pub name: String,
# }
#
let mock = MockableGrpcClient::new();

// Add metadata and delay to response
mock.mock::<GetUserRequest, User>("user.UserService", "GetUser")
    .respond_with(
        MockResponseDefinition::ok(User {
            name: "Test User".to_string(),
        })
        .with_metadata("x-request-id", "12345")
        .with_metadata("server", "test-server")
        .with_delay(200) // 200ms delay
    )
    .await;
# Ok(())
# }
```

## Resetting Mocks

When you need to clear all configured mock responses:

```rust
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# use tonic_mock::client_mock::MockableGrpcClient;
let mock = MockableGrpcClient::new();

// Configure mock responses...

// Reset all mock configurations
mock.reset().await;
# Ok(())
# }
```

## Important Notes

- All configuration methods (`respond_with`, `respond_when`, and `reset`) are async and must be awaited
- The `handle_request` method used in client implementations is also async and must be awaited
- Use `mock.clone()` if you need to keep a reference to the mock after giving it to a client
*/

use bytes::Bytes;
use http::{HeaderMap, HeaderName, header::HeaderValue};
use prost::Message;
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::sleep;
use tonic::Status;

use crate::grpc_mock::{decode_grpc_message, encode_grpc_response};

/// Predefined response for a mock gRPC service
#[derive(Clone)]
pub struct MockResponseDefinition<Resp> {
    /// The response to return
    pub response: Option<Resp>,
    /// Status to return (if error)
    pub status: Option<Status>,
    /// Additional metadata as key-value pairs
    pub metadata_pairs: Vec<(String, String)>,
    /// Delay before responding (simulates network latency)
    pub delay_ms: Option<u64>,
}

impl<Resp> Default for MockResponseDefinition<Resp> {
    fn default() -> Self {
        Self {
            response: None,
            status: None,
            metadata_pairs: Vec::new(),
            delay_ms: None,
        }
    }
}

impl<Resp> MockResponseDefinition<Resp> {
    /// Create a new success response definition
    ///
    /// # Arguments
    /// * `response` - The successful response to return
    ///
    /// # Example
    /// ```
    /// use tonic_mock::client_mock::MockResponseDefinition;
    ///
    /// #[derive(Clone, PartialEq, ::prost::Message)]
    /// pub struct MyResponse {
    ///     #[prost(string, tag = "1")]
    ///     pub result: String,
    /// }
    ///
    /// let mock_response = MockResponseDefinition::ok(MyResponse {
    ///     result: "success".to_string(),
    /// });
    /// ```
    pub fn ok(response: Resp) -> Self {
        Self {
            response: Some(response),
            status: None,
            metadata_pairs: Vec::new(),
            delay_ms: None,
        }
    }

    /// Create a new error response definition
    ///
    /// # Arguments
    /// * `status` - The error status to return
    ///
    /// # Example
    /// ```
    /// use tonic_mock::client_mock::MockResponseDefinition;
    /// use tonic::{Code, Status};
    ///
    /// let mock_error = MockResponseDefinition::<()>::err(
    ///     Status::new(Code::NotFound, "Resource not found")
    /// );
    /// ```
    pub fn err(status: Status) -> Self {
        Self {
            response: None,
            status: Some(status),
            metadata_pairs: Vec::new(),
            delay_ms: None,
        }
    }

    /// Add a metadata entry to the response
    ///
    /// # Arguments
    /// * `key` - The metadata key
    /// * `value` - The metadata value
    ///
    /// # Returns
    /// Self with the added metadata
    ///
    /// # Example
    /// ```
    /// use tonic_mock::client_mock::MockResponseDefinition;
    ///
    /// #[derive(Clone, PartialEq, ::prost::Message)]
    /// pub struct MyResponse {
    ///     #[prost(string, tag = "1")]
    ///     pub result: String,
    /// }
    ///
    /// let mock_response = MockResponseDefinition::ok(MyResponse {
    ///     result: "success".to_string(),
    /// })
    /// .with_metadata("x-request-id", "12345")
    /// .with_metadata("content-type", "application/grpc+proto");
    /// ```
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata_pairs
            .push((key.to_string(), value.to_string()));
        self
    }

    /// Add a delay to simulate network latency
    ///
    /// # Arguments
    /// * `delay_ms` - The delay in milliseconds
    ///
    /// # Returns
    /// Self with the added delay
    ///
    /// # Example
    /// ```
    /// use tonic_mock::client_mock::MockResponseDefinition;
    ///
    /// #[derive(Clone, PartialEq, ::prost::Message)]
    /// pub struct MyResponse {
    ///     #[prost(string, tag = "1")]
    ///     pub result: String,
    /// }
    ///
    /// // Create a response with a 200ms delay
    /// let mock_response = MockResponseDefinition::ok(MyResponse {
    ///     result: "delayed response".to_string(),
    /// })
    /// .with_delay(200);
    /// ```
    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = Some(delay_ms);
        self
    }
}

// Private function to create headers from a MockResponseDefinition
fn create_headers_from_def<Resp: Clone>(response_def: &MockResponseDefinition<Resp>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let response_clone = response_def.clone();

    // Add all metadata pairs as headers
    for (key, value) in response_clone.metadata_pairs {
        if let Ok(header_value) = HeaderValue::from_str(value.as_str()) {
            headers.insert(key.parse::<HeaderName>().unwrap(), header_value);
        }
    }

    // Add delay as a special header if present
    if let Some(delay) = response_def.delay_ms {
        if let Ok(delay_header) = HeaderValue::from_str(&delay.to_string()) {
            headers.insert("mock-delay-ms", delay_header);
        }
    }

    headers
}

/// Type alias for a predicate function
type PredicateFn<Req> = Arc<dyn Fn(&Req) -> bool + Send + Sync>;

/// A mockable gRPC client for testing
///
/// This struct provides a way to mock gRPC services for testing. It allows
/// configuring mock responses for specific service methods with various
/// options, such as:
///
/// - Returning static responses for any request
/// - Conditionally returning responses based on request content
/// - Simulating network delays
/// - Including custom metadata in responses
/// - Returning error statuses instead of responses
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use tonic_mock::client_mock::{MockableGrpcClient, MockResponseDefinition};
/// use prost::Message;
///
/// #[derive(Clone, PartialEq, Message)]
/// pub struct MyRequest {
///     #[prost(string, tag = "1")]
///     pub id: String,
/// }
///
/// #[derive(Clone, PartialEq, Message)]
/// pub struct MyResponse {
///     #[prost(string, tag = "1")]
///     pub result: String,
/// }
///
/// // Create a mock client
/// let mock = MockableGrpcClient::new();
///
/// // Configure a mock response
/// mock.mock::<MyRequest, MyResponse>("my.Service", "MyMethod")
///     .respond_with(MockResponseDefinition::ok(MyResponse {
///         result: "test result".to_string(),
///     }))
///     .await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct MockableGrpcClient {
    handlers: Arc<Mutex<Vec<MockHandler>>>,
}

/// Abstract handler type that doesn't expose generic parameters
#[allow(clippy::type_complexity)]
enum MockHandler {
    Any {
        service: String,
        method: String,
        handler: Box<dyn Fn(&[u8]) -> Result<(Bytes, HeaderMap), Status> + Send + Sync>,
    },
}

impl MockableGrpcClient {
    /// Create a new mockable gRPC client
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a mock response for a specific service method and request type
    ///
    /// This method returns a builder that can be used to configure mock responses
    /// for the specified service method.
    ///
    /// # Arguments
    /// * `service_name` - The name of the gRPC service (e.g., "package.Service")
    /// * `method_name` - The name of the method to mock
    ///
    /// # Returns
    /// A builder for configuring mock responses
    ///
    /// # Example
    /// ```
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use tonic_mock::client_mock::{MockableGrpcClient, MockResponseDefinition};
    /// use prost::Message;
    ///
    /// #[derive(Clone, PartialEq, Message)]
    /// pub struct MyRequest {
    ///     #[prost(string, tag = "1")]
    ///     pub id: String,
    /// }
    ///
    /// #[derive(Clone, PartialEq, Message)]
    /// pub struct MyResponse {
    ///     #[prost(string, tag = "1")]
    ///     pub result: String,
    /// }
    ///
    /// // You can configure a mock to respond to a specific service method
    /// let mock = MockableGrpcClient::new();
    ///
    /// // Configure a mock response for "UserService.GetUser"
    /// mock.mock::<MyRequest, MyResponse>("user.UserService", "GetUser")
    ///     .respond_with(MockResponseDefinition::ok(MyResponse {
    ///         result: "User data".to_string(),
    ///     }))
    ///     .await;
    /// # Ok(())
    /// # }
    /// ```
    pub fn mock<Req, Resp>(&self, service_name: &str, method_name: &str) -> MockBuilder<Req, Resp>
    where
        Req: Message + Default + 'static,
        Resp: Message + Default + Clone + 'static,
    {
        MockBuilder {
            client: self.clone(),
            service_name: service_name.to_string(),
            method_name: method_name.to_string(),
            _marker: PhantomData,
        }
    }

    /// Reset all mock definitions
    ///
    /// This method clears all previously configured mock responses.
    pub async fn reset(&self) {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.clear();
    }

    /// Handle a gRPC request
    ///
    /// This method is used internally by client implementations to handle
    /// mock requests. It looks up the appropriate handler for the service
    /// and method and delegates to it.
    ///
    /// # Arguments
    /// * `service_name` - The name of the gRPC service
    /// * `method_name` - The name of the method being called
    /// * `request_bytes` - The encoded request message
    ///
    /// # Returns
    /// The encoded response and any metadata, or an error status
    pub async fn handle_request(
        &self,
        service_name: &str,
        method_name: &str,
        request_bytes: &[u8],
    ) -> Result<(Bytes, HeaderMap), Status> {
        // Find handler that matches this service and method
        let handler_result = {
            let handlers = self.handlers.lock().unwrap();

            // Find the handler and get its result
            let mut handler_result = None;
            for handler in handlers.iter().rev() {
                // Reverse iteration to check most recent first
                match handler {
                    MockHandler::Any {
                        service,
                        method,
                        handler: h,
                    } => {
                        if service == service_name && method == method_name {
                            let result = h(request_bytes);

                            // For error statuses that are predicate skips, we should continue to the next handler
                            if let Err(status) = &result {
                                if status.message() == "__TONIC_MOCK_PREDICATE_SKIP__" {
                                    continue;
                                }
                            }

                            handler_result = Some(result);
                            break;
                        }
                    }
                }
            }

            // If no handler was found, return an error
            handler_result.unwrap_or_else(|| {
                Err(Status::unimplemented(format!(
                    "No mock handler configured for {}::{}",
                    service_name, method_name
                )))
            })
        };

        // Process the result outside the mutex guard
        if let Ok((_response_bytes, metadata)) = &handler_result {
            if let Some(delay_header) = metadata.get("mock-delay-ms") {
                if let Ok(delay_str) = delay_header.to_str() {
                    if let Ok(delay_ms) = delay_str.parse::<u64>() {
                        if delay_ms > 0 {
                            // Use tokio's sleep to simulate network delay
                            // The mutex guard is already dropped here
                            sleep(Duration::from_millis(delay_ms)).await;
                        }
                    }
                }
            }
        }

        handler_result
    }

    /// Register a handler function for a specific service and method
    async fn register_handler<F>(&self, service_name: String, method_name: String, handler: F)
    where
        F: Fn(&[u8]) -> Result<(Bytes, HeaderMap), Status> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.push(MockHandler::Any {
            service: service_name,
            method: method_name,
            handler: Box::new(handler),
        });
    }
}

/// Builder for configuring mock responses
pub struct MockBuilder<Req, Resp>
where
    Req: Message + Default + 'static,
    Resp: Message + Default + Clone + 'static,
{
    client: MockableGrpcClient,
    service_name: String,
    method_name: String,
    _marker: PhantomData<(Req, Resp)>,
}

impl<Req, Resp> MockBuilder<Req, Resp>
where
    Req: Message + Default + 'static,
    Resp: Message + Default + Clone + 'static,
{
    /// Configure a static response for any request
    ///
    /// This method adds a handler that returns the specified response
    /// for any request to the service method, regardless of the request content.
    ///
    /// # Arguments
    /// * `response_def` - The mock response definition
    ///
    /// # Returns
    /// Self for method chaining
    ///
    /// # Example
    /// ```
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use tonic_mock::client_mock::{MockableGrpcClient, MockResponseDefinition};
    /// use prost::Message;
    ///
    /// #[derive(Clone, PartialEq, Message)]
    /// pub struct HelloRequest {
    ///     #[prost(string, tag = "1")]
    ///     pub name: String,
    /// }
    ///
    /// #[derive(Clone, PartialEq, Message)]
    /// pub struct HelloResponse {
    ///     #[prost(string, tag = "1")]
    ///     pub message: String,
    /// }
    ///
    /// let mock = MockableGrpcClient::new();
    ///
    /// // Configure a response for any Hello request
    /// mock.mock::<HelloRequest, HelloResponse>("greeter.Greeter", "SayHello")
    ///    .respond_with(MockResponseDefinition::ok(HelloResponse {
    ///        message: "Hello, world!".to_string(),
    ///    }))
    ///    .await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn respond_with(self, response_def: MockResponseDefinition<Resp>) -> Self {
        let service_name = self.service_name.clone();
        let method_name = self.method_name.clone();
        let response_clone = response_def.clone();

        let handler = move |_request_bytes: &[u8]| {
            // Create the response based on the definition
            if let Some(status) = &response_clone.status {
                // Error response
                return Err(status.clone());
            }

            if let Some(response) = &response_clone.response {
                // Success response
                let response_bytes = encode_grpc_response(response.clone());
                let headers = create_headers_from_def(&response_clone);
                return Ok((response_bytes, headers));
            }

            // In theory shouldn't happen if the ResponseDefinition is properly constructed
            Err(Status::internal(
                "Invalid MockResponseDefinition: both response and status are None",
            ))
        };

        self.client
            .register_handler(service_name, method_name, handler)
            .await;

        self
    }

    /// Configure a conditional response based on a request predicate
    ///
    /// This method adds a handler that returns the specified response
    /// only if the request matches the predicate function. If the predicate
    /// returns false, the request falls through to the next matching handler.
    ///
    /// # Arguments
    /// * `predicate` - A function that evaluates the request and returns true if it should be handled
    /// * `response_def` - The mock response definition to use if the predicate matches
    ///
    /// # Returns
    /// Self for method chaining
    ///
    /// # Example
    /// ```
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use tonic_mock::client_mock::{MockableGrpcClient, MockResponseDefinition};
    /// use tonic::{Code, Status};
    /// use prost::Message;
    ///
    /// #[derive(Clone, PartialEq, Message)]
    /// pub struct GetUserRequest {
    ///     #[prost(string, tag = "1")]
    ///     pub user_id: String,
    /// }
    ///
    /// #[derive(Clone, PartialEq, Message)]
    /// pub struct User {
    ///     #[prost(string, tag = "1")]
    ///     pub name: String,
    /// }
    ///
    /// let mock = MockableGrpcClient::new();
    ///
    /// // Configure a response for a specific user ID
    /// mock.mock::<GetUserRequest, User>("user.UserService", "GetUser")
    ///     .respond_when(
    ///         |req| req.user_id == "user123",
    ///         MockResponseDefinition::ok(User {
    ///             name: "User 123".to_string(),
    ///         })
    ///     )
    ///     .await
    ///     // Default response for any other user ID
    ///     .respond_with(
    ///         MockResponseDefinition::err(Status::new(Code::NotFound, "User not found"))
    ///     )
    ///     .await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn respond_when<F>(
        self,
        predicate: F,
        response_def: MockResponseDefinition<Resp>,
    ) -> Self
    where
        F: Fn(&Req) -> bool + Send + Sync + 'static,
    {
        let service_name = self.service_name.clone();
        let method_name = self.method_name.clone();
        let predicate = Arc::new(predicate) as PredicateFn<Req>;
        let response_clone = response_def.clone();

        let handler = move |request_bytes: &[u8]| {
            // First decode the request
            let req: Req = match decode_grpc_message(request_bytes) {
                Ok(req) => req,
                Err(status) => return Err(status),
            };

            // Check if the predicate matches
            if !predicate(&req) {
                // Return a special status that signals to skip this handler
                return Err(Status::internal("__TONIC_MOCK_PREDICATE_SKIP__"));
            }

            // Create the response based on the definition
            if let Some(status) = &response_clone.status {
                // Error response
                return Err(status.clone());
            }

            if let Some(response) = &response_clone.response {
                // Success response
                let response_bytes = encode_grpc_response(response.clone());
                let headers = create_headers_from_def(&response_clone);
                return Ok((response_bytes, headers));
            }

            // In theory shouldn't happen if the ResponseDefinition is properly constructed
            Err(Status::internal(
                "Invalid MockResponseDefinition: both response and status are None",
            ))
        };

        self.client
            .register_handler(service_name, method_name, handler)
            .await;

        self
    }
}

/// Extension trait for gRPC clients to support mocking
///
/// This trait should be implemented for your gRPC client types to
/// enable creating mock instances with the `with_mock` method.
///
/// # Example
///
/// ```
/// use tonic_mock::client_mock::{GrpcClientExt, MockableGrpcClient};
///
/// // A typical gRPC client
/// pub struct MyServiceClient<T> {
///     inner: T,
/// }
///
/// // Implement the extension trait
/// impl GrpcClientExt<MyServiceClient<MockableGrpcClient>> for MyServiceClient<MockableGrpcClient> {
///     fn with_mock(mock: MockableGrpcClient) -> Self {
///         Self { inner: mock }
///     }
/// }
/// ```
pub trait GrpcClientExt<S> {
    /// Create a new client instance that uses the provided mock service
    fn with_mock(mock: MockableGrpcClient) -> S;
}
