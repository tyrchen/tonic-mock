#![allow(clippy::result_large_err)]
use std::error::Error;
use tonic::{Code, Status};
use tonic_mock::{
    grpc_mock::{
        create_grpc_uri, decode_grpc_message, encode_grpc_request, encode_grpc_response,
        mock_grpc_call,
    },
    test_utils::{TestRequest, TestResponse},
};

// Define a simple Echo service interface
trait EchoService {
    fn echo(&self, request: TestRequest) -> Result<TestResponse, Status>;
}

// Implement a mock Echo service
struct MockEchoService;

impl EchoService for MockEchoService {
    fn echo(&self, request: TestRequest) -> Result<TestResponse, Status> {
        // Extract the request ID
        let id_str = String::from_utf8_lossy(&request.id).to_string();

        // Check if the ID contains "error" to simulate an error case
        if id_str.contains("error") {
            return Err(Status::new(
                Code::InvalidArgument,
                "Invalid ID: contains 'error'",
            ));
        }

        // Extract the request data
        let data_str = String::from_utf8_lossy(&request.data).to_string();

        // Create a response with the echoed data
        Ok(TestResponse::new(
            200,
            format!("Echo: {} - {}", id_str, data_str),
        ))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== gRPC Mock Example ===\n");

    // Example 1: Manual request/response encoding
    manual_request_response_example()?;

    // Example 2: Using the mock_grpc_call helper
    mock_grpc_call_example()?;

    // Example 3: Error handling example
    error_handling_example()?;

    println!("\nAll examples completed successfully!");
    Ok(())
}

// Example 1: Manually creating and processing gRPC encoded messages
fn manual_request_response_example() -> Result<(), Box<dyn Error>> {
    println!("Example 1: Manual request/response encoding");

    // Create a request
    let request = TestRequest::new("request-1", "Hello, gRPC!");
    println!("- Creating request with ID: 'request-1'");

    // Create a gRPC URI
    let uri = create_grpc_uri("echo.EchoService", "Echo");
    println!("- Created URI: {}", uri);

    // Encode the request
    let encoded_request = encode_grpc_request(request);
    println!("- Encoded request to {} bytes", encoded_request.len());

    // Decode the gRPC message
    let decoded_request: TestRequest = decode_grpc_message(&encoded_request)?;
    println!(
        "- Decoded request message: id={}, data={}",
        String::from_utf8_lossy(&decoded_request.id),
        String::from_utf8_lossy(&decoded_request.data)
    );

    // Create a mock service and process the request
    let service = MockEchoService;
    let response = service.echo(decoded_request)?;
    println!("- Service generated response: {}", response.message);

    // Encode the response
    let encoded_response = encode_grpc_response(response);
    println!("- Encoded response to {} bytes", encoded_response.len());

    println!("- Example 1 completed\n");
    Ok(())
}

// Example 2: Using the mock_grpc_call helper function
fn mock_grpc_call_example() -> Result<(), Box<dyn Error>> {
    println!("Example 2: Using mock_grpc_call helper");

    // Create a request
    let request = TestRequest::new("request-2", "Testing mock_grpc_call");
    println!("- Creating request with ID: 'request-2'");

    // Use the mock_grpc_call helper to simulate a gRPC call
    let response = mock_grpc_call("echo.EchoService", "Echo", request, |req: TestRequest| {
        // This mimics the service handler
        println!(
            "- Handler received request: id={}, data={}",
            String::from_utf8_lossy(&req.id),
            String::from_utf8_lossy(&req.data)
        );

        // Process the request and return a response
        let id_str = String::from_utf8_lossy(&req.id).to_string();
        let data_str = String::from_utf8_lossy(&req.data).to_string();
        Ok(TestResponse::new(
            200,
            format!("Processed: {} - {}", id_str, data_str),
        ))
    })?;

    println!(
        "- Received response: code={}, message={}",
        response.code, response.message
    );
    println!("- Example 2 completed\n");
    Ok(())
}

// Example 3: Error handling
fn error_handling_example() -> Result<(), Box<dyn Error>> {
    println!("Example 3: Error handling");

    // Create a request that will trigger an error
    let request = TestRequest::new("error-id", "This will cause an error");
    println!("- Creating request with ID: 'error-id'");

    // Attempt to call the service using the mock_grpc_call helper
    let result = mock_grpc_call("echo.EchoService", "Echo", request, |req: TestRequest| {
        // Use our service implementation
        let service = MockEchoService;
        service.echo(req)
    });

    // Check the result
    match result {
        Ok(response) => {
            println!("- Unexpected success: {}", response.message);
            Ok(())
        }
        Err(status) => {
            println!(
                "- Received expected error: {} - {}",
                status.code(),
                status.message()
            );
            println!("- Example 3 completed");
            Ok(())
        }
    }
}
