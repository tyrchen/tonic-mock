# Implementation Plan: Phase 1

## Focus: Core Testing Infrastructure

This phase focuses on establishing a solid testing foundation for the existing functionality before extending the library.

### 1. Test Directory Structure Setup

Create the necessary test directory structure:

```
tests/
├── unit/
│   ├── mock_tests.rs  // Tests for MockBody and ProstDecoder
│   ├── request_tests.rs  // Tests for streaming_request
│   └── response_tests.rs  // Tests for response processing
├── integration/
│   └── sample_service.rs  // Simple gRPC service for integration testing
├── bench/
│   └── stream_bench.rs  // Benchmarks for streaming operations
└── common/
    ├── proto/  // Protocol definitions for test services
    ├── test_utils.rs  // Common testing utilities
    └── mod.rs  // Module definitions
```

### 2. Unit Test Implementation

#### 2.1 MockBody Tests (`tests/unit/mock_tests.rs`)

```rust
#[cfg(test)]
mod tests {
    use tonic_mock::{MockBody, ProstDecoder};
    use prost::Message;
    use bytes::Bytes;

    // Test message struct
    #[derive(Clone, PartialEq, Message)]
    pub struct TestMessage {
        #[prost(bytes = "bytes", tag = "1")]
        pub id: Bytes,
        #[prost(bytes = "bytes", tag = "2")]
        pub data: Bytes,
    }

    #[test]
    fn test_mock_body_creation() {
        // Test creating MockBody with different message counts
        // Verify length and emptiness properties
    }

    #[test]
    fn test_mock_body_encoding() {
        // Test that messages are correctly encoded
    }

    #[test]
    fn test_prost_decoder() {
        // Test that ProstDecoder correctly decodes messages
    }
}
```

#### 2.2 Request Tests (`tests/unit/request_tests.rs`)

```rust
#[cfg(test)]
mod tests {
    use tonic_mock::streaming_request;
    use tonic::{Request, Streaming};
    use futures::StreamExt;
    use tokio::runtime::Runtime;

    // Test message struct definition

    #[test]
    fn test_streaming_request_empty() {
        // Test with empty vector
    }

    #[test]
    fn test_streaming_request_single() {
        // Test with single message
    }

    #[test]
    fn test_streaming_request_multiple() {
        // Test with multiple messages
    }

    #[test]
    fn test_request_metadata() {
        // Test that metadata is correctly handled
    }
}
```

#### 2.3 Response Tests (`tests/unit/response_tests.rs`)

```rust
#[cfg(test)]
mod tests {
    use tonic_mock::{process_streaming_response, stream_to_vec, StreamResponseInner};
    use tonic::{Response, Status};
    use futures::Stream;
    use std::pin::Pin;
    use tokio::runtime::Runtime;

    // Test response message struct definition

    #[test]
    fn test_process_streaming_response() {
        // Test processing a streaming response with different message counts
    }

    #[test]
    fn test_stream_to_vec() {
        // Test converting a stream to a vector
    }

    #[test]
    fn test_error_handling() {
        // Test handling errors in the stream
    }
}
```

### 3. Integration Test Implementation

#### 3.1 Sample Service (`tests/integration/sample_service.rs`)

```rust
// Define a simple gRPC service using tonic
// Implement streaming methods
// Test the service with tonic-mock utilities
```

### 4. Benchmark Implementation

#### 4.1 Stream Benchmarks (`tests/bench/stream_bench.rs`)

```rust
// Benchmark different message sizes
// Benchmark different stream lengths
// Compare with manual implementation
```

### 5. CI/CD Pipeline Setup

Create GitHub Actions workflow for automated testing:

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true
      - uses: actions-rs/cargo@v1
        with:
          command: test
          args: --all-features

  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true
          components: clippy
      - uses: actions-rs/cargo@v1
        with:
          command: clippy
          args: -- -D warnings
```

### 6. Code Coverage Setup

Add code coverage reporting:

```yaml
# .github/workflows/coverage.yml
name: Coverage

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true
      - name: Install cargo-tarpaulin
        run: cargo install cargo-tarpaulin
      - name: Run cargo-tarpaulin
        run: cargo tarpaulin --out Xml
      - name: Upload to codecov.io
        uses: codecov/codecov-action@v1
```

### 7. Documentation Updates

Update documentation to reflect testing approach:

- Add testing documentation to README.md
- Document test utilities in code comments
- Add examples of writing tests with tonic-mock

### Timeline

1. Test directory structure setup: 1 day
2. Unit tests implementation: 3 days
3. Integration tests implementation: 2 days
4. Benchmarks implementation: 2 days
5. CI/CD pipeline setup: 1 day
6. Code coverage setup: 1 day
7. Documentation updates: 1 day

Total estimated time: 11 days

### Dependencies and Requirements

- Rust stable toolchain
- Tonic and Prost
- Tokio runtime for async testing
- Cargo-tarpaulin for code coverage
- GitHub Actions for CI/CD
