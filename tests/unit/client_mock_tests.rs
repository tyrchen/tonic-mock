#[cfg(test)]
mod tests {
    use tonic::{
        Code, Request, Response, Status,
        metadata::{Ascii, MetadataKey},
    };
    use tonic_mock::{
        client_mock::{GrpcClientExt, MockResponseDefinition, MockableGrpcClient},
        test_utils::{TestRequest, TestResponse},
    };

    /// Example gRPC client that we'll mock for testing
    #[derive(Debug, Clone)]
    struct ExampleServiceClient<T> {
        inner: T,
    }

    // Implementation of the GrpcClientExt trait for our example client
    impl GrpcClientExt<ExampleServiceClient<MockableGrpcClient>>
        for ExampleServiceClient<MockableGrpcClient>
    {
        fn with_mock(mock: MockableGrpcClient) -> Self {
            Self { inner: mock }
        }
    }

    // Instead of trying to implement a full Tonic client, create a simplified one
    // that directly uses our mock infrastructure for testing
    impl ExampleServiceClient<MockableGrpcClient> {
        pub async fn get_data(
            &mut self,
            request: Request<TestRequest>,
        ) -> Result<Response<TestResponse>, Status> {
            // Extract the request data
            let request_data = request.into_inner();

            // Encode the request
            let encoded = tonic_mock::grpc_mock::encode_grpc_request(request_data);

            // Call the mock service
            let (response_bytes, http_metadata) = self
                .inner
                .handle_request("example.TestService", "GetData", &encoded)
                .await?;

            // Decode the response
            let response: TestResponse =
                tonic_mock::grpc_mock::decode_grpc_message(&response_bytes)?;

            // Create a new response and return
            let mut tonic_response = Response::new(response);

            // Copy HTTP headers to gRPC metadata
            for (name, value) in http_metadata.into_iter() {
                // Convert the HTTP header name to a string we can use
                if let Some(key) = name {
                    let key_str = key.as_str();
                    // Skip mock-specific headers
                    if !key_str.starts_with("mock-") {
                        if let Ok(val_str) = value.to_str() {
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

        pub async fn process_data(
            &mut self,
            request: Request<TestRequest>,
        ) -> Result<Response<TestResponse>, Status> {
            // Extract the request data
            let request_data = request.into_inner();

            // Encode the request
            let encoded = tonic_mock::grpc_mock::encode_grpc_request(request_data);

            // Call the mock service
            let (response_bytes, http_metadata) = self
                .inner
                .handle_request("example.TestService", "ProcessData", &encoded)
                .await?;

            // Decode the response
            let response: TestResponse =
                tonic_mock::grpc_mock::decode_grpc_message(&response_bytes)?;

            // Create a new response and return
            let mut tonic_response = Response::new(response);

            // Copy HTTP headers to gRPC metadata
            for (name, value) in http_metadata.into_iter() {
                // Convert the HTTP header name to a string we can use
                if let Some(key) = name {
                    let key_str = key.as_str();
                    // Skip mock-specific headers
                    if !key_str.starts_with("mock-") {
                        if let Ok(val_str) = value.to_str() {
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

    #[tokio::test]
    async fn test_mockable_client_basic() {
        // Create a mock client
        let mock = MockableGrpcClient::new();

        // Configure mock responses
        mock.mock::<TestRequest, TestResponse>("example.TestService", "GetData")
            .respond_with(MockResponseDefinition::ok(TestResponse::new(
                200,
                "Mock response",
            )))
            .await;

        // Create a client that uses the mock
        let mut client = ExampleServiceClient::with_mock(mock);

        // Make a request
        let request = TestRequest::new("test-id", "test-data");
        let response = client.get_data(Request::new(request)).await.unwrap();

        // Verify the response
        assert_eq!(response.get_ref().code, 200);
        assert_eq!(response.get_ref().message, "Mock response");
    }

    #[tokio::test]
    async fn test_mockable_client_error_response() {
        // Create a mock client
        let mock = MockableGrpcClient::new();

        // Configure mock to return an error
        mock.mock::<TestRequest, TestResponse>("example.TestService", "GetData")
            .respond_with(MockResponseDefinition::err(Status::new(
                Code::InvalidArgument,
                "Invalid request",
            )))
            .await;

        // Create a client that uses the mock
        let mut client = ExampleServiceClient::with_mock(mock);

        // Make a request
        let request = TestRequest::new("test-id", "test-data");
        let result = client.get_data(Request::new(request)).await;

        // Verify the error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(err.message(), "Invalid request");
    }

    #[tokio::test]
    async fn test_mockable_client_conditional_response() {
        // Create a mock client
        let mock = MockableGrpcClient::new();

        // Configure different responses based on request content
        mock.mock::<TestRequest, TestResponse>("example.TestService", "ProcessData")
            .respond_when(
                |req| String::from_utf8_lossy(&req.id) == "valid-id",
                MockResponseDefinition::ok(TestResponse::new(200, "Valid ID")),
            )
            .await
            .respond_when(
                |req| String::from_utf8_lossy(&req.id) == "invalid-id",
                MockResponseDefinition::err(Status::new(Code::InvalidArgument, "Invalid ID")),
            )
            .await;

        // Create a client that uses the mock
        let mut client = ExampleServiceClient::with_mock(mock);

        // Test with valid ID
        let valid_request = TestRequest::new("valid-id", "test-data");
        let valid_response = client
            .process_data(Request::new(valid_request))
            .await
            .unwrap();
        assert_eq!(valid_response.get_ref().code, 200);
        assert_eq!(valid_response.get_ref().message, "Valid ID");

        // Test with invalid ID
        let invalid_request = TestRequest::new("invalid-id", "test-data");
        let invalid_result = client.process_data(Request::new(invalid_request)).await;
        assert!(invalid_result.is_err());
        let err = invalid_result.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(err.message(), "Invalid ID");
    }

    #[tokio::test]
    async fn test_mockable_client_with_metadata() {
        // Create a mock client
        let mock = MockableGrpcClient::new();

        // Configure response with metadata
        mock.mock::<TestRequest, TestResponse>("example.TestService", "GetData")
            .respond_with(
                MockResponseDefinition::ok(TestResponse::new(200, "Response with metadata"))
                    .with_metadata("x-test-header", "test-value"),
            )
            .await;

        // Create a client that uses the mock
        let mut client = ExampleServiceClient::with_mock(mock);

        // Make a request
        let request = TestRequest::new("test-id", "test-data");
        let response = client.get_data(Request::new(request)).await.unwrap();

        // Verify the response and metadata
        assert_eq!(response.get_ref().code, 200);
        assert_eq!(response.get_ref().message, "Response with metadata");
        assert_eq!(
            response.metadata().get("x-test-header").unwrap(),
            "test-value"
        );
    }

    #[tokio::test]
    async fn test_mockable_client_method_not_mocked() {
        // Create a mock client
        let mock = MockableGrpcClient::new();

        // Only configure the GetData method
        mock.mock::<TestRequest, TestResponse>("example.TestService", "GetData")
            .respond_with(MockResponseDefinition::ok(TestResponse::new(200, "OK")))
            .await;

        // Create a client that uses the mock
        let mut client = ExampleServiceClient::with_mock(mock);

        // Call the ProcessData method which is not mocked
        let request = TestRequest::new("test-id", "test-data");
        let result = client.process_data(Request::new(request)).await;

        // Should return Unimplemented error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_mockable_client_reset() {
        // Create a mock client
        let mock = MockableGrpcClient::new();

        // Configure a response
        mock.mock::<TestRequest, TestResponse>("example.TestService", "GetData")
            .respond_with(MockResponseDefinition::ok(TestResponse::new(200, "OK")))
            .await;

        // Create a client that uses the mock
        let mut client = ExampleServiceClient::with_mock(mock.clone());

        // First call should succeed
        let request = TestRequest::new("test-id", "test-data");
        let result1 = client.get_data(Request::new(request.clone())).await;
        assert!(result1.is_ok());

        // Reset the mock
        mock.reset().await;

        // Second call should fail with Unimplemented error
        let result2 = client.get_data(Request::new(request)).await;
        assert!(result2.is_err());
        assert_eq!(result2.unwrap_err().code(), Code::Unimplemented);
    }
}
