# System Patterns

## Code Organization
- Public API exposed in `lib.rs`
- Implementation details in separate modules (e.g., `mock.rs`)
- Usage examples embedded in doc comments

## Type Patterns
- Use of generics for flexible API design
- Type aliases for complex types (`StreamResponseInner<T>`, `StreamResponse<T>`)
- Trait bounds to ensure message types implement required traits

## Rust Idioms
- Use of Rust's type system to enforce correctness
- Async/await for handling streaming operations
- Trait implementations for standard interfaces (e.g., `Body` for `MockBody`)
- Phantom data for zero-cost type information (`ProstDecoder<U>`)

## Error Handling
- Use of `Result<T, Status>` for error propagation
- Conversion of domain errors to tonic `Status` errors

## Testing Patterns
- Example-driven development with doc tests
- Simplified test utilities to reduce boilerplate
- Helper functions that encapsulate common testing patterns

## API Design Principles
- Simple, focused functions with clear purposes
- Generic interfaces that work with any prost Message type
- Builder-style pattern for constructing complex objects
- Closure-based callbacks for processing streaming data
