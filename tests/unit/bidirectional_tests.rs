#![allow(clippy::needless_range_loop)]

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };
    use tokio::runtime::Runtime;
    use tonic::{Request, Response, Status, Streaming};
    use tonic_mock::{
        BidirectionalStreamingTest, StreamResponseInner, streaming_request,
        test_utils::{TestRequest, TestResponse},
    };

    // Create a simple bidirectional service for testing
    async fn echo_service(
        request: Request<Streaming<TestRequest>>,
    ) -> Result<Response<StreamResponseInner<TestResponse>>, Status> {
        let mut in_stream = request.into_inner();

        // Create counter to keep track of requests
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Create response stream
        let out_stream = async_stream::try_stream! {
            while let Some(msg) = in_stream.message().await? {
                let count = counter_clone.fetch_add(1, Ordering::SeqCst);

                // Create response based on request
                let id_str = String::from_utf8_lossy(&msg.id).to_string();
                let data_str = String::from_utf8_lossy(&msg.data).to_string();

                let response = TestResponse::new(
                    200,
                    format!("Echo #{}: id={}, data={}", count, id_str, data_str)
                );

                yield response;
            }
        };

        Ok(Response::new(Box::pin(out_stream)))
    }

    // Test simple bidirectional streaming with streaming_request
    #[test]
    fn test_bidirectional_streaming() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create test messages
            let messages = vec![
                TestRequest::new("id1", "data1"),
                TestRequest::new("id2", "data2"),
                TestRequest::new("id3", "data3"),
            ];

            // Create streaming request
            let request = streaming_request(messages);

            // Call service
            let response = echo_service(request).await.unwrap();
            let response_stream = response.into_inner();

            // Collect all responses
            let responses: Vec<_> = response_stream
                .collect::<Vec<Result<TestResponse, Status>>>()
                .await;

            // Verify responses
            assert_eq!(responses.len(), 3);

            // Check first response
            let first = &responses[0].as_ref().unwrap();
            assert_eq!(first.code, 200);
            assert!(first.message.contains("id=id1"));
            assert!(first.message.contains("data=data1"));

            // Check second response
            let second = &responses[1].as_ref().unwrap();
            assert_eq!(second.code, 200);
            assert!(second.message.contains("id=id2"));
            assert!(second.message.contains("data=data2"));

            // Check third response
            let third = &responses[2].as_ref().unwrap();
            assert_eq!(third.code, 200);
            assert!(third.message.contains("id=id3"));
            assert!(third.message.contains("data=data3"));
        });
    }

    // Test bidirectional streaming with delayed responses
    #[test]
    fn test_bidirectional_streaming_with_delay() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create delayed echo service
            let delayed_echo = |req: Request<Streaming<TestRequest>>| async move {
                let mut in_stream = req.into_inner();

                // Create response stream with explicit type annotation for the stream
                // to resolve the type inference issue
                let stream = async_stream::try_stream! {
                    while let Some(msg) = in_stream.message().await? {
                        // Add artificial delay
                        tokio::time::sleep(Duration::from_millis(50)).await;

                        let id_str = String::from_utf8_lossy(&msg.id).to_string();
                        let data_str = String::from_utf8_lossy(&msg.data).to_string();

                        let response = TestResponse::new(
                            200,
                            format!("Delayed echo: id={}, data={}", id_str, data_str)
                        );

                        yield response;
                    }
                };

                // Explicitly annotate the Box::pin operation to help type inference
                Ok::<Response<StreamResponseInner<TestResponse>>, Status>(Response::new(Box::pin(
                    stream,
                )))
            };

            // Create test messages
            let messages = vec![
                TestRequest::new("id1", "data1"),
                TestRequest::new("id2", "data2"),
            ];

            // Create streaming request
            let request = streaming_request(messages);

            // Call service
            let response = delayed_echo(request).await.unwrap();
            let mut response_stream = response.into_inner();

            // Get first response with timeout
            let first_result =
                tokio::time::timeout(Duration::from_millis(100), response_stream.next()).await;

            assert!(first_result.is_ok());
            let first = first_result.unwrap().unwrap().unwrap();
            assert_eq!(first.code, 200);
            assert!(first.message.contains("id=id1"));

            // Get second response
            let second = response_stream.next().await.unwrap().unwrap();
            assert_eq!(second.code, 200);
            assert!(second.message.contains("id=id2"));

            // No more responses
            assert!(response_stream.next().await.is_none());
        });
    }

    // Test using the BidirectionalStreamingTest utility
    #[test]
    fn test_bidirectional_streaming_test_utility() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Simple echo service that just echoes back the message with a code of 200
            async fn simple_echo_service(
                request: Request<Streaming<TestRequest>>,
            ) -> Result<Response<StreamResponseInner<TestResponse>>, Status> {
                let mut stream = request.into_inner();

                let response_stream = async_stream::try_stream! {
                    while let Some(msg) = stream.message().await? {
                        // Create a simple response without delay
                        let id_str = String::from_utf8_lossy(&msg.id).to_string();
                        yield TestResponse::new(
                            200,
                            format!("Echo: {}", id_str)
                        );
                    }
                };

                Ok(Response::new(Box::pin(response_stream)))
            }

            // Create a test context
            let mut test = BidirectionalStreamingTest::new(simple_echo_service);

            // Send a message
            test.send_client_message(TestRequest::new("test_id", "test_data"))
                .await;

            // Get response with a timeout BEFORE completion
            match test
                .get_server_response_with_timeout(Duration::from_secs(1))
                .await
            {
                Ok(Some(resp)) => {
                    assert_eq!(resp.code, 200);
                    assert!(resp.message.contains("Echo: test_id"));
                }
                Ok(None) => panic!("Expected a response but got None"),
                Err(status) => panic!("Got error: {}", status),
            }

            // Now complete the test
            test.complete().await;
        });
    }

    #[tokio::test]
    async fn test_bidirectional_streaming_dispose() {
        // Create a test context
        let mut test = BidirectionalStreamingTest::new(echo_service);

        // Send a message
        test.send_client_message(TestRequest::new("dispose-test", "data"))
            .await;

        // Explicitly dispose of the test
        test.dispose();

        // Verify that getting responses after dispose returns None
        let response = test.get_server_response().await;
        assert!(response.is_none(), "Expected None response after dispose");

        // Try with timeout as well
        let response = test
            .get_server_response_with_timeout(Duration::from_millis(50))
            .await;
        assert!(
            matches!(response, Ok(None)),
            "Expected Ok(None) response after dispose"
        );
    }

    #[tokio::test]
    async fn test_bidirectional_streaming_complete_idempotent() {
        // Create a test context
        let mut test = BidirectionalStreamingTest::new(echo_service);

        // Send a message
        test.send_client_message(TestRequest::new("complete-test", "data"))
            .await;

        // Call complete multiple times (should be idempotent)
        test.complete().await;
        test.complete().await; // Second call should be a no-op

        // We should still be able to get the response
        let response = test.get_server_response().await;
        assert!(
            response.is_some(),
            "Expected a response after multiple complete calls"
        );
    }

    #[tokio::test]
    async fn test_bidirectional_streaming_service_error() {
        // Create a service that returns an error
        async fn error_service(
            _request: Request<Streaming<TestRequest>>,
        ) -> Result<Response<StreamResponseInner<TestResponse>>, Status> {
            Err(Status::internal("Test error"))
        }

        // Create a test context with the error service
        let mut test = BidirectionalStreamingTest::<TestRequest, TestResponse>::new(error_service);

        // Send a message
        test.send_client_message(TestRequest::new("error-test", "data"))
            .await;

        // Complete the test
        test.complete().await;

        // We should get None because the service returned an error
        let response = test.get_server_response().await;
        assert!(
            response.is_none(),
            "Expected None response from error service"
        );
    }

    #[tokio::test]
    async fn test_timeout_on_empty_stream() {
        // Create a service that never yields responses
        async fn empty_service(
            _request: Request<Streaming<TestRequest>>,
        ) -> Result<Response<StreamResponseInner<TestResponse>>, Status> {
            // Create an empty stream that never yields anything
            let stream = async_stream::try_stream! {
                // Stream never yields any items, but we need to specify the Result type
                if false {
                    yield TestResponse::new(0, "This will never be returned");
                }
            };
            Ok(Response::new(Box::pin(stream)))
        }

        // Create a test context with the empty service
        let mut test = BidirectionalStreamingTest::<TestRequest, TestResponse>::new(empty_service);

        // Send a message and complete
        test.send_client_message(TestRequest::new("timeout-test", "data"))
            .await;
        test.complete().await;

        // Try with a short timeout
        let result = test
            .get_server_response_with_timeout(Duration::from_millis(50))
            .await;

        // It should be Ok(None) because the stream is empty but didn't error
        match result {
            Ok(None) => (), // Expected
            other => panic!("Expected Ok(None), got {:?}", other),
        }
    }
}
