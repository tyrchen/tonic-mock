#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use tonic::{Code, Status};
    use tonic_mock::{
        grpc_mock::{
            create_grpc_uri, decode_grpc_message, encode_grpc_request, encode_grpc_response,
            mock_grpc_call,
        },
        test_utils::{TestRequest, TestResponse},
    };

    #[test]
    fn test_encode_grpc_request() {
        let request = TestRequest::new("test-id", "test-data");
        let encoded = encode_grpc_request(request);

        // Verify the encoded data format (5 byte header + payload)
        assert!(encoded.len() > 5);
        assert_eq!(encoded[0], 0); // No compression

        // Decode to verify
        let decoded: TestRequest = decode_grpc_message(&encoded).unwrap();
        assert_eq!(decoded.id, "test-id".as_bytes());
        assert_eq!(decoded.data, "test-data".as_bytes());
    }

    #[test]
    fn test_encode_grpc_response() {
        let response = TestResponse::new(200, "Test Response");
        let encoded = encode_grpc_response(response);

        // Verify the encoded data format (5 byte header + payload)
        assert!(encoded.len() > 5);
        assert_eq!(encoded[0], 0); // No compression
    }

    #[test]
    fn test_create_grpc_uri() {
        let uri = create_grpc_uri("example.TestService", "TestMethod");

        assert_eq!(uri.path(), "/example.TestService/TestMethod");
        assert_eq!(uri.scheme().unwrap(), "http");
        assert_eq!(uri.authority().unwrap(), "localhost");
    }

    #[test]
    fn test_decode_grpc_message() {
        // Create a request
        let original_request = TestRequest::new("test-id", "test-data");

        // Encode the request
        let encoded = encode_grpc_request(original_request.clone());

        // Decode message from encoded data
        let decoded_request: TestRequest = decode_grpc_message(&encoded).unwrap();

        // Verify decoded message matches original
        assert_eq!(decoded_request.id, original_request.id);
        assert_eq!(decoded_request.data, original_request.data);
    }

    #[test]
    fn test_decode_grpc_message_too_short() {
        // Create a message that's too short (less than 5 bytes)
        let bytes = Bytes::from_static(&[0, 0, 0]);

        // Try to decode, should return error
        let result: Result<TestRequest, Status> = decode_grpc_message(&bytes);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), Code::InvalidArgument);
    }

    #[test]
    fn test_decode_grpc_message_compression_not_supported() {
        // Create a message with compression flag set to 1 (not supported)
        let bytes = Bytes::from_static(&[1, 0, 0, 0, 0]);

        // Try to decode, should return error about compression
        let result: Result<TestRequest, Status> = decode_grpc_message(&bytes);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), Code::Unimplemented);
    }

    #[test]
    fn test_decode_grpc_message_incomplete() {
        // Create a header indicating 10 bytes of data but provide only 5
        let bytes = Bytes::from_static(&[0, 0, 0, 0, 10, 1, 2, 3, 4, 5]);

        // Try to decode, should return error about incomplete message
        let result: Result<TestRequest, Status> = decode_grpc_message(&bytes);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), Code::InvalidArgument);
    }

    #[test]
    fn test_mock_grpc_call() {
        // Create a request
        let request = TestRequest::new("test-id", "test-data");

        // Mock a gRPC call
        let response = mock_grpc_call(
            "example.TestService",
            "TestMethod",
            request,
            |req: TestRequest| {
                // Handler that echoes back the request ID in the response message
                let id_str = String::from_utf8_lossy(&req.id).to_string();
                Ok(TestResponse::new(200, format!("Processed: {}", id_str)))
            },
        )
        .unwrap();

        // Verify response
        assert_eq!(response.code, 200);
        assert_eq!(response.message, "Processed: test-id");
    }

    #[test]
    fn test_mock_grpc_call_with_error() {
        // Create a request
        let request = TestRequest::new("invalid", "data");

        // Mock a gRPC call that returns an error
        let result = mock_grpc_call(
            "example.TestService",
            "TestMethod",
            request,
            |req: TestRequest| {
                // Handler that checks the request and returns an error
                let id_str = String::from_utf8_lossy(&req.id).to_string();
                if id_str == "invalid" {
                    Err(Status::new(Code::InvalidArgument, "Invalid request ID"))
                } else {
                    Ok(TestResponse::new(200, "OK"))
                }
            },
        );

        // Verify error
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), Code::InvalidArgument);
    }
}
