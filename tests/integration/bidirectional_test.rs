use futures::StreamExt;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::runtime::Runtime;
use tonic::{Response, Status, Streaming};
use tonic_mock::{
    StreamResponseInner, streaming_request,
    test_utils::{TestRequest, TestResponse},
};

// Test basic bidirectional streaming functionality
#[test]
fn test_bidirectional_streaming_integration() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Create a service that counts requests and generates responses
        async fn counter_service(
            req: tonic::Request<Streaming<TestRequest>>,
        ) -> Result<Response<StreamResponseInner<TestResponse>>, Status> {
            let mut in_stream = req.into_inner();
            let counter = Arc::new(AtomicUsize::new(0));

            // Echo stream that counts requests
            let stream = async_stream::try_stream! {
                while let Some(msg) = in_stream.message().await? {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    let id_str = String::from_utf8_lossy(&msg.id).to_string();
                    let data_str = String::from_utf8_lossy(&msg.data).to_string();

                    yield TestResponse::new(
                        count as i32 + 100,  // use count as part of response code
                        format!("Request #{}: id={}, data={}", count, id_str, data_str)
                    );
                }
            };

            Ok(Response::new(Box::pin(stream)))
        }

        // Create test messages
        let test_messages = vec![
            TestRequest::new("id1", "hello"),
            TestRequest::new("id2", "world"),
            TestRequest::new("id3", "bidirectional"),
        ];

        // Create streaming request
        let request = streaming_request(test_messages);

        // Call the service
        let response = counter_service(request).await.unwrap();
        let response_stream = response.into_inner();

        // Collect all responses
        let responses: Vec<_> = response_stream
            .collect::<Vec<Result<TestResponse, Status>>>()
            .await;

        // Verify responses
        assert_eq!(responses.len(), 3);

        // Check response details
        let first = &responses[0].as_ref().unwrap();
        assert_eq!(first.code, 100); // first request
        assert!(first.message.contains("id=id1"));
        assert!(first.message.contains("data=hello"));

        let second = &responses[1].as_ref().unwrap();
        assert_eq!(second.code, 101); // second request
        assert!(second.message.contains("id=id2"));
        assert!(second.message.contains("data=world"));

        let third = &responses[2].as_ref().unwrap();
        assert_eq!(third.code, 102); // third request
        assert!(third.message.contains("id=id3"));
        assert!(third.message.contains("data=bidirectional"));
    });
}

// Test bidirectional streaming with timeout handling
#[test]
fn test_bidirectional_streaming_timeout() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Create a service with intentional delays
        async fn delayed_service(
            req: tonic::Request<Streaming<TestRequest>>,
        ) -> Result<Response<StreamResponseInner<TestResponse>>, Status> {
            let mut in_stream = req.into_inner();

            let stream = async_stream::try_stream! {
                let mut count = 0;
                while let Some(msg) = in_stream.message().await? {
                    count += 1;

                    // Add increasing delay for each message
                    let delay = count * 50; // 50ms, 100ms, 150ms, etc.
                    tokio::time::sleep(Duration::from_millis(delay as u64)).await;

                    let id_str = String::from_utf8_lossy(&msg.id).to_string();

                    yield TestResponse::new(
                        delay,
                        format!("Delayed response for id={} (delay={}ms)", id_str, delay)
                    );
                }
            };

            Ok(Response::new(Box::pin(stream)))
        }

        // Create test messages
        let test_messages = vec![
            TestRequest::new("fast", "should be fast"),
            TestRequest::new("medium", "might timeout"),
            TestRequest::new("slow", "should timeout"),
        ];

        // Create streaming request
        let request = streaming_request(test_messages);

        // Call the service
        let response = delayed_service(request).await.unwrap();
        let mut response_stream = response.into_inner();

        // First response should come quickly
        let first_result =
            tokio::time::timeout(Duration::from_millis(60), response_stream.next()).await;

        assert!(first_result.is_ok());
        let first = first_result.unwrap().unwrap().unwrap();
        assert_eq!(first.code, 50);
        assert!(first.message.contains("id=fast"));

        // Second response should come within a reasonable timeout
        let second_result =
            tokio::time::timeout(Duration::from_millis(110), response_stream.next()).await;

        assert!(second_result.is_ok());
        let second = second_result.unwrap().unwrap().unwrap();
        assert_eq!(second.code, 100);
        assert!(second.message.contains("id=medium"));

        // Third response should timeout with a short timeout
        let third_result =
            tokio::time::timeout(Duration::from_millis(100), response_stream.next()).await;

        assert!(third_result.is_err()); // Should timeout

        // But should succeed with a longer timeout
        let third_retry =
            tokio::time::timeout(Duration::from_millis(200), response_stream.next()).await;

        assert!(third_retry.is_ok());
        let third = third_retry.unwrap().unwrap().unwrap();
        assert_eq!(third.code, 150);
        assert!(third.message.contains("id=slow"));

        // No more responses
        assert!(response_stream.next().await.is_none());
    });
}
