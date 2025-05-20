// Common test utilities for tonic-mock tests

use crate::common::{TestMessage, TestResponse};
use bytes::Bytes;
use tonic::{Response, Status};
use tonic_mock::StreamResponseInner;

// Create a vector of test messages with specified count
pub fn create_test_messages(count: usize) -> Vec<TestMessage> {
    let mut messages = Vec::with_capacity(count);
    for i in 0..count {
        messages.push(TestMessage::new(i.to_string(), format!("test_data_{}", i)));
    }
    messages
}

// Create a stream response from a vector of responses
pub fn create_stream_response(
    responses: Vec<TestResponse>,
) -> Response<StreamResponseInner<TestResponse>> {
    let stream = async_stream::try_stream! {
        for response in responses {
            yield response;
        }
    };

    Response::new(Box::pin(stream))
}

// Create a stream response with errors at specified indices
#[allow(dead_code)]
pub fn create_stream_response_with_errors(
    responses: Vec<TestResponse>,
    error_indices: Vec<usize>,
    error_status: Status,
) -> Response<StreamResponseInner<TestResponse>> {
    let stream = async_stream::try_stream! {
        for (i, response) in responses.into_iter().enumerate() {
            if error_indices.contains(&i) {
                yield Err(error_status.clone())?;
            } else {
                yield response;
            }
        }
    };

    Response::new(Box::pin(stream))
}

// Helper to assert test message equality
#[allow(dead_code)]
pub fn assert_message_eq(message: &TestMessage, id: impl AsRef<str>, data: impl AsRef<str>) {
    let id_bytes = Bytes::from(id.as_ref().to_string());
    let data_bytes = Bytes::from(data.as_ref().to_string());
    assert_eq!(message.id, id_bytes);
    assert_eq!(message.data, data_bytes);
}

// Helper to assert test response equality
#[allow(dead_code)]
pub fn assert_response_eq(response: &TestResponse, code: i32, message: impl AsRef<str>) {
    assert_eq!(response.code, code);
    assert_eq!(response.message, message.as_ref());
}
