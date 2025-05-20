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
        StreamResponseInner, streaming_request,
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
                )
                    as StreamResponseInner<TestResponse>))
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
}
