#![allow(clippy::needless_range_loop)]

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use tokio::runtime::Runtime;
    use tonic::{Code, Status};
    use tonic_mock::test_utils::{
        TestRequest, TestResponse, assert_message_eq, assert_response_eq, create_stream_response,
        create_stream_response_with_errors, create_test_messages,
    };

    #[test]
    fn test_test_request_new() {
        let req = TestRequest::new("test-id", "test-data");
        assert_message_eq(&req, "test-id", "test-data");
    }

    #[test]
    fn test_test_response_new() {
        let resp = TestResponse::new(200, "OK");
        assert_response_eq(&resp, 200, "OK");
    }

    #[test]
    fn test_create_test_messages() {
        // Test with zero
        let messages = create_test_messages(0);
        assert!(messages.is_empty());

        // Test with small count
        let messages = create_test_messages(3);
        assert_eq!(messages.len(), 3);
        assert_message_eq(&messages[0], "0", "test_data_0");
        assert_message_eq(&messages[1], "1", "test_data_1");
        assert_message_eq(&messages[2], "2", "test_data_2");

        // Test with larger count
        let messages = create_test_messages(10);
        assert_eq!(messages.len(), 10);
        for i in 0..10 {
            assert_message_eq(&messages[i], i.to_string(), format!("test_data_{}", i));
        }
    }

    #[test]
    fn test_assert_message_eq() {
        let req = TestRequest::new("id-123", "data-456");

        // Test with string literals
        assert_message_eq(&req, "id-123", "data-456");

        // Test with owned String
        assert_message_eq(&req, String::from("id-123"), String::from("data-456"));

        // Test with string slices
        let id = "id-123";
        let data = "data-456";
        assert_message_eq(&req, id, data);
    }

    #[test]
    fn test_assert_response_eq() {
        let resp = TestResponse::new(404, "Not Found");

        // Test with string literals
        assert_response_eq(&resp, 404, "Not Found");

        // Test with owned String
        assert_response_eq(&resp, 404, String::from("Not Found"));

        // Test with string slices
        let message = "Not Found";
        assert_response_eq(&resp, 404, message);
    }

    #[test]
    fn test_create_stream_response() {
        let rt = Runtime::new().unwrap();

        // Create responses
        let responses = vec![
            TestResponse::new(200, "OK"),
            TestResponse::new(201, "Created"),
            TestResponse::new(202, "Accepted"),
        ];

        // Create stream response
        let stream_response = create_stream_response(responses);
        let stream = stream_response.into_inner();

        // Collect responses from stream
        let collected_responses =
            rt.block_on(async { stream.collect::<Vec<Result<TestResponse, Status>>>().await });

        // Verify responses
        assert_eq!(collected_responses.len(), 3);
        assert!(collected_responses[0].is_ok());
        assert!(collected_responses[1].is_ok());
        assert!(collected_responses[2].is_ok());

        let resp1 = collected_responses[0].as_ref().unwrap();
        let resp2 = collected_responses[1].as_ref().unwrap();
        let resp3 = collected_responses[2].as_ref().unwrap();

        assert_response_eq(resp1, 200, "OK");
        assert_response_eq(resp2, 201, "Created");
        assert_response_eq(resp3, 202, "Accepted");
    }

    #[test]
    fn test_create_stream_response_with_errors() {
        let rt = Runtime::new().unwrap();

        // Create responses
        let responses = vec![
            TestResponse::new(200, "OK"),
            TestResponse::new(201, "Created"),
            TestResponse::new(202, "Accepted"),
            TestResponse::new(203, "Non-Authoritative Information"),
        ];

        // Create error status
        let error_status = Status::new(Code::Internal, "Test error");

        // Create stream with errors at indices 1 and 3
        let stream_response =
            create_stream_response_with_errors(responses, vec![1, 3], error_status.clone());

        let stream = stream_response.into_inner();

        // Collect responses from stream
        let collected_responses =
            rt.block_on(async { stream.collect::<Vec<Result<TestResponse, Status>>>().await });

        // Verify responses
        assert_eq!(collected_responses.len(), 2); // Only 2 because the 2nd error halts the stream

        // First should be OK
        assert!(collected_responses[0].is_ok());
        let resp1 = collected_responses[0].as_ref().unwrap();
        assert_response_eq(resp1, 200, "OK");

        // Second should be an error
        assert!(collected_responses[1].is_err());
        let err = collected_responses[1].as_ref().unwrap_err();
        assert_eq!(err.code(), Code::Internal);
        assert_eq!(err.message(), "Test error");

        // Empty error indices should have no errors
        let stream_response = create_stream_response_with_errors(
            vec![TestResponse::new(200, "OK")],
            vec![],
            error_status,
        );

        let stream = stream_response.into_inner();

        let collected_responses =
            rt.block_on(async { stream.collect::<Vec<Result<TestResponse, Status>>>().await });

        assert_eq!(collected_responses.len(), 1);
        assert!(collected_responses[0].is_ok());
    }

    #[test]
    fn test_create_stream_response_empty() {
        let rt = Runtime::new().unwrap();

        // Create empty responses
        let responses: Vec<TestResponse> = vec![];

        // Create stream response
        let stream_response = create_stream_response(responses);
        let stream = stream_response.into_inner();

        // Collect responses from stream
        let collected_responses =
            rt.block_on(async { stream.collect::<Vec<Result<TestResponse, Status>>>().await });

        // Verify responses
        assert!(collected_responses.is_empty());
    }
}
