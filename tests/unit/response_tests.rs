#[cfg(test)]
mod tests {
    use crate::common::{TestResponse, test_utils};
    use std::sync::{Arc, Mutex};
    use tokio::runtime::Runtime;
    use tonic::{Code, Status};
    use tonic_mock::{process_streaming_response, stream_to_vec};

    #[test]
    fn test_process_streaming_response() {
        let rt = Runtime::new().unwrap();

        // Create a vector of test responses
        let responses = vec![
            TestResponse::new(0, "Response 0"),
            TestResponse::new(1, "Response 1"),
            TestResponse::new(2, "Response 2"),
        ];

        // Create a streaming response
        let stream_response = test_utils::create_stream_response(responses);

        // Process the streaming response with a callback
        let processed_msgs = Arc::new(Mutex::new(Vec::new()));
        let processed_indexes = Arc::new(Mutex::new(Vec::new()));

        {
            let processed_msgs = processed_msgs.clone();
            let processed_indexes = processed_indexes.clone();

            rt.block_on(async {
                process_streaming_response(stream_response, |msg, idx| {
                    assert!(msg.is_ok());
                    if let Ok(msg) = msg {
                        processed_msgs.lock().unwrap().push(msg.clone());
                        processed_indexes.lock().unwrap().push(idx);
                    }
                })
                .await;
            });
        }

        // Verify the results
        let processed_msgs = processed_msgs.lock().unwrap();
        let processed_indexes = processed_indexes.lock().unwrap();

        assert_eq!(processed_msgs.len(), 3);
        assert_eq!(processed_indexes.len(), 3);

        for i in 0..3 {
            assert_eq!(processed_msgs[i].code, i as i32);
            assert_eq!(processed_msgs[i].message, format!("Response {}", i));
            assert_eq!(processed_indexes[i], i);
        }
    }

    #[test]
    fn test_process_streaming_response_with_errors() {
        let rt = Runtime::new().unwrap();

        // Create a vector of test responses
        let responses = vec![
            TestResponse::new(0, "Response 0"),
            TestResponse::new(1, "Response 1"),
            TestResponse::new(2, "Response 2"),
            TestResponse::new(3, "Response 3"),
            TestResponse::new(4, "Response 4"),
        ];

        // Create error status
        let error_status = Status::new(Code::Internal, "Test error");

        // Create a streaming response with errors at indices 1 and 3
        let stream_response = test_utils::create_stream_response_with_errors(
            responses,
            vec![1, 3],
            error_status.clone(),
        );

        // Process the streaming response with a callback
        let success_msgs = Arc::new(Mutex::new(Vec::new()));
        let error_indices = Arc::new(Mutex::new(Vec::new()));

        {
            let success_msgs = success_msgs.clone();
            let error_indices = error_indices.clone();

            rt.block_on(async {
                process_streaming_response(stream_response, |msg, idx| {
                    if let Ok(msg) = msg {
                        success_msgs.lock().unwrap().push(msg.clone());
                    } else {
                        error_indices.lock().unwrap().push(idx);
                    }
                })
                .await;
            });
        }

        // Verify the results
        let success_msgs = success_msgs.lock().unwrap();
        let error_indices = error_indices.lock().unwrap();

        // Due to how the stream terminates on errors, we get fewer results than expected
        assert_eq!(success_msgs.len(), 1); // Should have 1 successful message (just the first one)
        assert_eq!(error_indices.len(), 1); // Should have 1 error index (first error encountered)

        // Check success messages
        assert_eq!(success_msgs[0].code, 0);

        // Check error indices
        assert_eq!(error_indices[0], 1);
    }

    #[test]
    fn test_stream_to_vec() {
        let rt = Runtime::new().unwrap();

        // Create a vector of test responses
        let responses = vec![
            TestResponse::new(0, "Response 0"),
            TestResponse::new(1, "Response 1"),
            TestResponse::new(2, "Response 2"),
        ];

        // Create a streaming response
        let stream_response = test_utils::create_stream_response(responses);

        // Convert the stream to a vector
        let result = rt.block_on(async { stream_to_vec(stream_response).await });

        // Verify the results
        assert_eq!(result.len(), 3);

        #[allow(clippy::needless_range_loop)]
        for i in 0..3 {
            assert!(result[i].is_ok());
            let response = result[i].as_ref().unwrap();
            assert_eq!(response.code, i as i32);
            assert_eq!(response.message, format!("Response {}", i));
        }
    }

    #[test]
    fn test_stream_to_vec_with_errors() {
        let rt = Runtime::new().unwrap();

        // Create a vector of test responses
        let responses = vec![
            TestResponse::new(0, "Response 0"),
            TestResponse::new(1, "Response 1"),
            TestResponse::new(2, "Response 2"),
        ];

        // Create error status
        let error_status = Status::new(Code::Internal, "Test error");

        // Create a streaming response with an error at index 1
        let stream_response = test_utils::create_stream_response_with_errors(
            responses,
            vec![1],
            error_status.clone(),
        );

        // Convert the stream to a vector
        let result = rt.block_on(async { stream_to_vec(stream_response).await });

        // Verify the results - due to how errors are handled, we only get the items before the error
        assert_eq!(result.len(), 2); // Should have 2 items: 1 success and 1 error

        // Check success and error messages
        assert!(result[0].is_ok());
        assert!(result[1].is_err());

        assert_eq!(result[0].as_ref().unwrap().code, 0);
        assert_eq!(result[0].as_ref().unwrap().message, "Response 0");

        assert_eq!(result[1].as_ref().err().unwrap().code(), Code::Internal);
        assert_eq!(result[1].as_ref().err().unwrap().message(), "Test error");
    }
}
