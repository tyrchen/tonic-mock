// This example demonstrates how to use tonic-mock to test gRPC services with streaming endpoints.
//
// The example includes:
// 1. A sample gRPC service with client streaming, server streaming, and bidirectional streaming
// 2. Test cases for each streaming pattern using tonic-mock's test utilities
// 3. Example of error handling and testing error conditions

#![allow(clippy::needless_range_loop)]

use bytes::Bytes;
use futures::Stream;
use std::{cell::Cell, pin::Pin, time::Duration};
use tokio::runtime::Runtime;
use tonic::{Request, Response, Status, Streaming};
use tonic_mock::{
    process_streaming_response, stream_to_vec, streaming_request,
    test_utils::{
        TestRequest, TestResponse, assert_response_eq, create_stream_response_with_errors,
        create_test_messages,
    },
};

// -------------------- Sample gRPC Service -----------------------

// Define a sample gRPC service for testing
#[derive(Debug, Default)]
struct EchoService;

// Define the service trait
#[tonic::async_trait]
trait EchoServiceTrait {
    // Client streaming endpoint - client sends multiple messages, server responds with one
    async fn client_streaming(
        &self,
        request: Request<Streaming<TestRequest>>,
    ) -> Result<Response<TestResponse>, Status>;

    // Server streaming endpoint - client sends one message, server responds with multiple
    async fn server_streaming(
        &self,
        request: Request<TestRequest>,
    ) -> Result<
        Response<Pin<Box<dyn Stream<Item = Result<TestResponse, Status>> + Send + Sync + 'static>>>,
        Status,
    >;

    // Bidirectional streaming endpoint - client and server exchange multiple messages
    async fn bidirectional_streaming(
        &self,
        request: Request<Streaming<TestRequest>>,
    ) -> Result<
        Response<Pin<Box<dyn Stream<Item = Result<TestResponse, Status>> + Send + Sync + 'static>>>,
        Status,
    >;
}

// Implement the service
#[tonic::async_trait]
impl EchoServiceTrait for EchoService {
    async fn client_streaming(
        &self,
        request: Request<Streaming<TestRequest>>,
    ) -> Result<Response<TestResponse>, Status> {
        let mut stream = request.into_inner();
        let mut count = 0;
        let mut data = Vec::new();

        // Process all incoming messages
        while let Some(message) = stream.message().await? {
            count += 1;
            data.push(String::from_utf8_lossy(&message.id).to_string());
        }

        // Return a summary response
        let response = TestResponse::new(count, format!("Processed: {}", data.join(", ")));
        Ok(Response::new(response))
    }

