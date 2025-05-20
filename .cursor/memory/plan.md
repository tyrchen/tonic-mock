# Enhancement Plan for tonic-mock

## Requirements Analysis

### Current Limitations
1. Limited to basic streaming request/response testing
2. No dedicated test suite beyond doc tests
3. No support for error injection or timeout configuration
4. Limited examples for complex testing scenarios
5. No support for interceptors or metadata testing
6. Lack of performance benchmarks

### Enhancement Goals
1. Expand core functionality while maintaining API simplicity
2. Develop comprehensive test coverage
3. Improve documentation with more examples and tutorials
4. Add support for advanced testing scenarios
5. Ensure compatibility with latest tonic and prost versions

## Components Affected

### Core Library
- `src/lib.rs`: Add new public API functions
- `src/mock.rs`: Expand mock implementation
- New modules:
  - `src/error.rs`: Error injection utilities
  - `src/interceptor.rs`: Interceptor support
  - `src/timeout.rs`: Timeout configuration
  - `src/bidirectional.rs`: Bidirectional streaming support

### Testing Components
- New directory: `tests/`
  - `tests/unit/`: Unit tests for each component
  - `tests/integration/`: Tests with sample gRPC service
  - `tests/bench/`: Performance benchmarks

### Documentation
- `README.md`: Enhanced examples and usage guide
- `examples/`: Directory with example projects
- Doc comments: Expanded with more context

## Implementation Strategy

### Phase 1: Core Testing Infrastructure
1. Create comprehensive test suite for existing functionality
2. Set up basic CI/CD pipeline for test automation
3. Document test patterns and approach

### Phase 2: API Extensions
1. Implement timeout configuration
2. Add error injection capabilities
3. Develop interceptor support
4. Create bidirectional streaming utilities

### Phase 3: Documentation and Examples
1. Expand README with detailed examples
2. Create example projects demonstrating usage patterns
3. Improve inline documentation

### Phase 4: Performance and Compatibility
1. Add benchmarks for performance testing
2. Ensure compatibility with latest tonic/prost
3. Test with different Rust versions

## Detailed Implementation Steps

### Phase 1: Core Testing Infrastructure

#### Step 1.1: Set up test directory structure
```rust
mkdir -p tests/unit
mkdir -p tests/integration
mkdir -p tests/bench
```

#### Step 1.2: Create unit tests for existing functionality
Create comprehensive unit tests for:
- `streaming_request`
- `process_streaming_response`
- `stream_to_vec`
- `MockBody`
- `ProstDecoder`

#### Step 1.3: Set up CI/CD pipeline
- Add GitHub Actions workflow for test automation
- Configure code coverage reporting

### Phase 2: API Extensions

#### Step 2.1: Timeout Configuration
Add timeout support to streaming operations:
```rust
pub fn streaming_request_with_timeout<T>(
    messages: Vec<T>,
    timeout: std::time::Duration,
) -> Request<Streaming<T>>
where
    T: Message + Default + 'static,
{
    // Implementation
}
```

#### Step 2.2: Error Injection
Create utilities for injecting errors in streams:
```rust
pub fn streaming_request_with_errors<T>(
    messages: Vec<T>,
    error_pattern: ErrorPattern,
) -> Request<Streaming<T>>
where
    T: Message + Default + 'static,
{
    // Implementation
}

pub enum ErrorPattern {
    AfterMessages(usize), // Inject error after N messages
    Random(f32),          // Random error with probability
    Alternating,          // Alternate between message and error
    Custom(Box<dyn Fn(usize) -> bool>), // Custom error pattern function
}
```

#### Step 2.3: Interceptor Support
Add support for testing interceptors:
```rust
pub fn streaming_request_with_interceptor<T, I>(
    messages: Vec<T>,
    interceptor: I,
) -> Request<Streaming<T>>
where
    T: Message + Default + 'static,
    I: tonic::service::Interceptor + 'static,
{
    // Implementation
}
```

#### Step 2.4: Bidirectional Streaming
Create utilities for testing bidirectional streaming:
```rust
pub fn bidirectional_streaming_test<Req, Resp, F>(
    requests: Vec<Req>,
    response_handler: F,
) -> (Request<Streaming<Req>>, StreamResponse<Resp>)
where
    Req: Message + Default + 'static,
    Resp: Message + Default + 'static,
    F: Fn(Req) -> Result<Resp, Status> + 'static,
{
    // Implementation
}
```

### Phase 3: Documentation and Examples

#### Step 3.1: Expand README
- Add detailed examples for all new functionality
- Provide usage patterns and best practices
- Include troubleshooting section

#### Step 3.2: Create example projects
- Simple gRPC service testing example
- Complex bidirectional streaming example
- Error handling and timeout testing example

#### Step 3.3: Improve inline documentation
- Add more context to existing functions
- Document new functionality thoroughly
- Include examples for advanced use cases

### Phase 4: Performance and Compatibility

#### Step 4.1: Add benchmarks
- Benchmark large message volume handling
- Measure performance of different stream processing approaches
- Compare with manual implementation approaches

#### Step 4.2: Ensure compatibility
- Test with latest tonic/prost versions
- Verify compatibility with different Rust versions
- Document compatibility requirements

## Challenges & Mitigations

### Challenge 1: Maintaining API Simplicity
**Mitigation**: Keep core API unchanged and add new functions rather than modifying existing ones. Use builder pattern for complex configurations.

### Challenge 2: Backward Compatibility
**Mitigation**: Ensure all new functionality is opt-in and doesn't change existing behavior. Write compatibility tests.

### Challenge 3: Performance Impact
**Mitigation**: Benchmark new features to ensure they don't negatively impact performance. Provide optimized implementations where possible.

### Challenge 4: Documentation Complexity
**Mitigation**: Organize documentation hierarchically from simple to advanced use cases. Use examples extensively.

## Success Criteria

1. All existing and new functionality covered by comprehensive tests
2. Test coverage > 90%
3. Documentation covers all use cases with examples
4. Performance benchmarks show acceptable overhead
5. Compatibility with latest tonic and prost versions
6. CI/CD pipeline validates all changes automatically
