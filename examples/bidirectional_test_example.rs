// Example showing how to test gRPC bidirectional streaming

use futures::StreamExt;
use prost::Message;
use std::time::Duration;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status, Streaming};
use tonic_mock::{
    StreamResponseInner, streaming_request,
    test_utils::{TestRequest, TestResponse},
};

// Sample schema for our messages
#[derive(Clone, PartialEq, Message)]
pub struct ExampleRequest {
    #[prost(string, tag = "1")]
    pub user_id: String,
    #[prost(string, tag = "2")]
    pub message: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ExampleResponse {
    #[prost(int32, tag = "1")]
    pub status: i32,
    #[prost(string, tag = "2")]
    pub response: String,
}

// Our sample bidirectional streaming service
async fn chat_service(
    request: Request<Streaming<ExampleRequest>>,
) -> Result<Response<StreamResponseInner<ExampleResponse>>, Status> {
    let mut request_stream = request.into_inner();

    // Create response stream
    let response_stream = async_stream::try_stream! {
        while let Some(req) = request_stream.message().await? {
            // Simple echo response
            let response = ExampleResponse {
                status: 200,
                response: format!("Received: {} from user {}", req.message, req.user_id),
            };

            yield response;
        }
    };

    Ok(Response::new(Box::pin(response_stream)))
}

// A simple example of bidirectional streaming testing
async fn run_bidirectional_test_example() {
    println!("Testing bidirectional streaming with TestRequest/TestResponse...");

    // Create channel pairs for request and response
    let (client_tx, mut client_rx) = mpsc::channel::<TestRequest>(10);
    let (service_tx, mut service_rx) = mpsc::channel::<TestResponse>(10);

    // Task to collect client messages and process them
    let service_task = tokio::spawn(async move {
        println!("Service task started");

        // Collect and process messages
        let mut responses = Vec::new();

        while let Some(message) = client_rx.recv().await {
            let id_str = String::from_utf8_lossy(&message.id).to_string();
            let data_str = String::from_utf8_lossy(&message.data).to_string();

            println!("Service received: id={}, data={}", id_str, data_str);

            // Create a response
            let response =
                TestResponse::new(200, format!("Echo: id={}, data={}", id_str, data_str));

            // Add to responses and send it back
            responses.push(response.clone());

            // Send the response back to the client
            if let Err(e) = service_tx.send(response).await {
                println!("Error sending response: {}", e);
                break;
            }
        }

        println!("Service task completed with {} responses", responses.len());
        responses
    });

    // Wait a bit to ensure everything is set up
    println!("Test setup complete, waiting a bit...");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send client messages
    println!("Sending client message 1...");
    client_tx
        .send(TestRequest::new("msg1", "Hello server!"))
        .await
        .unwrap();

    // Give the service handler time to process
    println!("Waiting for response...");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get server response
    match service_rx.recv().await {
        Some(response) => {
            println!(
                "Received response 1: code={}, message={}",
                response.code, response.message
            );
        }
        None => {
            println!("No response received for message 1");
        }
    }

    // Send another message
    println!("Sending client message 2...");
    client_tx
        .send(TestRequest::new("msg2", "How are you?"))
        .await
        .unwrap();

    // Wait for response with timeout
    println!("Waiting for response with timeout...");
    match tokio::time::timeout(Duration::from_secs(1), service_rx.recv()).await {
        Ok(Some(response)) => {
            println!(
                "Received response 2: code={}, message={}",
                response.code, response.message
            );
        }
        Ok(None) => {
            println!("Channel closed without receiving response");
        }
        Err(_) => {
            println!("Timeout waiting for response");
        }
    }

    // Close the client channel to signal we're done
    println!("Completing the test...");
    drop(client_tx);

    // Wait for the service task to complete
    match service_task.await {
        Ok(responses) => {
            println!("Service task completed with {} responses", responses.len());
        }
        Err(e) => {
            println!("Service task failed: {}", e);
        }
    }

    println!("Test completed!");
}

async fn run_direct_api_example() {
    println!("\nTesting bidirectional streaming with custom ExampleRequest/ExampleResponse...");

    // Create a request using a channel
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ExampleRequest>(10);

    // Collect messages in a separate task
    let request_task = tokio::spawn(async move {
        let mut requests = Vec::new();

        // Receive all the messages
        while let Some(req) = rx.recv().await {
            requests.push(req);
        }

        // Return the collected requests
        requests
    });

    // Send messages to the service
    tx.send(ExampleRequest {
        user_id: "user123".to_string(),
        message: "Hello chat!".to_string(),
    })
    .await
    .unwrap();

    tx.send(ExampleRequest {
        user_id: "user123".to_string(),
        message: "How's the weather?".to_string(),
    })
    .await
    .unwrap();

    // Close the channel to signal end of messages
    drop(tx);

    // Wait for all messages to be collected
    let messages = request_task.await.unwrap();

    // Create a direct mock stream
    let request = streaming_request(messages);

    // Call the chat service
    match chat_service(request).await {
        Ok(response) => {
            let mut response_stream = response.into_inner();

            // Process responses
            let mut count = 0;
            println!("Processing responses:");
            while let Some(result) = response_stream.next().await {
                count += 1;
                match result {
                    Ok(resp) => {
                        println!(
                            "  Response {}: status={}, response={}",
                            count, resp.status, resp.response
                        );
                    }
                    Err(e) => {
                        println!("  Error: {}", e);
                    }
                }
            }
            println!("Received {} responses.", count);
        }
        Err(status) => {
            println!("Service call failed: {}", status);
        }
    }

    println!("Example completed!");
}

#[tokio::main]
async fn main() {
    // Run the bidirectional test example
    run_bidirectional_test_example().await;

    // Run the direct API example
    run_direct_api_example().await;
}