    async fn server_streaming(
        &self,
        request: Request<TestRequest>,
    ) -> Result<
        Response<Pin<Box<dyn Stream<Item = Result<TestResponse, Status>> + Send + Sync + 'static>>>,
        Status,
    > {
        let message = request.into_inner();
        let id_str = String::from_utf8_lossy(&message.id).to_string();

        // Try to parse the ID as a count of responses to send
        let count = id_str.parse::<i32>().unwrap_or(3);

        // If count is negative, return an error for testing error handling
        if count < 0 {
            return Err(Status::invalid_argument("Count cannot be negative"));
        }

        // Create a stream of responses
        let stream = async_stream::try_stream! {
            for i in 0..count {
                // Inject an error at position 2 if count > 5 (for testing error cases)
                if i == 2 && count > 5 {
                    Err(Status::internal("Simulated error at position 2"))?;
                } else {
                    yield TestResponse::new(
                        i,
                        format!("Response {} for request {}", i, id_str)
                    );
                }
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }

    async fn bidirectional_streaming(
        &self,
        request: Request<Streaming<TestRequest>>,
    ) -> Result<
        Response<Pin<Box<dyn Stream<Item = Result<TestResponse, Status>> + Send + Sync + 'static>>>,
        Status,
    > {
        let mut in_stream = request.into_inner();

        // Create a collection to store all messages without holding references to in_stream
        let mut messages = Vec::new();

        // Collect all messages
        while let Some(message) = in_stream.message().await? {
            let index = messages.len();
            let id = message.id.clone();
            let data = message.data.clone();
            messages.push((index, id, data));
        }

        // Create a response stream
        let stream = async_stream::try_stream! {
            for (index, id, data) in messages {
                // Echo back with the message details
                yield TestResponse::new(
                    index as i32,
                    format!("Echo: id={:?}, data={:?}", id, data)
                );
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }
}

// -------------------- Test Functions -----------------------

// Test client streaming - client sends multiple messages, server aggregates them
fn test_client_streaming() {
    let rt = Runtime::new().unwrap();

    // Create the service
    let service = EchoService;

    // Create test messages using the test utility
    let messages = create_test_messages(5);

    // Create a streaming request
    let request = streaming_request(messages);

    // Call the service
    let response = rt.block_on(async { service.client_streaming(request).await.unwrap() });

    // Verify response
    let response = response.into_inner();
    assert_eq!(response.code, 5); // 5 messages processed
    assert!(response.message.contains("Processed: 0, 1, 2, 3, 4"));

    println!("✅ Client streaming test passed");
}

// Test server streaming - client sends a request, server sends multiple responses
fn test_server_streaming() {
    let rt = Runtime::new().unwrap();

    // Create the service
    let service = EchoService;

    // Create a request asking for 4 responses
    let request = Request::new(TestRequest::new("4", "test_data"));

    // Call the service
    let response = rt.block_on(async { service.server_streaming(request).await.unwrap() });

    // Process responses using stream_to_vec
    let results = rt.block_on(async { stream_to_vec(response).await });

    // Verify responses
    assert_eq!(results.len(), 4);
    for i in 0..4 {
        assert!(results[i].is_ok());
        let response = results[i].as_ref().unwrap();
        assert_eq!(response.code, i as i32);
        assert!(
            response
                .message
                .contains(&format!("Response {} for request 4", i))
        );
    }

    println!("✅ Server streaming test passed");
}

// Test bidirectional streaming - client and server exchange multiple messages
fn test_bidirectional_streaming() {
    let rt = Runtime::new().unwrap();

    // Create the service
    let service = EchoService;

    // Create test messages
    let messages = create_test_messages(3);

    // Create a streaming request
    let request = streaming_request(messages);

    // Call the service
    let response = rt.block_on(async { service.bidirectional_streaming(request).await.unwrap() });

    // Process responses using process_streaming_response
    rt.block_on(async {
        process_streaming_response(response, |msg: Result<TestResponse, Status>, idx| {
            assert!(msg.is_ok());
            if let Ok(response) = msg.as_ref() {
                assert_eq!(response.code, idx as i32);
                assert!(response.message.contains("Echo:"));
                assert!(
                    response
                        .message
                        .contains(&format!("id={:?}", Bytes::from(idx.to_string())))
                );
            }
        })
        .await;
    });

    println!("✅ Bidirectional streaming test passed");
}

// Test error handling in server streaming
fn test_server_streaming_errors() {
    let rt = Runtime::new().unwrap();

    // Create the service
    let service = EchoService;

    // 1. Test invalid argument error
    let request = Request::new(TestRequest::new("-1", "negative_count"));
    let result = rt.block_on(async { service.server_streaming(request).await });
    assert!(result.is_err());
    assert_eq!(result.err().unwrap().code(), tonic::Code::InvalidArgument);

    // 2. Test error in the middle of stream
    let request = Request::new(TestRequest::new("7", "error_in_stream"));
    let response = rt.block_on(async { service.server_streaming(request).await.unwrap() });

    // Process responses with potential errors
    let results = rt.block_on(async { stream_to_vec(response).await });

    // First two responses should be OK
    assert!(results[0].is_ok());
    assert!(results[1].is_ok());

    // Third response should be an error
    assert!(results[2].is_err());
    assert_eq!(
        results[2].as_ref().err().unwrap().code(),
        tonic::Code::Internal
    );

    println!("✅ Error handling test passed");
}

// Test creating a stream response with errors using test utilities
fn test_create_stream_with_errors() {
    let rt = Runtime::new().unwrap();

    // Create test responses
    let responses = vec![
        TestResponse::new(0, "Response 0"),
        TestResponse::new(1, "Response 1"),
        TestResponse::new(2, "Response 2"),
        TestResponse::new(3, "Response 3"),
    ];

    // Create an error status
    let error_status = Status::internal("Test error");

    // Create a streaming response with errors at indices 1 and 3
    let stream_response = create_stream_response_with_errors(responses, vec![1, 3], error_status);

    // Process the stream and check results
    let results = rt.block_on(async { stream_to_vec(stream_response).await });

    // Due to how streaming works with errors, we'll only get items up to the first error
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert!(results[1].is_err());

    // Check the successful response
    let response = results[0].as_ref().unwrap();
    assert_response_eq(response, 0, "Response 0");

    // Check the error
    assert_eq!(
        results[1].as_ref().err().unwrap().code(),
        tonic::Code::Internal
    );
    assert_eq!(results[1].as_ref().err().unwrap().message(), "Test error");

    println!("✅ Create stream with errors test passed");
}

// Test timeout functionality when processing streaming responses
fn test_streaming_response_timeout() {
    use tonic_mock::{process_streaming_response_with_timeout, stream_to_vec_with_timeout};

    let rt = Runtime::new().unwrap();

    // Create the service - this one has a deliberate delay
    let service = TimeoutEchoService;

    // Create a request that will cause a delay
    let request = Request::new(TestRequest::new("slow", "test_data"));

    // Call the service
    let _response = rt.block_on(async { service.server_streaming(request).await.unwrap() });

    // 1. Test processing with timeout
    let timeout_occurred = Cell::new(false);
    rt.block_on(async {
        // We need to create a fresh response for each test since streams can only be consumed once
        let fresh_request = Request::new(TestRequest::new("slow", "test_data"));
        let fresh_response = service.server_streaming(fresh_request).await.unwrap();

        process_streaming_response_with_timeout(
            fresh_response,
            Duration::from_millis(50), // Very short timeout
            |msg, idx| {
                if idx == 0 {
                    // First message should be ok
                    assert!(msg.is_ok());
                } else if idx == 1 {
                    // Second message should time out
                    assert!(msg.is_err());
                    assert_eq!(
                        msg.as_ref().err().unwrap().code(),
                        tonic::Code::DeadlineExceeded
                    );
                    timeout_occurred.set(true);
                }
            },
        )
        .await;
    });

    assert!(timeout_occurred.get(), "Timeout should have occurred");

    // 2. Test converting to vec with timeout
    let request = Request::new(TestRequest::new("slow", "test_data"));
    let response = rt.block_on(async { service.server_streaming(request).await.unwrap() });

    let results = rt
        .block_on(async { stream_to_vec_with_timeout(response, Duration::from_millis(50)).await });

    assert_eq!(results.len(), 2); // First message and timeout error
    assert!(results[0].is_ok());
    assert!(results[1].is_err());
    assert_eq!(
        results[1].as_ref().err().unwrap().code(),
        tonic::Code::DeadlineExceeded
    );

    println!("✅ Streaming response timeout test passed");
}

// A service that deliberately introduces delays for testing timeouts
#[derive(Debug)]
struct TimeoutEchoService;

#[tonic::async_trait]
impl EchoServiceTrait for TimeoutEchoService {
    async fn client_streaming(
        &self,
        request: Request<Streaming<TestRequest>>,
    ) -> Result<Response<TestResponse>, Status> {
        // Just delegate to the regular service
        EchoService.client_streaming(request).await
    }

    async fn server_streaming(
        &self,
        request: Request<TestRequest>,
    ) -> Result<
        Response<Pin<Box<dyn Stream<Item = Result<TestResponse, Status>> + Send + Sync + 'static>>>,
        Status,
    > {
        let message = request.into_inner();
        let id_str = String::from_utf8_lossy(&message.id).to_string();

        // Create a stream with deliberate delays
        let stream = async_stream::try_stream! {
            // First message comes immediately
            yield TestResponse::new(0, format!("Fast response for {}", id_str));

            // Second message has a delay
            tokio::time::sleep(Duration::from_millis(200)).await;
            yield TestResponse::new(1, format!("Slow response for {}", id_str));

            // Third message has an even longer delay
            tokio::time::sleep(Duration::from_millis(300)).await;
            yield TestResponse::new(2, format!("Very slow response for {}", id_str));
        };

        Ok(Response::new(Box::pin(stream)))
    }

    async fn bidirectional_streaming(
        &self,
        request: Request<Streaming<TestRequest>>,
    ) -> Result<
        Response<Pin<Box<dyn Stream<Item = Result<TestResponse, Status>> + Send + Sync + 'static>>>,
        Status,
    > {
        // Just delegate to the regular service
        EchoService.bidirectional_streaming(request).await
    }
}

// -------------------- Main Function -----------------------

fn main() {
    println!("Running gRPC testing examples with tonic-mock");
    println!("---------------------------------------------");

    // Run all tests
    test_client_streaming();
    test_server_streaming();
    test_bidirectional_streaming();
    test_server_streaming_errors();
    test_create_stream_with_errors();
    test_streaming_response_timeout();

    println!("---------------------------------------------");
    println!("All tests passed! 🎉");
}
