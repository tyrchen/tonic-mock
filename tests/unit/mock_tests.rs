#[cfg(test)]
mod tests {
    use crate::common::TestMessage;
    use bytes::Bytes;
    use http_body::Body;
    use prost::Message;
    use std::pin::Pin;
    use tonic_mock::MockBody;

    #[test]
    fn test_mock_body_creation() {
        // Test with empty messages
        let empty_messages: Vec<TestMessage> = Vec::new();
        let body = MockBody::<TestMessage>::new(empty_messages);
        assert_eq!(body.len(), 0);
        assert!(body.is_empty());

        // Test with single message
        let single_message = vec![TestMessage::new("1", "test_data")];
        let body = MockBody::<TestMessage>::new(single_message);
        assert_eq!(body.len(), 1);
        assert!(!body.is_empty());

        // Test with multiple messages
        let multiple_messages = vec![
            TestMessage::new("1", "data1"),
            TestMessage::new("2", "data2"),
            TestMessage::new("3", "data3"),
        ];
        let body = MockBody::<TestMessage>::new(multiple_messages);
        assert_eq!(body.len(), 3);
        assert!(!body.is_empty());
    }

    #[test]
    fn test_mock_body_encoding() {
        // Create a test message and encode it
        let message = TestMessage::new("test_id", "test_data");

        // Test that the message can be encoded into bytes
        let encoded = message.encode_to_vec();
        assert!(!encoded.is_empty());

        // Verify we can decode the message again
        let decoded = TestMessage::decode(&encoded[..]).unwrap();
        assert_eq!(decoded.id, Bytes::from("test_id"));
        assert_eq!(decoded.data, Bytes::from("test_data"));
    }

    #[test]
    fn test_mock_body_polling() {
        // Create messages and a MockBody
        let messages = vec![
            TestMessage::new("1", "data1"),
            TestMessage::new("2", "data2"),
        ];
        let mut body = MockBody::<TestMessage>::new(messages);

        // Test that the body behaves as expected for http_body::Body
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

        // Create a dummy waker
        fn noop_raw_waker() -> RawWaker {
            fn no_op(_: *const ()) {}
            fn clone(_: *const ()) -> RawWaker {
                noop_raw_waker()
            }

            let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
            RawWaker::new(std::ptr::null(), vtable)
        }

        let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
        let mut cx = Context::from_waker(&waker);

        // Testing poll_frame behavior - first poll should return the first item
        let poll_result = Pin::new(&mut body).poll_frame(&mut cx);
        assert!(matches!(poll_result, Poll::Ready(Some(Ok(_)))));

        // Second poll should return the second item
        let poll_result = Pin::new(&mut body).poll_frame(&mut cx);
        assert!(matches!(poll_result, Poll::Ready(Some(Ok(_)))));

        // Third poll should return None (end of stream)
        let poll_result = Pin::new(&mut body).poll_frame(&mut cx);
        assert!(matches!(poll_result, Poll::Ready(None)));
    }
}
