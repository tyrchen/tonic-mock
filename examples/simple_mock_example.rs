use prost::Message;
use tonic::{
    Code, Request, Status,
    metadata::{Ascii, MetadataKey},
};
use tonic_mock::client_mock::{GrpcClientExt, MockResponseDefinition, MockableGrpcClient};

/// Simple request message
#[derive(Clone, PartialEq, Message)]
pub struct SimpleRequest {
    #[prost(string, tag = "1")]
    pub value: String,
}

/// Simple response message
#[derive(Clone, PartialEq, Message)]
pub struct SimpleResponse {
    #[prost(string, tag = "1")]
    pub result: String,
    #[prost(int32, tag = "2")]
    pub status_code: i32,
}

/// Client for the SimpleService
#[derive(Debug, Clone)]
pub struct SimpleServiceClient<T> {
    inner: T,
}

// Implement the GrpcClientExt trait for our service client
impl GrpcClientExt<SimpleServiceClient<MockableGrpcClient>>
    for SimpleServiceClient<MockableGrpcClient>
{
    fn with_mock(mock: MockableGrpcClient) -> Self {
        Self { inner: mock }
    }
}

// Implement a basic method for our client that uses the mock
impl SimpleServiceClient<MockableGrpcClient> {
    pub async fn process(
        &mut self,
        request: Request<SimpleRequest>,
    ) -> Result<tonic::Response<SimpleResponse>, Status> {
        // Extract the inner client
        let client = &self.inner;

        // Get the request data and print for debugging
        let request_data = request.into_inner();
        println!("Sending request with value: {}", request_data.value);

        // Encode the request using tonic-mock's helpers
        let encoded = tonic_mock::grpc_mock::encode_grpc_request(request_data);

        // Call the mock service
        let (response_bytes, http_metadata) = client
            .handle_request("simple.SimpleService", "Process", &encoded)
            .await?;

        // Decode the response using tonic-mock's helpers
        let response: SimpleResponse = tonic_mock::grpc_mock::decode_grpc_message(&response_bytes)?;

        // Create a new response and return
        let mut tonic_response = tonic::Response::new(response);

        // Copy any metadata from HTTP headers to gRPC metadata
        for (key, value) in http_metadata.into_iter() {
            if let Some(key) = key {
                if let Ok(val_str) = value.to_str() {
                    let key_str = key.as_str();
                    // Skip the mock-specific headers like mock-delay-ms
                    if !key_str.starts_with("mock-") {
                        if let Ok(metadata_value) = val_str.parse() {
                            tonic_response.metadata_mut().insert(
                                key_str.parse::<MetadataKey<Ascii>>().unwrap(),
                                metadata_value,
                            );
                        }
                    }
                }
            }
        }

        Ok(tonic_response)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple Mock Example ===\n");

    // 1. Create a mock and configure a response
    let mock = MockableGrpcClient::new();
    mock.mock::<SimpleRequest, SimpleResponse>("simple.SimpleService", "Process")
        .respond_with(
            MockResponseDefinition::ok(SimpleResponse {
                result: "Success!".to_string(),
                status_code: 200,
            })
            .with_metadata("x-request-id", "12345")
            .with_metadata("content-type", "application/grpc+proto"),
        )
        .await;

    // 2. Create a client with the mock
    let mut client = SimpleServiceClient::with_mock(mock);

    // 3. Make a request
    let request = SimpleRequest {
        value: "test-input".to_string(),
    };

    // 4. Call the service
    let response = client.process(Request::new(request)).await?;

    // 5. Check the result
    println!(
        "Response: {} (code: {})",
        response.get_ref().result,
        response.get_ref().status_code
    );
    println!("Metadata: {:?}", response.metadata());

    // Example 2: Predicate-based response
    println!("\n=== Conditional Response Example ===\n");

    // Create a new mock
    let mock = MockableGrpcClient::new();

    // Configure responses - first add the specific cases
    mock.mock::<SimpleRequest, SimpleResponse>("simple.SimpleService", "Process")
        .respond_when(
            |req| req.value == "invalid",
            MockResponseDefinition::err(Status::new(
                Code::InvalidArgument,
                "Invalid request value",
            )),
        )
        .await
        .respond_when(
            |req| req.value == "premium",
            MockResponseDefinition::ok(SimpleResponse {
                result: "Premium service!".to_string(),
                status_code: 200,
            })
            .with_metadata("service-tier", "premium")
            .with_delay(100), // Simulate a network delay of 100ms
        )
        .await
        .respond_with(
            // This is the fallback for any non-matching request
            MockResponseDefinition::ok(SimpleResponse {
                result: "Standard service".to_string(),
                status_code: 200,
            })
            .with_metadata("service-tier", "standard"),
        )
        .await;

    // Create a client
    let mut client = SimpleServiceClient::with_mock(mock);

    // Test different request types
    let test_values = vec!["standard", "premium", "invalid"];

    for value in test_values {
        let request = SimpleRequest {
            value: value.to_string(),
        };

        println!("\nTesting request with value: {}", value);
        match client.process(Request::new(request)).await {
            Ok(response) => {
                println!(
                    "Success: {} (code: {})",
                    response.get_ref().result,
                    response.get_ref().status_code
                );
                println!("Metadata: {:?}", response.metadata());
            }
            Err(status) => {
                println!("Error: {} (code: {:?})", status.message(), status.code());
            }
        }
    }

    // Example 3: Resetting and reconfiguring mocks
    println!("\n=== Reset and Reconfigure Example ===\n");

    let mock = MockableGrpcClient::new();

    // Configure a response
    mock.mock::<SimpleRequest, SimpleResponse>("simple.SimpleService", "Process")
        .respond_with(MockResponseDefinition::ok(SimpleResponse {
            result: "Initial configuration".to_string(),
            status_code: 200,
        }))
        .await;

    let mut client = SimpleServiceClient::with_mock(mock.clone());

    // Make a request with the initial configuration
    let request = SimpleRequest {
        value: "test".to_string(),
    };

    let response = client.process(Request::new(request.clone())).await?;
    println!("Initial response: {}", response.get_ref().result);

    // Reset and reconfigure
    mock.reset().await;
    mock.mock::<SimpleRequest, SimpleResponse>("simple.SimpleService", "Process")
        .respond_with(MockResponseDefinition::ok(SimpleResponse {
            result: "After reset configuration".to_string(),
            status_code: 201,
        }))
        .await;

    // Make another request with new configuration
    let response = client.process(Request::new(request)).await?;
    println!(
        "After reset response: {} (code: {})",
        response.get_ref().result,
        response.get_ref().status_code
    );

    Ok(())
}
