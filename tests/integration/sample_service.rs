use crate::common::{TestMessage, TestResponse, test_utils};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tonic::{Request, Response, Status, Streaming};
use tonic_mock::streaming_request;

// Custom stream type that doesn't require Sync
type CustomResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + 'static>>;

// Define a sample gRPC service for testing
pub struct SampleService;

// Define the service trait
#[allow(dead_code)]
#[tonic::async_trait]
pub trait TestService {
    async fn unary(&self, request: Request<TestMessage>) -> Result<Response<TestResponse>, Status>;

    async fn server_streaming(
        &self,
        request: Request<TestMessage>,
    ) -> Result<Response<CustomResponseStream<TestResponse>>, Status>;

    async fn client_streaming(
        &self,
        request: Request<Streaming<TestMessage>>,
    ) -> Result<Response<TestResponse>, Status>;

    async fn bidirectional_streaming(
        &self,
        request: Request<Streaming<TestMessage>>,
    ) -> Result<Response<CustomResponseStream<TestResponse>>, Status>;
}

// Implement the service
#[tonic::async_trait]
impl TestService for SampleService {
    async fn unary(&self, request: Request<TestMessage>) -> Result<Response<TestResponse>, Status> {
        let message = request.into_inner();
        let id_str = String::from_utf8_lossy(&message.id).to_string();

        // Simple echo with a code
        let response = TestResponse::new(1, format!("Echo: {}", id_str));

        Ok(Response::new(response))
    }

    async fn server_streaming(
        &self,
        request: Request<TestMessage>,
    ) -> Result<Response<CustomResponseStream<TestResponse>>, Status> {
        let message = request.into_inner();
        let id_str = String::from_utf8_lossy(&message.id).to_string();

        // Generate multiple responses based on the request
        let count = id_str.parse::<i32>().unwrap_or(3);

        let stream = async_stream::try_stream! {
            for i in 0..count {
                yield TestResponse::new(
                    i,
                    format!("Response {} for request {}", i, id_str)
                );
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }

    async fn client_streaming(
        &self,
        request: Request<Streaming<TestMessage>>,
    ) -> Result<Response<TestResponse>, Status> {
        let mut stream = request.into_inner();
        let mut count = 0;

        // Process all messages and return a summary
        while stream.message().await?.is_some() {
            count += 1;
            // Process each message...
        }

        let response = TestResponse::new(count, format!("Processed {} messages", count));

        Ok(Response::new(response))
    }

    async fn bidirectional_streaming(
        &self,
        request: Request<Streaming<TestMessage>>,
    ) -> Result<Response<CustomResponseStream<TestResponse>>, Status> {
        let mut in_stream = request.into_inner();

        // Clone this for later use
        let mut messages = Vec::new();

        // First, collect all messages to avoid holding references to in_stream
        while let Some(message) = in_stream.message().await? {
            messages.push((messages.len(), message.id.clone()));
        }

        // Then create a response stream that doesn't reference in_stream
        let stream = async_stream::try_stream! {
            for (count, id_bytes) in messages {
                yield TestResponse::new(
                    count as i32,
                    format!("Echo for message {}: {:?}", count, id_bytes)
                );
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }
}

// Integration tests
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_client_streaming() {
        let rt = Runtime::new().unwrap();

        // Create the service
        let service = SampleService;

        // Create test messages
        let messages = test_utils::create_test_messages(5);

        // Create streaming request
        let request = streaming_request(messages);

        // Call the service
        let response = rt.block_on(async { service.client_streaming(request).await.unwrap() });

        // Verify response
        let response = response.into_inner();
        assert_eq!(response.code, 5); // 5 messages processed
        assert_eq!(response.message, "Processed 5 messages");
    }

    #[test]
    fn test_bidirectional_streaming() {
        let rt = Runtime::new().unwrap();

        // Create the service
        let service = SampleService;

        // Create test messages
        let messages = test_utils::create_test_messages(3);

        // Create streaming request
        let request = streaming_request(messages);

        // Call the service
        let response =
            rt.block_on(async { service.bidirectional_streaming(request).await.unwrap() });

        // Extract the inner stream
        let mut stream = response.into_inner();

        // Process responses manually
        let mut responses = Vec::new();
        rt.block_on(async {
            while let Some(result) = stream.as_mut().next().await {
                responses.push(result);
            }
        });

        // Verify responses
        assert_eq!(responses.len(), 3);

        #[allow(clippy::needless_range_loop)]
        for i in 0..3 {
            assert!(responses[i].is_ok());
            let response = responses[i].as_ref().unwrap();
            assert_eq!(response.code, i as i32);
            assert!(
                response
                    .message
                    .contains(&format!("Echo for message {}", i))
            );
            assert!(
                response
                    .message
                    .contains(&format!("{:?}", Bytes::from(i.to_string())))
            );
        }
    }
}
