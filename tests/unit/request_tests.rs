#[cfg(test)]
mod tests {
    use crate::common::{TestMessage, test_utils};
    use tokio::runtime::Runtime;
    use tonic::{Request, Status, Streaming};
    use tonic_mock::streaming_request;

    // Helper function to extract messages from a streaming request
    async fn extract_messages(
        request: Request<Streaming<TestMessage>>,
    ) -> Result<Vec<TestMessage>, Status> {
        let mut stream = request.into_inner();
        let mut messages = Vec::new();

        while let Some(message) = stream.message().await? {
            messages.push(message);
        }

        Ok(messages)
    }

    #[test]
    fn test_streaming_request_empty() {
        let rt = Runtime::new().unwrap();

        // Create an empty streaming request
        let empty_messages: Vec<TestMessage> = Vec::new();
        let request = streaming_request(empty_messages);

        // Extract and verify no messages are present
        let messages = rt.block_on(extract_messages(request)).unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_streaming_request_single() {
        let rt = Runtime::new().unwrap();

        // Create a single message streaming request
        let message = TestMessage::new("single_id", "single_data");
        let messages = vec![message];
        let request = streaming_request(messages);

        // Extract and verify the message
        let extracted = rt.block_on(extract_messages(request)).unwrap();
        assert_eq!(extracted.len(), 1);
        test_utils::assert_message_eq(&extracted[0], "single_id", "single_data");
    }

    #[test]
    fn test_streaming_request_multiple() {
        let rt = Runtime::new().unwrap();

        // Create a streaming request with multiple messages
        let messages = test_utils::create_test_messages(5);
        let request = streaming_request(messages);

        // Extract and verify the messages
        let extracted = rt.block_on(extract_messages(request)).unwrap();
        assert_eq!(extracted.len(), 5);

        #[allow(clippy::needless_range_loop)]
        for i in 0..5 {
            test_utils::assert_message_eq(&extracted[i], i.to_string(), format!("test_data_{}", i));
        }
    }

    #[test]
    fn test_request_metadata() {
        // Create a streaming request
        let messages = test_utils::create_test_messages(1);
        let mut request = streaming_request(messages);

        // Add metadata to the request
        let metadata = request.metadata_mut();
        metadata.insert("key1", "value1".parse().unwrap());
        metadata.insert("key2", "value2".parse().unwrap());

        // Verify metadata is accessible
        let metadata = request.metadata();
        assert_eq!(metadata.get("key1").unwrap(), "value1");
        assert_eq!(metadata.get("key2").unwrap(), "value2");
    }
}
