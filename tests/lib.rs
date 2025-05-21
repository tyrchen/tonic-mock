// Main test module for tonic-mock

// Common test utilities and data structures
pub mod common;

// Unit tests
#[cfg(test)]
mod unit {
    pub mod bidirectional_tests;
    pub mod client_mock_tests;
    pub mod grpc_mock_tests;
    pub mod mock_tests;
    pub mod request_tests;
    pub mod response_tests;
    pub mod test_utils_tests;
}

// Integration tests
#[cfg(test)]
mod integration {
    pub mod bidirectional_test;
    pub mod sample_service;
}

// Benchmarks are in the bench directory and run separately
