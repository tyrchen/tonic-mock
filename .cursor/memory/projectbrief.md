# Project Brief: tonic-mock

## Overview
`tonic-mock` is a Rust library providing test utilities for easily mocking the tonic gRPC streaming interface. It simplifies the process of testing gRPC services by providing tools to mock streaming requests and process streaming responses.

## Core Purpose
The primary purpose of this library is to make it easier to test gRPC services built with the tonic framework, particularly for streaming interfaces, which are typically difficult to test.

## Key Features
1. `streaming_request`: Generate streaming requests for gRPC testing
2. `process_streaming_response`: Process and validate streaming responses
3. `stream_to_vec`: Convert a streaming response to a vector for simplified testing
4. Mock implementation for tonic's streaming interface

## Technical Context
- Built for Rust's tonic gRPC framework
- Version: 0.3.0
- License: MIT
- Dependencies:
  - bytes 1.x
  - futures 0.3.x
  - http-body 1.x
  - http 1.x
  - prost 0.13.x
  - tonic 0.13.x

## Codebase Structure
- `src/lib.rs`: Main entry point with the public API
- `src/mock.rs`: Implementation of the mock functionality

## Maintainer
Tyr Chen <tyr.chen@gmail.com>
