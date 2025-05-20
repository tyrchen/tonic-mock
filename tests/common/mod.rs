// Common test utilities and modules for tonic-mock tests

pub mod proto;
pub mod test_utils;

use bytes::Bytes;
use prost::Message;

// Test message struct used across tests
#[derive(Clone, PartialEq, Message)]
pub struct TestMessage {
    #[prost(bytes = "bytes", tag = "1")]
    pub id: Bytes,
    #[prost(bytes = "bytes", tag = "2")]
    pub data: Bytes,
}

impl TestMessage {
    pub fn new(id: impl Into<Bytes>, data: impl Into<Bytes>) -> Self {
        Self {
            id: id.into(),
            data: data.into(),
        }
    }
}

// Test response message used across tests
#[derive(Clone, PartialEq, Message)]
pub struct TestResponse {
    #[prost(int32, tag = "1")]
    pub code: i32,
    #[prost(string, tag = "2")]
    pub message: String,
}

impl TestResponse {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
