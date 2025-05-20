// Example showing how to test gRPC bidirectional streaming

use prost::Message;
use tonic::{Request, Response, Status, Streaming};
use tonic_mock::{
    StreamResponseInner, stream_to_vec, streaming_request,
    test_utils::{TestRequest, TestResponse},
};

// Sample schema for our messages
#[derive(Clone, PartialEq, Message)]
pub struct ExampleRequest {
    #[prost(string, tag = "1")]
    pub user_id: String,
    #[prost(string, tag = "2")]
    pub message: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ExampleResponse {
    #[prost(int32, tag = "1")]
    pub status: i32,
    #[prost(string, tag = "2")]
    pub response: String,
}

// Our sample bidirectional streaming service
async fn echo_service(
    request: Request<Streaming<ExampleRequest>>,
) -> Result<Response<StreamResponseInner<ExampleResponse>>, Status> {
    let mut request_stream = request.into_inner();

    // Create response stream
    let response_stream = async_stream::try_stream! {
        while let Some(req) = request_stream.message().await? {
            // Simple echo response
            let response = ExampleResponse {
                status: 200,
                response: format!("Received: {} from user {}", req.message, req.user_id),
            };

            yield response;
        }
    };

    // Return the response without explicit casting
    Ok(Response::new(Box::pin(response_stream)))
}

// A concrete example using the direct approach with pre-made messages
async fn run_direct_api_example() {
    println!("\nExample 1: Testing bidirectional streaming with pre-collected messages");

    // Create messages to send
    let messages = vec![
        ExampleRequest {
            user_id: "user123".to_string(),
            message: "Hello chat!".to_string(),
        },
        ExampleRequest {
            user_id: "user123".to_string(),
            message: "How's the weather?".to_string(),
        },
    ];

    println!(
        "- Created {} messages to send to the service",
        messages.len()
    );

    // Create a direct mock stream
    let request = streaming_request(messages);

    println!("- Calling the echo service...");

    // Call the service
    match echo_service(request).await {
        Ok(response) => {
            println!("- Service call successful, processing responses");

            // Process the stream by collecting into a vector
            let results = stream_to_vec(response).await;

            // Display the responses
            for (i, result) in results.iter().enumerate() {
                match result {
                    Ok(resp) => {
                        println!(
                            "  Response {}: status={}, message={}",
                            i + 1,
                            resp.status,
                            resp.response
                        );
                    }
                    Err(e) => {
                        println!("  Error {}: {}", i + 1, e);
                    }
                }
            }

            println!("- Received {} total responses", results.len());
        }
        Err(status) => {
            println!("- Service call failed: {}", status);
        }
    }

    println!("Example 1 completed!");
}

// A simpler example using TestRequest/TestResponse
async fn run_test_utils_example() {
    println!("\nExample 2: Testing with TestRequest/TestResponse");

    // Create a test service that echoes messages with a counter
    async fn test_echo_service(
        request: Request<Streaming<TestRequest>>,
    ) -> Result<Response<StreamResponseInner<TestResponse>>, Status> {
        let mut request_stream = request.into_inner();
        let mut counter = 0;

        // Create response stream that echoes each request with a counter
        let response_stream = async_stream::try_stream! {
            while let Some(msg) = request_stream.message().await? {
                counter += 1;
                let id_str = String::from_utf8_lossy(&msg.id).to_string();
                let data_str = String::from_utf8_lossy(&msg.data).to_string();

                println!("  Service received message {}: id={}, data={}",
                       counter, id_str, data_str);

                // Echo message with counter
                yield TestResponse::new(
                    counter,  // Use counter as status code
                    format!("Echo #{}: id={}, data={}", counter, id_str, data_str)
                );
            }
            println!("  Service completed after processing {} messages", counter);
        };

        // Return the response without explicit casting
        Ok(Response::new(Box::pin(response_stream)))
    }

    println!("- Creating test messages");

    // Create test messages
    let test_messages = vec![
        TestRequest::new("msg1", "First message"),
        TestRequest::new("msg2", "Second message"),
        TestRequest::new("msg3", "Final message"),
    ];

    println!(
        "- Sending {} messages to the test service",
        test_messages.len()
    );

    // Create the streaming request
    let request = streaming_request(test_messages);

    // Call the service
    match test_echo_service(request).await {
        Ok(response) => {
            println!("- Service call successful, processing responses");

            // Convert the stream to a vector of results
            let results = stream_to_vec(response).await;

            // Check the results
            println!("- Received {} responses:", results.len());
            for (i, result) in results.iter().enumerate() {
                match result {
                    Ok(resp) => {
                        println!(
                            "  Response {}: code={}, message={}",
                            i + 1,
                            resp.code,
                            resp.message
                        );
                    }
                    Err(e) => {
                        println!("  Error {}: {}", i + 1, e);
                    }
                }
            }
        }
        Err(status) => {
            println!("- Service call failed: {}", status);
        }
    }

    println!("Example 2 completed!");
}

#[tokio::main]
async fn main() {
    println!("=== Bidirectional Streaming Test Examples ===");

    // Run examples
    run_direct_api_example().await;
    run_test_utils_example().await;

    println!("\nAll examples completed successfully!");
}
