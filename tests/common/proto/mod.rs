// Proto module for tests
// This module simulates the generated code from protobuf files

use bytes::Bytes;
use futures::Stream;
use prost::Message;
use std::pin::Pin;
use tonic::{Request, Response, Status, Streaming};

// Re-export the test proto file's contents
#[allow(dead_code)]
pub const TEST_PROTO_CONTENT: &str = include_str!("test.proto");

// Test request message (generated from proto)
#[derive(Clone, PartialEq, Message)]
pub struct TestRequest {
    #[prost(bytes = "bytes", tag = "1")]
    pub id: Bytes,
    #[prost(bytes = "bytes", tag = "2")]
    pub data: Bytes,
}

#[allow(dead_code)]
impl TestRequest {
    pub fn new(id: impl Into<Bytes>, data: impl Into<Bytes>) -> Self {
        Self {
            id: id.into(),
            data: data.into(),
        }
    }
}

// Test response message (generated from proto)
#[derive(Clone, PartialEq, Message)]
pub struct TestResponse {
    #[prost(int32, tag = "1")]
    pub code: i32,
    #[prost(string, tag = "2")]
    pub message: String,
}

#[allow(dead_code)]
impl TestResponse {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

// Stream response type
#[allow(dead_code)]
pub type ResponseStream =
    Pin<Box<dyn Stream<Item = Result<TestResponse, Status>> + Send + 'static>>;

// Test service trait (generated from proto)
#[allow(dead_code)]
#[tonic::async_trait]
pub trait TestService {
    async fn unary(&self, request: Request<TestRequest>) -> Result<Response<TestResponse>, Status>;

    async fn server_streaming(
        &self,
        request: Request<TestRequest>,
    ) -> Result<Response<ResponseStream>, Status>;

    async fn client_streaming(
        &self,
        request: Request<Streaming<TestRequest>>,
    ) -> Result<Response<TestResponse>, Status>;

    async fn bidirectional_streaming(
        &self,
        request: Request<Streaming<TestRequest>>,
    ) -> Result<Response<ResponseStream>, Status>;
}
